use crate::ai_translator;
use anyhow::Result;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
#[cfg(target_os = "macos")]
use tauri_plugin_shell::ShellExt;

pub async fn trans_and_replace_text(app: &AppHandle) -> Result<()> {
    let start = Instant::now();
    let settings = crate::store::get_settings(app)?;

    // 1. 复制选中文本：先清空剪贴板，按完 Ctrl+C 后等到剪贴板里有新内容再读
    app.clipboard().write_text("")?;
    if !settings.daily_mode {
        simulate_keyboard_shortcut(app, "a").await?;
    }
    simulate_keyboard_shortcut(app, "c").await?;
    // 2. 读取剪贴板内容
    let original_text = wait_clipboard_text(app, Duration::from_millis(500)).await;
    println!("原始文本: {:?}（复制用时 {:?}）", original_text, start.elapsed());
    if original_text.trim().is_empty() {
        println!("没有复制到文字，跳过翻译");
        return Ok(());
    }

    if !settings.daily_mode {
        // 3. 如果是游戏模式 -> 显示翻译状态
        let status_text = format!(
            "DeepRant翻译中... ({}→{} | 场景:{} | 模式:{})",
            settings.translation_from,
            settings.translation_to,
            settings.game_scene,
            settings.translation_mode
        );
        app.clipboard().write_text(&status_text)?;

        // 4. 粘贴状态文本
        simulate_keyboard_shortcut(app, "a").await?;
        simulate_keyboard_shortcut(app, "v").await?;
    }

    // 5. 调用AI翻译
    let request_start = Instant::now();
    let translated = ai_translator::translate_with_gpt(app, &original_text).await?;
    println!("翻译结果: {:?}（模型用时 {:?}）", translated, request_start.elapsed());

    // 6. 粘贴翻译结果
    app.clipboard().write_text(translated)?;
    if !settings.daily_mode {
        simulate_keyboard_shortcut(app, "a").await?;
    }
    simulate_keyboard_shortcut(app, "v").await?;
    println!("翻译总用时 {:?}", start.elapsed());

    Ok(())
}

/// 轮询剪贴板直到有文字，最多等 `max`；Ctrl+C 之后剪贴板不是立刻就更新
async fn wait_clipboard_text(app: &AppHandle, max: Duration) -> String {
    let start = Instant::now();
    loop {
        let text = app.clipboard().read_text().unwrap_or_default();
        if !text.is_empty() || start.elapsed() >= max {
            return text;
        }
        sleep(Duration::from_millis(15)).await;
    }
}

async fn sleep(d: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

pub async fn has_text_selection(app: &AppHandle) -> Result<bool> {
    // 模拟 Cmd+C/Ctrl+C
    simulate_keyboard_shortcut(app, "c").await?;

    // 读取复制后的剪贴板内容
    let new_clipboard = app.clipboard().read_text().unwrap_or_default();

    // 如果新的剪贴板内容不为空，说明有选中的文本
    let has_selection = !new_clipboard.is_empty();

    Ok(has_selection)
}

/// 模拟键盘组合键按下
async fn simulate_keyboard_shortcut(app: &AppHandle, key: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let shell = app.shell();
        let script = format!(
            r#"
            tell application "System Events"
                keystroke "{}" using command down
                delay 0.1
            end tell
            "#,
            key
        );

        let output = shell
            .command("osascript")
            .args(["-e", &script])
            .output()
            .await?;

        if !output.status.success() {
            println!("按键模拟失败: {:?}", String::from_utf8(output.stderr)?);
            return Ok(());
        }
    }

    // 以前每按一次键都要启动一个 PowerShell（0.3~1 秒），一次翻译按 6 次键，光这里就要好几秒。
    // 现在直接调 SendInput，几毫秒。
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        send_ctrl_combo(key)?;
        // 给目标程序一点时间处理按键（全选/粘贴）
        sleep(Duration::from_millis(30)).await;
    }

    Ok(())
}

/// Ctrl+字母。按快捷键（默认 Alt+T）时 Alt 往往还按着，会变成 Ctrl+Alt+字母：
/// 先按下 Ctrl 再松开 Alt/Shift/Win（Ctrl 按着时松开 Alt 不会激活菜单栏），然后按字母
#[cfg(target_os = "windows")]
fn send_ctrl_combo(key: &str) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    let letter = key
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic())
        .ok_or_else(|| anyhow::anyhow!("不支持的按键: {}", key))?
        .to_ascii_uppercase();
    let input = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let mut inputs = vec![input(VK_CONTROL, false)];
    for vk in [VK_LMENU, VK_RMENU, VK_LSHIFT, VK_RSHIFT, VK_LWIN, VK_RWIN] {
        // 最高位为 1 表示这个键现在是按下的
        if unsafe { GetAsyncKeyState(vk.0 as i32) } < 0 {
            inputs.push(input(vk, true));
        }
    }
    let vk = VIRTUAL_KEY(letter as u16);
    inputs.push(input(vk, false));
    inputs.push(input(vk, true));
    inputs.push(input(VK_CONTROL, true));

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        anyhow::bail!("模拟按键失败（只发出了 {}/{} 个）", sent, inputs.len());
    }
    Ok(())
}

pub async fn send_phrase(app: &AppHandle, phrase: &str) -> Result<()> {
    // 将短语写入剪贴板
    app.clipboard().write_text(phrase)?;

    // 模拟粘贴操作
    simulate_keyboard_shortcut(app, "v").await?;

    Ok(())
}
