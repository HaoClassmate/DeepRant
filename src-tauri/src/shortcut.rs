// 导入所需的 Tauri 相关模块
use crate::shell_helper::trans_and_replace_text;
use crate::store::{get_settings, update_settings_field, HotkeyConfig};
use std::str::FromStr;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};

/// 从字符串转换为修饰键
fn parse_modifiers(modifiers: &[String]) -> Modifiers {
    let mut result = Modifiers::empty();
    for modifier in modifiers {
        match modifier.as_str() {
            "Control" | "ControlLeft" | "ControlRight" => result |= Modifiers::CONTROL,
            "Alt" | "AltLeft" | "AltRight" => result |= Modifiers::ALT,
            "Shift" | "ShiftLeft" | "ShiftRight" => result |= Modifiers::SHIFT,
            "Meta" | "MetaLeft" | "MetaRight" => result |= Modifiers::META,
            _ => {}
        }
    }
    result
}

/// 注册单个快捷键
fn register_shortcut<F>(
    app: &AppHandle,
    modifiers: &[String],
    key: &str,
    handler: F,
) -> Result<(), String>
where
    F: Fn(&AppHandle, &Shortcut, ShortcutEvent) + Send + Sync + 'static,
{
    println!("开始注册快捷键...");
    println!("修饰键: {:?}", modifiers);
    println!("主键: {}", key);

    let code = match Code::from_str(key) {
        Ok(c) => {
            println!("成功解析按键代码");
            c
        }
        Err(_) => {
            let err = format!("无效的按键代码: {}", key);
            println!("错误: {}", err);
            return Err(err);
        }
    };

    let parsed_modifiers = parse_modifiers(modifiers);
    println!("解析后的修饰键: {:?}", parsed_modifiers);

    let shortcut = Shortcut::new(Some(parsed_modifiers), code);
    println!("创建快捷键组合: {:?}", shortcut);

    let global_shortcut = app.global_shortcut();
    match global_shortcut.on_shortcut(shortcut, handler) {
        Ok(_) => {
            println!("快捷键注册成功");
            Ok(())
        }
        Err(e) => {
            let err = format!("注册快捷键失败: {}", e);
            println!("错误: {}", err);
            Err(err)
        }
    }
}

/// 更新快捷键
///
/// # 参数
/// * `app` - Tauri应用句柄
/// * `old_modifiers` - 旧的修饰键列表
/// * `old_key` - 旧的主键
/// * `new_modifiers` - 新的修饰键列表
/// * `new_key` - 新的主键
/// * `handler` - 快捷键触发时的处理函数
///
/// # 返回值
/// * `Result<(), String>` - 成功返回 Ok(()), 失败返回错误信息
fn update_shortcut<F>(
    app: &AppHandle,
    old_modifiers: &[String],
    old_key: &str,
    new_modifiers: &[String],
    new_key: &str,
    handler: F,
) -> Result<(), String>
where
    F: Fn(&AppHandle, &Shortcut, ShortcutEvent) + Send + Sync + 'static,
{
    println!("开始更新快捷键...");
    println!("旧快捷键: 修饰键={:?}, 主键={}", old_modifiers, old_key);
    println!("新快捷键: 修饰键={:?}, 主键={}", new_modifiers, new_key);

    let global_shortcut = app.global_shortcut();

    // 注销旧快捷键
    if let Ok(old_code) = Code::from_str(old_key) {
        println!("正在注销旧快捷键...");
        let old_shortcut = Shortcut::new(Some(parse_modifiers(old_modifiers)), old_code);
        match global_shortcut.unregister(old_shortcut) {
            Ok(_) => println!("成功注销旧快捷键"),
            Err(e) => {
                println!("注销现有快捷键失败: {}", e);
                // 继续执行,因为旧快捷键可能本来就不存在
            }
        }
    } else {
        println!("旧快捷键格式无效,跳过注销步骤");
    }

    // 注册新快捷键
    println!("正在注册新快捷键...");
    match register_shortcut(app, new_modifiers, new_key, handler) {
        Ok(_) => {
            println!("新快捷键注册成功");
            Ok(())
        }
        Err(e) => {
            println!("新快捷键注册失败: {}", e);
            Err(e)
        }
    }
}

