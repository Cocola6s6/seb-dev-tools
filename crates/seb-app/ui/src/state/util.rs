
use crate::state::GlobalSettings;

/// 中控指令、中控配置这些只打内网后台，所以要认得出设备连的是哪套网关
const INNER_HOST: &str = "bike-seb-inner-test.costrip.cn";

/// 配置页填的网关是 "host:port"，判环境只看 host
fn gw_host(gw: &str) -> &str {
    gw.rsplit_once(':').map(|(h, _)| h).unwrap_or(gw).trim()
}

/// 中控指令只打内网，内网网关以配置页为准
pub fn inner_host(s: &GlobalSettings) -> String {
    let configured = gw_host(&s.client.default_inner_gw);
    if configured.is_empty() {
        INNER_HOST.to_string()
    } else {
        configured.to_string()
    }
}

pub fn host_label(s: &GlobalSettings, host: &str) -> String {
    match host_env_tag(s, host) {
        ("自定义", _) => host.to_string(),
        (label, _) => label.to_string(),
    }
}

/// 返回 (环境名, 配色后缀)，后缀拼成 badge-env-*（标签）或 env-*（色条）
pub fn host_env_tag(s: &GlobalSettings, host: &str) -> (&'static str, &'static str) {
    let matches = |gw: &str| {
        let h = gw_host(gw);
        !h.is_empty() && h == host
    };
    if matches(&s.client.default_inner_gw) || matches(&s.battery.default_inner_gw) {
        ("内网", "inner")
    } else if matches(&s.client.default_test_gw) || matches(&s.battery.default_test_gw) {
        ("外网", "test")
    } else if matches(&s.client.default_prod_gw) || matches(&s.battery.default_prod_gw) {
        ("正式", "prod")
    } else if host.contains("bike-seb-inner-test") || host.contains("10.12.55.31") {
        ("内网", "inner")
    } else if host.contains("bike-seb-test") || host.contains("140.143.180.28") {
        ("外网", "test")
    } else if host.contains("bike-seb.costrip.cn") || host.contains("140.143.214.51") {
        ("正式", "prod")
    } else if host.is_empty() {
        ("未配置", "none")
    } else {
        ("自定义", "custom")
    }
}

/// 设备编号尾号辨识度最高，超长时从中间省略，首尾都留着
pub fn elide_middle(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max || max < 4 {
        return s.to_string();
    }
    let head = (max - 1) * 3 / 5;
    let tail = max - 1 - head;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

pub const DEFAULT_BIKE_QR: &str = "https://gycx.cn?s={bike_no}";
pub const DEFAULT_BATTERY_QR: &str = "https://cosbike.net.cn/qr?{battery_no}";

/// 二维码地址模板，配置页留空就用内置的
pub fn qr_url(template: &str, fallback: &str, placeholder: &str, no: &str) -> String {
    let t = template.trim();
    let t = if t.is_empty() { fallback } else { t };
    t.replace(placeholder, no)
}

pub fn now_hms() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}
