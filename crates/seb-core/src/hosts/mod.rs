//! 本地 hosts 的托管块。只改 START/END 之间的内容，用户自己写的行原样保留

use std::path::PathBuf;

pub const DEFAULT_HOSTS_INNER: &str = include_str!("inner.hosts");
pub const DEFAULT_HOSTS_UAT: &str = include_str!("uat.hosts");

const START: &str = "# Added by GY TOOLS HOSTS START";
const END: &str = "# Added by GY TOOLS HOSTS END";

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
}
