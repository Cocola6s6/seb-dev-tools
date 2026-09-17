//! 帧结构：`AA AA | len(2) | 00 | cmd(1) | 01 00 00 03 | body | crc16-x25(2)`
//! len = body 长度 + 12；CRC 覆盖去掉帧头 AAAA 之后到 body 末尾。

pub mod cmd {
    pub const LOGIN: u8 = 0x01;
    pub const PING: u8 = 0x05;
    pub const LOCATION: u8 = 0x0C;
    pub const CONTROL: u8 = 0x2C;
    pub const REPLY: u8 = 0xAC;
}

pub fn cmd_name(code: u8) -> &'static str {
    match code {
        cmd::LOGIN => "登录",
        0x02 => "登出",
        cmd::PING => "心跳",
        0x0B => "告警",
        cmd::LOCATION => "定位/上报",
        0x0D => "BMS 电池",
        cmd::CONTROL => "远程控制",
        cmd::REPLY => "指令应答",
        _ => "未知",
    }
}

pub fn bcd_encode(value: &str, length: usize) -> Vec<u8> {
    let digits: Vec<u8> = {
        let s = value.trim();
        let mut v: Vec<u8> = s.bytes().map(|b| b.wrapping_sub(b'0') & 0x0F).collect();
        // 左边补 0 到指定长度；超长时取右侧（与 Python 的 rjust 行为一致）
        if v.len() < length {
            let mut pad = vec![0u8; length - v.len()];
            pad.extend_from_slice(&v);
            v = pad;
        }
        v
    };
    let mut out = Vec::with_capacity(length.div_ceil(2));
    let mut i = 0;
    while i < length {
        let high = digits.get(i).copied().unwrap_or(0) << 4;
        let low = if i >= length - 1 {
            0x0F
        } else {
            digits.get(i + 1).copied().unwrap_or(0)
        };
        out.push(high | low);
        i += 2;
    }
    out
}

pub fn crc16_x25(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0x8408
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

pub fn wrap(command: u8, body: &[u8]) -> Vec<u8> {
    let total = (body.len() + 12) as u16;
    let mut frame = Vec::with_capacity(body.len() + 12);
    frame.extend_from_slice(&[0xAA, 0xAA]);
    frame.extend_from_slice(&total.to_be_bytes());
    frame.extend_from_slice(&[0x00, command, 0x01, 0x00, 0x00, 0x03]);
    frame.extend_from_slice(body);
    let crc = crc16_x25(&frame[2..]);
    frame.extend_from_slice(&crc.to_be_bytes());
    frame
}

pub const DEFAULT_SOFT_VERSION: &str = "NS_TBIT_WD-219_R.2.0.7_09-4-2023 17:26:09 &T650_G.C.OPENCPU_ZB_WM-007-008-005_GPS_ACC_BLE_ECU_N58_OTA_H-0001_S-0000";

pub fn login(device_no: &str, soft_version: &str, unix_time: u32) -> Vec<u8> {
    let mut body = Vec::with_capacity(151);
    body.extend_from_slice(&bcd_encode(device_no, 9));
    body.extend_from_slice(&unix_time.to_be_bytes());
    body.push(0x00); // 登录原因
    body.extend_from_slice(&bcd_encode("123456789011112", 15)); // IMSI
    body.extend_from_slice(&bcd_encode("123456789011112", 15)); // IMEI
    body.extend_from_slice(&0u16.to_be_bytes()); // 保留
    body.extend_from_slice(&fixed_ascii("00000000", 8)); // 厂商码
    body.extend_from_slice(&fixed_ascii(soft_version, 115));
    wrap(cmd::LOGIN, &body)
}

pub fn ping(device_no: &str, soc: u8, voltage_v: f32) -> Vec<u8> {
    // 信号/车辆状态位用与 Python 模拟器相同的固定组合，够维持在线即可
    let signal: u16 =(2 & 0x03) | ((9 & 0x0F) << 2) | ((9 & 0x0F) << 6) | ((9 & 0x0F) << 10);
    let vehicle: u16 = 1
        | (1 << 1)
        | (1 << 2)
        | (1 << 3)
        | (1 << 4)
        | (1 << 6)
        | (1 << 7)
        | (1 << 8)
        | (1 << 9)
        | (1 << 10)
        | (1 << 11)
        | (1 << 12)
        | (3 << 13)
        | (1 << 15);
    let mut body = Vec::with_capacity(14);
    body.extend_from_slice(&bcd_encode(device_no, 9));
    body.extend_from_slice(&signal.to_be_bytes());
    body.extend_from_slice(&vehicle.to_be_bytes());
    body.extend_from_slice(&((voltage_v * 1000.0) as u16).to_be_bytes());
    body.push(soc);
    body.extend_from_slice(&0u16.to_be_bytes()); // 可用剩余容量
    wrap(cmd::PING, &body)
}

/// 指令应答（命令码 0xAC）：`处理结果(1) + TLV 个数(1, 这里为 0) + 消息 ID(ASCII)`。
pub fn reply(msg_id: &str, success: bool) -> Vec<u8> {
    let mut body = Vec::with_capacity(2 + msg_id.len());
    body.push(if success { 1 } else { 0 });
    body.push(0); // TLV 个数
    body.extend_from_slice(msg_id.as_bytes());
    wrap(cmd::REPLY, &body)
}

fn fixed_ascii(text: &str, len: usize) -> Vec<u8> {
    let mut out = text.as_bytes().to_vec();
    out.truncate(len);
    out.resize(len, 0);
    out
}

pub fn from_hex(text: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = text
        .replace("0x", "")
        .replace("0X", "")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '-')
        .collect();
    if cleaned.is_empty() {
        return Err("报文内容不能为空".to_string());
    }
    if cleaned.len() % 2 != 0 {
        return Err(format!("十六进制长度必须是偶数，当前 {} 个字符", cleaned.len()));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|_| format!("非法的十六进制片段: {}", &cleaned[i..i + 2]))
        })
        .collect()
}

