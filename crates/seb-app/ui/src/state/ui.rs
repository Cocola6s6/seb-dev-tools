
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
    pub level: LogLevel,
    pub tint: &'static str,
    /// 收发报文才有：Some("up") 上行、Some("down") 下行
    pub dir: Option<&'static str>,
    /// 模拟设备产生的日志才有
    pub device: String,
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
