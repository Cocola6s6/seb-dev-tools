//! 报文字段的中台语义。
//!
//! `key` 一律抄自中台上行 DTO 的字段名（中控 seb-iot-server 的 protocol/upstream、
//! 电池 battery-iot-center 的 cos-power-server/protocol），不要自己起名；
//! 改名前先去那两个工程比对。

use serde::{Deserialize, Serialize};

/// 中控 TBIT 帧头 `AA AA | len(2) | 00 | cmd | 01 00 00 03`
pub const TBIT_HEADER_LEN: usize = 10;
/// 电池帧头 `FA FB | type | tag | packId(16) | encrypt | len(2)`
pub const BATTERY_HEADER_LEN: usize = 23;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    /// 中台字段名，如 "relativeSoc"
    pub key: String,
    /// 中文名，如 "相对 SOC"
    pub label: String,
    pub value: String,
    /// 在整帧里的字节区间 [start, end)
    pub start: usize,
    pub end: usize,
}

impl Field {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        value: impl Into<String>,
        start: usize,
        end: usize,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value: value.into(),
            start,
            end,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Frame {
    pub bytes: Vec<u8>,
    pub fields: Vec<Field>,
}

/// 写字节必须经过它，语义就不可能和构造漂开。
pub struct FrameBuilder {
    header_len: usize,
    body: Vec<u8>,
    fields: Vec<Field>,
}

impl FrameBuilder {
    pub fn new(header_len: usize) -> Self {
        Self {
            header_len,
            body: Vec::new(),
            fields: Vec::new(),
        }
    }

    /// 无语义的填充（保留位、TLV 外壳等）
    pub fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.body.extend_from_slice(bytes);
        self
    }

    pub fn put(
        &mut self,
        key: impl Into<String>,
        label: impl Into<String>,
        value: impl Into<String>,
        bytes: &[u8],
    ) -> &mut Self {
        let start = self.header_len + self.body.len();
        self.body.extend_from_slice(bytes);
        let end = self.header_len + self.body.len();
        self.fields.push(Field::new(key, label, value, start, end));
        self
    }

    /// 下一个字节在整帧里的位置，配合 `field` 给同一段字节挂多个语义
    pub fn offset(&self) -> usize {
        self.header_len + self.body.len()
    }

    /// 一个字节里塞了好几个 bit 语义时，字节用 `raw` 写一次，语义用它逐条挂上去
    pub fn field(
        &mut self,
        key: impl Into<String>,
        label: impl Into<String>,
        value: impl Into<String>,
        start: usize,
        end: usize,
    ) -> &mut Self {
        self.fields.push(Field::new(key, label, value, start, end));
        self
    }

    /// `wrap` 负责补帧头帧尾，字段区间已经按 header_len 算好
    pub fn finish(self, wrap: impl FnOnce(&[u8]) -> Vec<u8>) -> Frame {
        Frame {
            bytes: wrap(&self.body),
            fields: self.fields,
        }
    }
}

/// 设备时间戳统一按本地时区展开，和中台日志里看到的一致
pub fn time_text(unix_time: u32) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_opt(unix_time as i64, 0) {
        chrono::LocalResult::Single(t) => t.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => unix_time.to_string(),
    }
}

/// 中台按 0xFFFF / 0xFF 表示「无效值」，照实标出来比印一串 65535 好认
pub fn opt_u16(v: u16, unit: &str) -> String {
    if v == u16::MAX {
        "无效".into()
    } else {
        format!("{v}{unit}")
    }
}

pub fn opt_u8(v: u8, unit: &str) -> String {
    if v == u8::MAX {
        "无效".into()
    } else {
        format!("{v}{unit}")
    }
}
