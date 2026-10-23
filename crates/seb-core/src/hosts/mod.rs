//! 本地 hosts 的托管块。只改 START/END 之间的内容，用户自己写的行原样保留

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_HOSTS_INNER: &str = include_str!("inner.hosts");
pub const DEFAULT_HOSTS_UAT: &str = include_str!("uat.hosts");

const START: &str = "# Added by GY TOOLS HOSTS START";
const END: &str = "# Added by GY TOOLS HOSTS END";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteHostsSyncResult {
    pub inner: String,
    pub uat: String,
    pub inner_count: usize,
    pub uat_count: usize,
}

#[derive(Deserialize)]
struct GyHostApiResponse {
    code: i32,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Vec<GySystemItem>,
}

#[derive(Deserialize)]
struct GySystemItem {
    #[serde(default)]
    system_name: String,
    #[serde(default)]
    system_line: String,
    #[serde(default)]
    gateway_list: Vec<GyGatewayItem>,
}

#[derive(Deserialize)]
struct GyGatewayItem {
    #[serde(default)]
    gateway_name: String,
    #[serde(default)]
    host_list: Vec<GyHostEntry>,
}

#[derive(Deserialize)]
struct GyHostEntry {
    #[serde(default)]
    ip: String,
    #[serde(default)]
    mark: String,
}

/// 从光宇内部接口拉取最新全量 hosts 配置，并解析出内网（sit）与外网（uat）分组
pub async fn fetch_gy_hosts() -> anyhow::Result<RemoteHostsSyncResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let res = client
        .get("http://i.gydev.cn/gytools/user/host/list")
        .send()
        .await?
        .error_for_status()?
        .json::<GyHostApiResponse>()
        .await?;

    if res.code != 0 {
        anyhow::bail!("获取光宇 Hosts 失败: {}", res.message);
    }

    let mut inner_lines = Vec::new();
    let mut uat_lines = Vec::new();
    let mut inner_count = 0;
    let mut uat_count = 0;

    for sys in res.data {
        let line = sys.system_line.trim();
        let name = sys.system_name.trim();

        let mut sys_inner = Vec::new();
        let mut sys_uat = Vec::new();

        for gw in sys.gateway_list {
            let domain = gw.gateway_name.trim();
            if domain.is_empty() {
                continue;
            }
            for h in gw.host_list {
                let ip = h.ip.trim();
                if ip.is_empty() {
                    continue;
                }
                match h.mark.as_str() {
                    "sit" => {
                        sys_inner.push(format!("{:<16}{}", ip, domain));
                        inner_count += 1;
                    }
                    "uat" => {
                        sys_uat.push(format!("{:<16}{}", ip, domain));
                        uat_count += 1;
                    }
                    _ => {}
                }
            }
        }

        if !sys_inner.is_empty() {
            inner_lines.push(format!("# GY TOOLS GROUP: env_sit_{line}_{name}"));
            inner_lines.extend(sys_inner);
            inner_lines.push(String::new());
        }

        if !sys_uat.is_empty() {
            uat_lines.push(format!("# GY TOOLS GROUP: env_uat_{line}_{name}"));
            uat_lines.extend(sys_uat);
            uat_lines.push(String::new());
        }
    }

    Ok(RemoteHostsSyncResult {
        inner: inner_lines.join("\n").trim().to_string(),
        uat: uat_lines.join("\n").trim().to_string(),
        inner_count,
        uat_count,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HostsEnv {
    Inner,
    Uat,
    Prod,
}

impl HostsEnv {
    pub fn next(self) -> Self {
        match self {
            HostsEnv::Inner => HostsEnv::Uat,
            HostsEnv::Uat => HostsEnv::Prod,
            HostsEnv::Prod => HostsEnv::Inner,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            HostsEnv::Inner => "内网",
            HostsEnv::Uat => "外网",
            HostsEnv::Prod => "正式",
        }
    }
}

pub fn hosts_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    } else {
        PathBuf::from("/etc/hosts")
    }
}

/// 取出托管块的内容，没有托管块就是空
pub fn managed_block(text: &str) -> &str {
    let Some(rest) = text.split_once(START).map(|(_, r)| r) else {
        return "";
    };
    rest.split_once(END).map(|(b, _)| b).unwrap_or(rest).trim_matches('\n')
}

/// 用新内容替换托管块；block 为空则连标记一起删掉
pub fn apply_block(text: &str, block: &str) -> String {
    let block = block.trim_matches('\n');
    let (head, tail) = match (text.find(START), text.find(END)) {
        (Some(s), Some(e)) if e > s => (&text[..s], &text[e + END.len()..]),
        (Some(s), _) => (&text[..s], ""),
        _ => (text, ""),
    };
    let head = head.trim_end_matches('\n');
    let tail = tail.trim_start_matches('\n');

    let mut out = String::from(head);
    if !block.is_empty() {
        out.push_str("\n\n");
        out.push_str(START);
        out.push('\n');
        out.push_str(block);
        out.push('\n');
        out.push_str(END);
    }
    if !tail.is_empty() {
        out.push_str("\n\n");
        out.push_str(tail.trim_end_matches('\n'));
    }
    out.push('\n');
    out
}

/// 比较两段 hosts 内容：空行和空白多少不算差异
fn same_block(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    };
    norm(a) == norm(b)
}

/// 当前 hosts 命中哪个环境，对不上任何一个就返回 None
pub fn detect(current: &str, inner: &str, uat: &str, prod: &str) -> Option<HostsEnv> {
    for (env, block) in [
        (HostsEnv::Inner, inner),
        (HostsEnv::Uat, uat),
        (HostsEnv::Prod, prod),
    ] {
        if same_block(current, block) {
            return Some(env);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW: &str = "127.0.0.1\tlocalhost\n\n# Added by GY TOOLS HOSTS START\n1.1.1.1 a.cn\n# Added by GY TOOLS HOSTS END\n";

    #[test]
    fn keeps_user_lines_and_swaps_block() {
        assert_eq!(managed_block(RAW), "1.1.1.1 a.cn");
        let next = apply_block(RAW, "2.2.2.2 b.cn");
        assert!(next.starts_with("127.0.0.1\tlocalhost"));
        assert_eq!(managed_block(&next), "2.2.2.2 b.cn");
    }

    #[test]
    fn empty_block_removes_markers() {
        let cleared = apply_block(RAW, "");
        assert_eq!(cleared, "127.0.0.1\tlocalhost\n");
        assert_eq!(managed_block(&cleared), "");
    }

    #[test]
    fn detect_ignores_blank_lines() {
        assert_eq!(detect("1.1.1.1 a.cn", "\n1.1.1.1  a.cn\n\n", "x", ""), Some(HostsEnv::Inner));
        assert_eq!(detect("", "a", "b", ""), Some(HostsEnv::Prod));
        assert_eq!(detect("zzz", "a", "b", ""), None);
    }

    #[tokio::test]
    async fn test_fetch_gy_hosts() {
        // 如果在光宇内网环境，可以验证真实拉取与解析
        if let Ok(res) = fetch_gy_hosts().await {
            assert!(res.inner_count > 0);
            assert!(res.uat_count > 0);
            assert!(res.inner.contains("env_sit_"));
            assert!(res.uat.contains("env_uat_"));
        }
    }
}
