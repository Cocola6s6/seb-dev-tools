pub mod config;
pub mod db;
pub mod device;
pub mod ecu;
pub mod flink;
pub mod frame;
pub mod mq;
pub mod payload;
pub mod redis;

pub use config::AppConfig;
pub use flink::{FlinkJobStatus, FlinkState};
pub use mq::Publisher;

pub mod exchange {
    pub const CONTROL: &str = "seb.command.test";
    pub const QUERY: &str = "seb.query.command.test";
    pub const SET: &str = "seb.set.command.test";
    pub const VOICE: &str = "seb.voice.command.test";
}

pub fn routing_key(exchange: &str, instance: &str) -> String {
    let instance = instance.trim();
    let instance = if instance.is_empty() { "0" } else { instance };
    format!("{exchange}-{instance}")
}

pub mod command_code {
    pub const QUERY: u16 = 0x07;
    pub const SET: u16 = 0x08;
    pub const VOICE: u16 = 0x0C;
    pub const CONTROL: u16 = 0x2C;
}

pub const CONTROL_TYPES: &[(&str, u16)] = &[
    ("远程设防", 0x01),
    ("远程撤防", 0x02),
    ("远程重启", 0x03),
    ("立即定位", 0x08),
    ("用户寻车", 0x09),
    ("远程开锁(业务开锁)", 0x0B),
    ("强制还车", 0x0C),
    ("上报BMS电池数据", 0x0D),
    ("远程打开电池锁", 0x0F),
    ("远程关闭电池锁", 0x10),
    ("远程打开后轮锁/头盔锁", 0x16),
    ("远程关闭后轮锁/头盔锁", 0x17),
    ("失能ACC", 0x1B),
    ("使能ACC", 0x1C),
    ("远程临时锁车", 0x30),
    ("远程恢复开锁", 0x31),
    ("上报融合定位包", 0x33),
    ("运维寻车", 0x36),
    ("上报多点定位包", 0x37),
    ("远程打开头盔锁", 0x3E),
    ("远程关闭头盔锁", 0x3F),
    ("强制借车", 0x47),
    ("预还车", 0x54),
    ("远程开启尾箱锁", 0x55),
    ("远程关闭尾箱锁", 0x56),
];

pub const ALARM_TYPES: &[(&str, u8)] = &[
    ("备用电池低电告警", 0x00),
    ("震动报警", 0x01),
    ("非法打开电门锁报警", 0x02),
    ("电子围栏报警", 0x03),
    ("断电报警", 0x05),
    ("超速告警", 0x06),
    ("轮动报警", 0x07),
    ("车辆故障告警", 0x08),
    ("车辆发生侧翻告警", 0x09),
    ("车辆扶正告警通知", 0x0A),
    ("车辆电瓶接入通知", 0x0B),
    ("RFID故障告警", 0x0C),
    ("摄像头故障告警", 0x0D),
    ("后轮锁故障告警", 0x0E),
    ("氢燃料电池低电告警", 0x0F),
    ("摄像头遮挡", 0x10),
    ("获取不到头盔通讯信息", 0x11),
    ("钢缆索故障告警", 0x12),
    ("头盔低电告警", 0x13),
    ("头盔未放置成功", 0x14),
    ("电池高温", 0x20),
];
