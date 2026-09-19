
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn css(&self) -> &'static str {
        match self {
            LogLevel::Info => "log-info",
            LogLevel::Warn => "log-warn",
            LogLevel::Error => "log-error",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub ts: String,
    pub text: String,
    /// 报文原文，和 fields 一起展开时用来定位字节
    pub hex: String,
    pub fields: Vec<super::client::Field>,
    pub level: LogLevel,
    pub tint: &'static str,
    /// 收发报文才有：Some("up") 上行、Some("down") 下行
    pub dir: Option<&'static str>,
    /// 模拟设备产生的日志才有
    pub device: String,
}

impl LogEntry {
    /// 每个词都要命中才算（多词是「在这辆车的基础上再查」），命中在展开区时返回 true 好自动展开
    pub fn hit(&self, terms: &[String]) -> Option<bool> {
        let row = format!("{} {}", self.device, self.text).to_lowercase();
        // 报文按连续十六进制匹配，这样 "2C" 和 "00 2C" 都能查到
        let hex = self.hex.replace(' ', "").to_lowercase();
        let detail = self
            .fields
            .iter()
            .map(|f| format!("{} {} {}\n", f.key, f.label, f.value))
            .collect::<String>()
            .to_lowercase();
        let mut in_detail = false;
        for t in terms {
            let in_row = row.contains(t.as_str());
            let deep = hex.contains(t.as_str()) || detail.contains(t.as_str());
            if !in_row && !deep {
                return None;
            }
            in_detail |= deep;
        }
        Some(in_detail)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Deploy,
    Control,
    Ecu,
    Client,
    Battery,
}

impl Page {
    pub fn index(self) -> usize {
        match self {
            Page::Deploy => 0,
            Page::Control => 1,
            Page::Ecu => 2,
            Page::Client => 3,
            Page::Battery => 4,
        }
    }

    pub fn tint(self) -> &'static str {
        match self {
            Page::Deploy => "deploy",
            Page::Control => "control",
            Page::Ecu => "ecu",
            Page::Client => "client",
            Page::Battery => "battery",
        }
    }

    pub fn is_client(self) -> bool {
        matches!(self, Page::Client | Page::Battery)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolboxMode {
    QrCode,
    Settings,
    System,
}

impl ToolboxMode {
    /// 双击百宝箱按顺序轮换，新功能往这条链后面加
    pub fn next(self) -> Self {
        match self {
            ToolboxMode::QrCode => ToolboxMode::Settings,
            ToolboxMode::Settings => ToolboxMode::System,
            ToolboxMode::System => ToolboxMode::QrCode,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ToolboxMode::QrCode => "二维码",
            ToolboxMode::Settings => "全局配置",
            ToolboxMode::System => "系统配置",
        }
    }
}
