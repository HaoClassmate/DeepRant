#![cfg_attr(not(target_os = "windows"), allow(dead_code, unused_imports))]
// 截图翻译：截取记住的聊天区域 → Windows 自带 OCR → 翻成中文 → 游戏上方的悬浮窗显示，
// 同时可以推送到远程网页（dota-translator 的 server/）。
//
// 全程不碰游戏进程：截图走 GDI，悬浮窗是一个置顶、透明、鼠标穿透的普通窗口。
// 只支持 Windows；其他系统上快捷键不注册，命令返回错误。

use crate::store::{get_settings, update_settings_field, OcrRegion};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::AppHandle;

pub const SELECTOR: &str = "ocr-selector";
pub const OVERLAY: &str = "ocr-overlay";

#[derive(Clone, Serialize)]
pub struct OcrItem {
    // 行首的 "名字:" 部分，没有就为空
    pub name: String,
    // 名字之后的原文
    pub text: String,
    // 译文；不需要翻译（中文、纯符号）时为空
    pub zh: String,
}

#[derive(Clone, Serialize)]
pub struct OverlayPayload {
    pub id: u64,
    // pending | done | error
    pub state: String,
    pub items: Vec<OcrItem>,
    pub message: String,
    pub seconds: u32,
    // 推送到网页的结果，没开推送时为空
    pub note: String,
    pub note_error: bool,
}

static BUSY: AtomicBool = AtomicBool::new(false);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
// 悬浮窗第一次创建时网页还没加载好，收不到事件：加载完后用 ocr_last_overlay 取最近一次
static LAST: Mutex<Option<OverlayPayload>> = Mutex::new(None);

pub fn last_overlay() -> Option<OverlayPayload> {
    LAST.lock().ok().and_then(|l| l.clone())
}

/// "Name: message" 拆成名字和消息；名字太长或没有冒号就整行当消息
fn split_name(line: &str) -> (String, String) {
    for sep in [": ", "：", ":"] {
        if let Some(i) = line.find(sep) {
            let name = line[..i].trim();
            if !name.is_empty() && name.chars().count() <= 40 {
                return (name.to_string(), line[i + sep.len()..].trim().to_string());
            }
        }
    }
    (String::new(), line.trim().to_string())
}

/// 已经是中文，或者没有字母可翻译
fn needs_translation(text: &str) -> bool {
    let han = text.chars().filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c)).count();
    let letters = text.chars().filter(|c| c.is_alphabetic() && !('\u{4E00}'..='\u{9FFF}').contains(c)).count();
    letters > 0 && han < letters
}

/// 推送到网页（dota-translator 的 /ocr），返回服务器新加了几条；出错时给出能看懂的原因
async fn push(url: &str, token: &str, items: &[OcrItem]) -> Result<u64, String> {
    if url.trim().is_empty() {
        return Err("没有填推送地址".into());
    }
    if token.trim().is_empty() {
        return Err("没有填口令".into());
    }
    let res = reqwest::Client::new()
        .post(url.trim())
        .timeout(std::time::Duration::from_secs(10))
        .json(&serde_json::json!({ "token": token.trim(), "items": items }))
        .send()
        .await
        .map_err(|e| format!("连不上服务器：{}", e))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    match status.as_u16() {
        // 地址写错时服务器也可能回 200（当成了游戏数据），所以要看回的是不是 /ocr 的格式
        200 => serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["added"].as_u64())
            .ok_or_else(|| "地址不对：服务器没有按截图推送接口回复，应为 https://dota.wanghaos.com/ocr".to_string()),
        403 => Err("口令不对（服务器返回 403）".into()),
        404 => Err("地址不对（服务器返回 404），应为 https://dota.wanghaos.com/ocr".into()),
        _ => Err(format!("服务器返回 HTTP {}：{}", status, body.chars().take(100).collect::<String>())),
    }
}

#[cfg(target_os = "windows")]
mod win {
    use super::*;
    use crate::ai_translator;
    use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

