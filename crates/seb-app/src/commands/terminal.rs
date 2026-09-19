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

        let script = format!(
            r#"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001 > $null
if (Get-Command stern -ErrorAction SilentlyContinue) {{
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

        let encoded = encode_ps_command(&script);

        Command::new("cmd")
            .args(["/c", "start", "powershell", "-NoExit", "-EncodedCommand", &encoded])
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

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(&url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd").args(["/c", "start", "", &url]).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(&url).spawn();
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn encode_ps_command(script: &str) -> String {
    let utf16_bytes: Vec<u8> = script
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();

    const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((utf16_bytes.len() + 2) / 3 * 4);
    for chunk in utf16_bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);

        out.push(B64_CHARS[(b0 >> 2) as usize] as char);
        out.push(B64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