pub fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub command: u8,
    pub declared_len: u16,
    pub crc_ok: bool,
    pub msg_id: Option<String>,
    pub control_command: Option<u8>,
}

impl Parsed {
    pub fn summary(&self) -> String {
        let mut text = format!("{}(0x{:02x})", cmd_name(self.command), self.command);
        if let Some(c) = self.control_command {
            if let Some(name) = crate::CONTROL_TYPES.iter().find(|(_, v)| *v as u8 == c) {
                text.push_str(&format!(" {}(0x{:02x})", name.0, c));
            } else {
                text.push_str(&format!(" 控制命令 0x{c:02x}"));
            }
        }
        if !self.crc_ok {
            text.push_str(" [CRC 校验不通过]");
        }
        text
    }
}

pub fn parse(frame: &[u8]) -> Option<Parsed> {
    if frame.len() < 12 || frame[0] != 0xAA || frame[1] != 0xAA {
        return None;
    }
    let declared_len = u16::from_be_bytes([frame[2], frame[3]]);
    let command = frame[5];
    let crc_ok = {
        let body = &frame[2..frame.len() - 2];
        let want = u16::from_be_bytes([frame[frame.len() - 2], frame[frame.len() - 1]]);
        crc16_x25(body) == want
    };
    // 下发指令的消息 ID 固定在 CRC 前的 34 字节
    let msg_id = if matches!(command, cmd::CONTROL | cmd::LOCATION) && frame.len() >= 36 + 12 {
        let start = frame.len() - 2 - 34;
        std::str::from_utf8(&frame[start..frame.len() - 2])
            .ok()
            .map(|s| s.to_string())
    } else {
        None
    };
    let control_command = (command == cmd::CONTROL && frame.len() > 10).then(|| frame[10]);
    Some(Parsed {
        command,
        declared_len,
        crc_ok,
        msg_id,
        control_command,
    })
}

pub fn split_frames(buf: &[u8]) -> (Vec<Vec<u8>>, usize) {
    let mut frames = Vec::new();
    let mut pos = 0;
    loop {
        let Some(start) = (pos..buf.len().saturating_sub(1)).find(|&i| buf[i] == 0xAA && buf[i + 1] == 0xAA) else {
            break;
        };
        if buf.len() - start < 4 {
            pos = start;
            break;
        }
        let total = u16::from_be_bytes([buf[start + 2], buf[start + 3]]) as usize;
        if total < 12 || total > 8192 {
            // 长度不合理，跳过这个疑似帧头继续找
            pos = start + 2;
            continue;
        }
        if buf.len() - start < total {
            pos = start;
            break;
        }
        frames.push(buf[start..start + total].to_vec());
        pos = start + total;
    }
    (frames, pos)
}