    fn emit(app: &AppHandle, payload: OverlayPayload) {
        if let Ok(mut last) = LAST.lock() {
            *last = Some(payload.clone());
        }
        if let Err(e) = app.emit_to(OVERLAY, "ocr-overlay", payload) {
            println!("发送悬浮窗事件失败: {}", e);
        }
    }

    /// 悬浮窗：放在聊天区域正上方（上面放不下就放下面），不抢焦点、鼠标穿透
    fn place_overlay(app: &AppHandle, region: &OcrRegion) -> tauri::Result<()> {
        let width = region.width.max(480);
        let height = 320u32;
        let above = region.y - height as i32 - 8;
        let y = if above >= 0 { above } else { region.y + region.height as i32 + 8 };
        let window = match app.get_webview_window(OVERLAY) {
            Some(w) => w,
            None => {
                let w = WebviewWindowBuilder::new(app, OVERLAY, WebviewUrl::App("index.html".into()))
                    .initialization_script("window.__DEEPRANT_VIEW = 'overlay';")
                    .title("DeepRant 截图翻译")
                    .decorations(false)
                    .transparent(true)
                    .shadow(false)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .resizable(false)
                    .focused(false)
                    .visible(false)
                    .build()?;
                w.set_ignore_cursor_events(true)?;
                w
            }
        };
        window.set_size(PhysicalSize::new(width, height))?;
        window.set_position(PhysicalPosition::new(region.x, y))?;
        // 一直显示着（内容为空时完全透明），之后不再 show/hide，避免抢走游戏的焦点
        if !window.is_visible().unwrap_or(false) {
            window.show()?;
        }
        Ok(())
    }

    /// 框选窗口：盖住主显示器，拖一个框
    pub fn open_selector(app: &AppHandle) -> Result<(), String> {
        if let Some(w) = app.get_webview_window(SELECTOR) {
            let _ = w.set_focus();
            return Ok(());
        }
        let monitor = app
            .primary_monitor()
            .map_err(|e| e.to_string())?
            .ok_or("找不到显示器")?;
        let window = WebviewWindowBuilder::new(app, SELECTOR, WebviewUrl::App("index.html".into()))
            .initialization_script("window.__DEEPRANT_VIEW = 'selector';")
            .title("框选聊天区域")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?;
        window.set_position(*monitor.position()).map_err(|e| e.to_string())?;
        window.set_size(*monitor.size()).map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        let _ = window.set_focus();
        Ok(())
    }