/// 初始化所有快捷键
pub fn init_shortcuts(app: &AppHandle) -> Result<(), String> {
    let settings = get_settings(app).map_err(|e| e.to_string())?;

    // 注册翻译快捷键
    register_shortcut(
        app,
        &settings.trans_hotkey.modifiers,
        &settings.trans_hotkey.key,
        create_trans_handler(app.clone()),
    )?;

    // 截图翻译（只有 Windows 能用）。注册失败（比如快捷键被别的程序占了）不影响翻译快捷键
    #[cfg(target_os = "windows")]
    {
        let screen = &settings.screen;
        if let Err(e) = register_shortcut(app, &screen.hotkey.modifiers, &screen.hotkey.key, screen_translate_handler) {
            println!("注册截图翻译快捷键失败: {}", e);
        }
        if let Err(e) = register_shortcut(app, &screen.select_hotkey.modifiers, &screen.select_hotkey.key, screen_select_handler) {
            println!("注册框选快捷键失败: {}", e);
        }
    }

    Ok(())
}

/// 把前端录到的按键数组（最后一个是主键）解析成快捷键配置，带上显示用的文字
fn hotkey_from_keys(keys: &[String]) -> Result<HotkeyConfig, String> {
    let is_modifier = |k: &String| {
        k.contains("Control") || k.contains("Alt") || k.contains("Shift") || k.contains("Meta")
    };
    if keys.len() < 2 || !keys.iter().any(is_modifier) || !keys.iter().any(|k| !is_modifier(k)) {
        return Err(
            "快捷键必须包含至少一个修饰键(Control/Alt/Shift/Command)和一个其他按键".to_string(),
        );
    }
    let modifiers: Vec<String> = keys[..keys.len() - 1]
        .iter()
        .map(|k| k.replace("Left", "").replace("Right", ""))
        .collect();
    let key = keys[keys.len() - 1].clone();
    let shortcut = format!(
        "{}+{}",
        modifiers
            .iter()
            .map(|m| match m.as_str() {
                "Control" =>
                    if cfg!(target_os = "macos") {
                        "⌃"
                    } else {
                        "Ctrl"
                    },
                "Alt" =>
                    if cfg!(target_os = "macos") {
                        "⌥"
                    } else {
                        "Alt"
                    },
                "Shift" => "⇧",
                "Meta" =>
                    if cfg!(target_os = "macos") {
                        "⌘"
                    } else {
                        "Win"
                    },
                _ => m,
            })
            .collect::<Vec<_>>()
            .join("+"),
        format_key_display(&key)
    );
    Ok(HotkeyConfig { modifiers, key, shortcut })
}

/// 两个快捷键是不是同一组按键（修饰键顺序无关）
fn same_hotkey(a: &HotkeyConfig, b: &HotkeyConfig) -> bool {
    let norm = |h: &HotkeyConfig| {
        let mut m = h.modifiers.clone();
        m.sort();
        m.dedup();
        (m, h.key.clone())
    };
    norm(a) == norm(b)
}

/// 新快捷键不能和 DeepRant 自己的其他快捷键重复；`except` 是正在修改的那个
fn check_conflict(app: &AppHandle, new: &HotkeyConfig, except: &str) -> Result<(), String> {
    let settings = get_settings(app).map_err(|e| e.to_string())?;
    let mut others = vec![("translate", "翻译", &settings.trans_hotkey)];
    if cfg!(target_os = "windows") {
        others.push(("screen", "截图翻译", &settings.screen.hotkey));
        others.push(("select", "框选区域", &settings.screen.select_hotkey));
    }
    for (id, name, hotkey) in others {
        if id != except && same_hotkey(new, hotkey) {
            return Err(format!("和「{}」的快捷键重复了", name));
        }
    }
    Ok(())
}

