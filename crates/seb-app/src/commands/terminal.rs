use std::process::Command;

#[tauri::command]
pub async fn open_terminal_log(
    device_no: Option<String>,
    bike_no: Option<String>,
) -> Result<String, String> {
    let mut patterns = Vec::new();
    if let Some(d) = device_no {
        let d = d.trim().to_string();
        if !d.is_empty() {
            patterns.push(d);
        }
    }
    if let Some(b) = bike_no {
        let b = b.trim().to_string();
        if !b.is_empty() {
            patterns.push(b);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let filter_arg = if patterns.is_empty() {
            String::new()
        } else {
            let regex = patterns.join("|");
            format!(" | grep --color=always -E \"{regex}\"")
        };

        let cmd = format!(
            r#"export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
if command -v stern >/dev/null 2>&1; then
    echo "使用 stern 监听 seb-iot-receiver 日志..."
    stern 'seb-iot-receiver' -n shared-electric-bicycle --tail=200{filter_arg}
elif command -v kubectl >/dev/null 2>&1; then
    echo "未检测到 stern，自动降级为 kubectl logs 监听..."
    kubectl logs -n shared-electric-bicycle -l app=seb-iot-receiver --tail=200 -f --prefix=true{filter_arg}
else
    echo "【错误】未检测到 stern 或 kubectl 工具。"
    echo "请先在终端安装 stern 或 kubectl："
    echo "  brew install stern"
    echo "  或者 brew install kubectl"
fi"#
        );

        let escaped_cmd = cmd.replace('\\', "\\\\").replace('"', "\\\"");
        let apple_script = format!(
            r#"tell application "Terminal"
                activate
                do script "{escaped_cmd}"
            end tell"#
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&apple_script)
            .output()
            .map_err(|e| format!("拉起终端失败: {e}"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("AppleScript 执行错误: {err}"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let filter_win = if patterns.is_empty() {
            String::new()
        } else {
            let regex = patterns.join("|");
            format!(" | Select-String -Pattern '{regex}'")
        };

        let win_cmd = format!(
            r#"if (Get-Command stern -ErrorAction SilentlyContinue) {{
    Write-Host "使用 stern 监听 seb-iot-receiver 日志..." -ForegroundColor Green
    stern 'seb-iot-receiver' -n shared-electric-bicycle --tail=200{filter_win}
}} elseif (Get-Command kubectl -ErrorAction SilentlyContinue) {{
    Write-Host "未检测到 stern，自动降级为 kubectl logs 监听..." -ForegroundColor Yellow
    kubectl logs -n shared-electric-bicycle -l app=seb-iot-receiver --tail=200 -f --prefix=true{filter_win}
}} else {{
    Write-Host "【错误】未检测到 stern 或 kubectl 工具。" -ForegroundColor Red
    Write-Host "请先安装 stern：scoop install stern 或 winget install stern"
}}"#
        );

        Command::new("cmd")
            .args(["/c", "start", "powershell", "-NoExit", "-Command", &win_cmd])
            .spawn()
            .map_err(|e| format!("Windows 拉起 PowerShell 失败: {e}"))?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        return Err("当前仅支持 macOS / Windows 终端拉起".to_string());
    }

    let display_cmd = if patterns.is_empty() {
        "stern 'seb-iot-receiver' -n shared-electric-bicycle --tail=200".to_string()
    } else {
        format!(
            "stern/kubectl 'seb-iot-receiver' -n shared-electric-bicycle | grep \"{}\"",
            patterns.join("|")
        )
    };

    Ok(display_cmd)
}