    /// GDI 截取屏幕上一块区域，返回 BGRA 像素（自上而下）
    fn capture(region: &OcrRegion) -> anyhow::Result<Vec<u8>> {
        use windows::Win32::Graphics::Gdi::*;
        let (w, h) = (region.width as i32, region.height as i32);
        unsafe {
            let screen = GetDC(None);
            let mem = CreateCompatibleDC(screen);
            let bitmap = CreateCompatibleBitmap(screen, w, h);
            let old = SelectObject(mem, bitmap);
            let blit = BitBlt(mem, 0, 0, w, h, screen, region.x, region.y, SRCCOPY | CAPTUREBLT);
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    // 负数：自上而下
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            let rows = GetDIBits(mem, bitmap, 0, h as u32, Some(pixels.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
            SelectObject(mem, old);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
            blit?;
            if rows == 0 {
                anyhow::bail!("读取截图像素失败");
            }
            Ok(pixels)
        }
    }

    /// 等比例放大（最近邻）：聊天字很小，放大后 Windows OCR 认得准很多
    fn upscale(pixels: &[u8], w: u32, h: u32, k: u32) -> Vec<u8> {
        let (nw, nh) = (w * k, h * k);
        let mut out = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            let src_row = ((y / k) * w * 4) as usize;
            let dst_row = (y * nw * 4) as usize;
            for x in 0..nw {
                let s = src_row + ((x / k) * 4) as usize;
                let d = dst_row + (x * 4) as usize;
                out[d..d + 4].copy_from_slice(&pixels[s..s + 4]);
            }
        }
        out
    }

    /// Windows.Media.Ocr 识别，返回每一行文字
    fn ocr(pixels: Vec<u8>, w: u32, h: u32) -> anyhow::Result<Vec<String>> {
        use windows::core::HSTRING;
        use windows::Globalization::Language;
        use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
        use windows::Media::Ocr::OcrEngine;
        use windows::Storage::Streams::DataWriter;
        use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

        unsafe {
            // 已经初始化过会返回错误，忽略
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
        }
        let max = OcrEngine::MaxImageDimension()?;
        let k = if w.max(h) * 2 <= max { 2 } else { 1 };
        let (pixels, w, h) = if k > 1 { (upscale(&pixels, w, h, k), w * k, h * k) } else { (pixels, w, h) };

        let bitmap = SoftwareBitmap::Create(BitmapPixelFormat::Bgra8, w as i32, h as i32)?;
        let writer = DataWriter::new()?;
        writer.WriteBytes(&pixels)?;
        bitmap.CopyFromBuffer(&writer.DetachBuffer()?)?;

        // 优先英文识别（东南亚服的聊天基本是拉丁字母），没装英文语言包就用系统语言
        let engine = Language::CreateLanguage(&HSTRING::from("en-US"))
            .and_then(|lang| OcrEngine::TryCreateFromLanguage(&lang))
            .or_else(|_| OcrEngine::TryCreateFromUserProfileLanguages())
            .map_err(|_| anyhow::anyhow!("系统没有可用的 OCR 语言包，请在 Windows 设置 → 语言里添加英语"))?;
        let result = engine.RecognizeAsync(&bitmap)?.get()?;
        let mut lines = Vec::new();
        for line in result.Lines()? {
            let text = line.Text()?.to_string();
            if !text.trim().is_empty() {
                lines.push(text.trim().to_string());
            }
        }
        Ok(lines)
    }

    pub async fn run(app: AppHandle) -> anyhow::Result<()> {
        let settings = get_settings(&app)?.screen;
        let Some(region) = settings.region.clone() else {
            // 还没框选过：先框选，选完自动翻译
            open_selector(&app).map_err(|e| anyhow::anyhow!(e))?;
            return Ok(());
        };
        if BUSY.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let seconds = settings.overlay_seconds.max(3);
        let result = async {
            place_overlay(&app, &region)?;
            emit(&app, OverlayPayload { id, state: "pending".into(), items: vec![], message: "识别中…".into(), seconds, note: String::new(), note_error: false });

            let (w, h) = (region.width, region.height);
            let r = region.clone();
            let lines = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<Vec<String>> {
                let pixels = capture(&r)?;
                ocr(pixels, w, h)
            })
            .await??;
            if lines.is_empty() {
                anyhow::bail!("没有识别到文字。聊天框里有字吗？可以按 Alt+Shift+Q 重新框选");
            }

            let mut items: Vec<OcrItem> = lines
                .iter()
                .map(|l| {
                    let (name, text) = split_name(l);
                    OcrItem { name, text, zh: String::new() }
                })
                .collect();
            let todo: Vec<usize> = (0..items.len()).filter(|&i| needs_translation(&items[i].text)).collect();
            let texts: Vec<String> = todo.iter().map(|&i| items[i].text.clone()).collect();
            let zh = ai_translator::translate_incoming(&app, &texts).await?;
            for (k, &i) in todo.iter().enumerate() {
                items[i].zh = zh.get(k).cloned().unwrap_or_default();
            }
            let done = |note: String, note_error: bool| OverlayPayload { id, state: "done".into(), items: items.clone(), message: String::new(), seconds, note, note_error };
            emit(&app, done(if settings.push_enabled { "推送到网页…".into() } else { String::new() }, false));

            if settings.push_enabled {
                let (note, bad) = match push(&settings.push_url, &settings.push_token, &items).await {
                    Ok(_) => ("已推送到网页".to_string(), false),
                    Err(e) => (format!("推送到网页失败：{}", e), true),
                };
                println!("{}", note);
                emit(&app, done(note, bad));
            }
            anyhow::Ok(())
        }
        .await;
        BUSY.store(false, Ordering::SeqCst);
        if let Err(e) = &result {
            emit(&app, OverlayPayload { id, state: "error".into(), items: vec![], message: e.to_string(), seconds, note: String::new(), note_error: false });
        }
        result
    }
}