/// 更新翻译快捷键
pub fn update_translator_shortcut(
    app: &AppHandle,
    keys: Vec<String>, // 直接接收按键数组
) -> Result<(), String> {
    println!("正在更新翻译快捷键: {:?}", keys);
    let new_hotkey = hotkey_from_keys(&keys)?;
    check_conflict(app, &new_hotkey, "translate")?;
    let settings = get_settings(app).map_err(|e| e.to_string())?;

    update_shortcut(
        app,
        &settings.trans_hotkey.modifiers,
        &settings.trans_hotkey.key,
        &new_hotkey.modifiers,
        &new_hotkey.key,
        create_trans_handler(app.clone()),
    )?;
    update_settings_field(app, |settings| settings.trans_hotkey = new_hotkey)
        .map_err(|e| format!("快捷键已更新，但保存设置失败: {}", e))?;
    println!("翻译快捷键更新成功");
    Ok(())
}

/// 截图翻译：按下翻译记住的区域
fn screen_translate_handler(app: &AppHandle, _shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() == ShortcutState::Pressed {
        crate::screen_ocr::trigger(app);
    }
}

/// 截图翻译：按下重新框选区域
fn screen_select_handler(app: &AppHandle, _shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() == ShortcutState::Pressed {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = crate::screen_ocr::ocr_select_region(app).await {
                println!("打开框选失败: {}", e);
            }
        });
    }
}

/// 更新截图翻译的快捷键；`which` 是 "screen"（翻译）或 "select"（框选）
pub fn update_screen_shortcut(app: &AppHandle, which: &str, keys: Vec<String>) -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Err("截图翻译目前只支持 Windows".to_string());
    }
    println!("正在更新截图翻译快捷键 {}: {:?}", which, keys);
    let new_hotkey = hotkey_from_keys(&keys)?;
    check_conflict(app, &new_hotkey, which)?;
    let screen = get_settings(app).map_err(|e| e.to_string())?.screen;

    match which {
        "screen" => update_shortcut(
            app,
            &screen.hotkey.modifiers,
            &screen.hotkey.key,
            &new_hotkey.modifiers,
            &new_hotkey.key,
            screen_translate_handler,
        )?,
        "select" => update_shortcut(
            app,
            &screen.select_hotkey.modifiers,
            &screen.select_hotkey.key,
            &new_hotkey.modifiers,
            &new_hotkey.key,
            screen_select_handler,
        )?,
        _ => return Err(format!("未知的快捷键: {}", which)),
    }
    update_settings_field(app, |settings| {
        if which == "screen" {
            settings.screen.hotkey = new_hotkey;
        } else {
            settings.screen.select_hotkey = new_hotkey;
        }
    })
    .map_err(|e| format!("快捷键已更新，但保存设置失败: {}", e))?;
    Ok(())
}

/// 创建翻译快捷键处理函数
fn create_trans_handler(
    app: AppHandle,
) -> impl Fn(&AppHandle, &Shortcut, ShortcutEvent) + Send + Sync + 'static {
    let app = Arc::new(app);
    move |_app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let app_clone = Arc::clone(&app);
            tauri::async_runtime::spawn(async move {
                if let Err(e) = trans_and_replace_text(app_clone.as_ref()).await {
                    println!("翻译替换失败: {:?}", e);
                }
            });
        }
    }
}

/// 格式化键盘代码为用户友好的显示文本
fn format_key_display(key: &str) -> String {
    if key.starts_with("Key") {
        key[3..].to_string()
    } else if key.starts_with("Digit") {
        key[5..].to_string()
    } else if key.starts_with("Arrow") {
        match key {
            "ArrowUp" => "↑".to_string(),
            "ArrowDown" => "↓".to_string(),
            "ArrowLeft" => "←".to_string(),
            "ArrowRight" => "→".to_string(),
            _ => key.to_string(),
        }
    } else {
        match key {
            "Space" => "空格".to_string(),
            "Tab" => "Tab".to_string(),
            "Enter" => "↵".to_string(),
            "Backspace" => "⌫".to_string(),
            "Delete" => "Del".to_string(),
            "Escape" => "Esc".to_string(),
            "CapsLock" => "⇪".to_string(),
            _ => key.to_string(),
        }
    }
}
