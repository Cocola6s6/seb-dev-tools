use super::AppState;
use seb_core::hosts::{self, HostsEnv};
use tauri::State;

/// 按 内网 → 外网 → 正式 轮换本地 hosts，返回切换后的环境名
#[tauri::command]
pub async fn switch_hosts(state: State<'_, AppState>) -> Result<String, String> {
    let sets = {
        let publisher = state.publisher.lock().await;
        publisher.config().settings.clone()
    };

    let path = hosts::hosts_path();
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;

    // 认不出当前是哪个环境就从内网开始
    let next = hosts::detect(
        hosts::managed_block(&text),
        &sets.hosts_inner,
        &sets.hosts_uat,
        &sets.hosts_prod,
    )
    .map(HostsEnv::next)
    .unwrap_or(HostsEnv::Inner);

    let block = match next {
        HostsEnv::Inner => &sets.hosts_inner,
        HostsEnv::Uat => &sets.hosts_uat,
        HostsEnv::Prod => &sets.hosts_prod,
    };
    let body = hosts::apply_block(&text, block);

    // 提权写盘会弹系统密码框，用户可能愣好一会儿，别占着异步线程
    tokio::task::spawn_blocking(move || write_hosts(&body))
        .await
        .map_err(|e| format!("写入 hosts 失败: {e}"))??;

    Ok(next.label().to_string())
}

/// hosts 当前能不能直接写：能写就不用再弹密码
#[tauri::command]
pub fn hosts_writable() -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open(hosts::hosts_path())
        .is_ok()
}

/// 把 hosts 的属主在「当前用户」和「root:wheel」之间来回换：免密开关
#[tauri::command]
pub async fn set_hosts_free(enable: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner = if enable {
            std::env::var("USER").map_err(|_| "读取当前用户失败".to_string())?
        } else {
            "root:wheel".to_string()
        };
        let shell = format!("/usr/sbin/chown '{owner}' /etc/hosts");
        tokio::task::spawn_blocking(move || elevated(&shell))
            .await
            .map_err(|e| e.to_string())?
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = enable;
        Err("当前系统不支持免密授权".to_string())
    }
}

/// 先按普通文件写，写不动再走提权；授权过的机器就一路无感
fn write_hosts(body: &str) -> Result<(), String> {
    let path = hosts::hosts_path();
    if std::fs::write(&path, body).is_ok() {
        flush_dns();
        return Ok(());
    }

    let tmp = std::env::temp_dir().join("seb-dev-tools-hosts");
    std::fs::write(&tmp, body).map_err(|e| format!("写入临时文件失败: {e}"))?;
    let tmp = tmp.display().to_string();

    #[cfg(target_os = "macos")]
    {
        elevated(&format!(
            "/bin/cp '{tmp}' /etc/hosts && {{ /usr/bin/dscacheutil -flushcache; /usr/bin/killall -HUP mDNSResponder; true; }}"
        ))?;
    }

    #[cfg(target_os = "windows")]
    {
        let target = seb_core::hosts::hosts_path().display().to_string();
        let script = format!(
            "$p = Start-Process -FilePath cmd.exe -ArgumentList '/c copy /y \"{tmp}\" \"{target}\" && ipconfig /flushdns' -Verb RunAs -Wait -PassThru\nexit $p.ExitCode"
        );
        let encoded = super::terminal::encode_ps_command(&script);
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-EncodedCommand", &encoded])
            .status()
            .map_err(|e| format!("提权失败: {e}"))?;
        if !status.success() {
            return Err("写入 hosts 失败，可能是未通过管理员授权".to_string());
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        return Err("当前系统暂不支持自动切换 hosts".to_string());
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    Ok(())
}

/// 走系统授权框执行一条 shell，用户取消要说清楚不是失败
#[cfg(target_os = "macos")]
fn elevated(shell: &str) -> Result<(), String> {
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        shell.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let out = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("提权失败: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("-128") {
            return Err("已取消授权".to_string());
        }
        return Err(err.trim().to_string());
    }
    Ok(())
}

/// 属主换成当前用户后刷缓存是没权限的，失败就算了：mDNSResponder 自己也会跟进 hosts 变化
fn flush_dns() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("dscacheutil").arg("-flushcache").status();
        let _ = std::process::Command::new("killall").args(["-HUP", "mDNSResponder"]).status();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("ipconfig").arg("/flushdns").status();
    }
}