/// 快捷键：翻译记住的区域（没有区域就先框选）
pub fn trigger(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(target_os = "windows")]
        if let Err(e) = win::run(app).await {
            println!("截图翻译失败: {}", e);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = app;
    });
}

#[tauri::command]
pub async fn ocr_select_region(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return win::open_selector(&app);
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("截图翻译目前只支持 Windows".into())
    }
}

#[tauri::command]
pub async fn ocr_translate_now(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return win::run(app).await.map_err(|e| e.to_string());
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("截图翻译目前只支持 Windows".into())
    }
}

/// 框选窗口松开鼠标：坐标是框选窗口里的 CSS 像素，换算成屏幕物理像素后保存，然后马上翻译一次
#[tauri::command]
pub async fn ocr_region_selected(
    app: AppHandle,
    window: tauri::WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let origin = window.outer_position().map_err(|e| e.to_string())?;
    let region = OcrRegion {
        x: origin.x + (x * scale).round() as i32,
        y: origin.y + (y * scale).round() as i32,
        width: ((width * scale).round() as u32).max(1),
        height: ((height * scale).round() as u32).max(1),
    };
    println!("截图区域: {:?}", region);
    update_settings_field(&app, |s| s.screen.region = Some(region)).map_err(|e| e.to_string())?;
    let _ = window.close();
    // 等框选窗口真正消失再截图，否则会截到半透明遮罩
    tokio_sleep(300).await;
    trigger(&app);
    Ok(())
}

/// 设置页的「测试推送」：发一条测试消息，返回结果说明
#[tauri::command]
pub async fn ocr_test_push(url: String, token: String) -> Result<String, String> {
    let item = OcrItem {
        name: "DeepRant".into(),
        text: format!("推送测试 {}", chrono_like_now()),
        zh: "看到这条说明截图翻译能推送到网页".into(),
    };
    push(&url, &token, &[item]).await.map(|_| "推送成功，打开网页的「截图翻译」标签可以看到测试消息".to_string())
}

// 测试消息带上时间，免得被服务器当成重复消息不显示
fn chrono_like_now() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // UTC+8
    let t = (secs + 8 * 3600) % 86400;
    format!("{:02}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
}

#[tauri::command]
pub fn ocr_cancel_select(window: tauri::WebviewWindow) {
    let _ = window.close();
}

#[tauri::command]
pub fn ocr_last_overlay() -> Option<OverlayPayload> {
    last_overlay()
}

async fn tokio_sleep(ms: u64) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(std::time::Duration::from_millis(ms))).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    // 需要本地跑着 dota-translator 的 server（GSI_TOKEN=t），否则跳过：
    //   DT_TEST_URL=http://127.0.0.1:47999/ocr cargo test push_
    //   线上：DT_TEST_URL=https://dota.wanghaos.com/ocr DT_TEST_TOKEN=<口令> cargo test push_
    #[test]
    fn push_reports_results() {
        let Ok(url) = std::env::var("DT_TEST_URL") else { return };
        let token = std::env::var("DT_TEST_TOKEN").unwrap_or_else(|_| "t".into());
        let token = token.as_str();
        let item = OcrItem { name: "T".into(), text: format!("push test {}", chrono_like_now()), zh: "测试".into() };
        tauri::async_runtime::block_on(async {
            assert_eq!(push(&url, token, &[item.clone()]).await, Ok(1));
            assert_eq!(push(&url, token, &[item.clone()]).await, Ok(0), "重复消息不应再加");
            assert_eq!(push(&url, "wrong", &[item.clone()]).await, Err("口令不对（服务器返回 403）".into()));
            assert!(push(&url, "", &[item.clone()]).await.unwrap_err().contains("口令"));
            assert!(push(&url.replace("/ocr", "/nope"), token, &[item]).await.unwrap_err().contains("地址不对"));
        });
    }
}
