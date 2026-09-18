//! 帧结构：`AA AA | len(2) | 00 | cmd(1) | 01 00 00 03 | body | crc16-x25(2)`
//! len = body 长度 + 12；CRC 覆盖去掉帧头 AAAA 之后到 body 末尾。

pub mod cmd {
    pub const LOGIN: u8 = 0x01;
    pub const LOCATION: u8 = 0x02;
    pub const BMS: u8 = 0x03;
    pub const ALARM: u8 = 0x04;
    pub const PING: u8 = 0x05;
    /// 服务端/网关对设备上报的回复 (Downlink Replies)
    pub const LOGIN_REPLY: u8 = 0x81;
    pub const LOCATION_REPLY: u8 = 0x82;
    pub const BMS_REPLY: u8 = 0x83;
    pub const ALARM_REPLY: u8 = 0x84;
    pub const PING_REPLY: u8 = 0x85;
    /// 远程查询参数，应答用 0x87；设置参数是 0x08/0x88
    pub const QUERY_PARAM: u8 = 0x07;
    pub const SET_PARAM: u8 = 0x08;
    pub const QUERY_PARAM_REPLY: u8 = 0x87;
    pub const SET_PARAM_REPLY: u8 = 0x88;
    /// 语音播报，和 CONTROL 一样带 34 字节 msgId
    pub const VOICE: u8 = 0x0C;
    pub const VOICE_REPLY: u8 = 0x8C;
    pub const CONTROL: u8 = 0x2C;
    pub const REPLY: u8 = 0xAC;
    pub const CONTROL_EXPAND_REPLY2: u8 = 0xBC;
}

/// 预还车：唯一需要回带 TLV 的完整应答的下发指令
pub const CONTROL_PRE_RETURN: u8 = 0x54;

pub mod tlv_tag {
    pub const VERTICAL_PARKING: u8 = 9;
    pub const SMART_HELMET: u8 = 10;
    pub const PRE_RETURN_VERTICAL_RESULT: u8 = 13;
    pub const GPS: u8 = 16;
    pub const TRUNK_LOCK_RESULT: u8 = 32;
    pub const VEHICLE_INFO: u8 = 33;
}

pub fn cmd_name(code: u8) -> &'static str {
    match code {
        cmd::LOGIN => "登录",
        cmd::LOCATION => "定位上报",
        cmd::BMS => "BMS 电池",
        cmd::ALARM => "告警",
        cmd::PING => "心跳",
        cmd::LOGIN_REPLY => "登录回复",
        cmd::LOCATION_REPLY => "定位回复",
        cmd::BMS_REPLY => "BMS 回复",
        cmd::ALARM_REPLY => "告警回复",
        cmd::PING_REPLY => "心跳回复",
        cmd::QUERY_PARAM => "查询参数",
        cmd::SET_PARAM => "设置参数",
        cmd::QUERY_PARAM_REPLY => "查询参数应答",
        cmd::SET_PARAM_REPLY => "设置参数应答",
        cmd::VOICE => "语音播报",
        cmd::VOICE_REPLY => "语音回复",
        cmd::CONTROL => "远程控制",
        cmd::REPLY => "指令应答",
        cmd::CONTROL_EXPAND_REPLY2 => "指令二次应答",
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

/// 参数查询/设置的应答：ASCII 的 "KEY=VALUE;" 拼到一起，后面接 34 字节 msgId
pub fn param_reply(command: u8, entries: &[(String, String)], msg_id: &str) -> Vec<u8> {
    let mut body = String::new();
    for (k, v) in entries {
        body.push_str(&format!("{k}={v};"));
    }
    body.push_str(msg_id);
    wrap(command, body.as_bytes())
}

// 上行业务包的字段顺序与默认值对齐 Python 模拟器 (seb-iot/python/iot/tbit)，改动前先比对那边。

#[derive(Clone, Debug, PartialEq)]
pub struct LocationOpts {
    pub lng: f64,
    pub lat: f64,
    /// 0-借车 1-还车 2-撤防 3-运输模式
    pub vehicle_state: u8,
    pub motion: bool,
    pub soc: u8,
    pub speed: u16,
    /// false 加锁 / true 解锁
    pub helmet_lock_unlocked: bool,
    /// true 在位（未取出）
    pub helmet_present: bool,
    pub trunk_latch: bool,
    pub acc_on: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreReturnOpts {
    pub success: bool,
    pub deflection_angle: f64,
    pub lng: f64,
    pub lat: f64,
    pub helmet_present: bool,
    pub trunk_latch: bool,
    pub speed: u32,
    pub helmet_lock_unlocked: bool,
}

fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + value.len());
    out.push(tag);
    out.push(value.len() as u8);
    out.extend_from_slice(value);
    out
}

/// 经纬度按 1/30000 分存放，即 度 × 60 × 30000。
pub fn coord_raw(deg: f64) -> u32 {
    (deg * 1_800_000.0) as u32
}

fn signal_status(gps: u16, satellites: u16, battery: u16, gsm: u16) -> u16 {
    (gps & 0x03) | ((satellites & 0x0F) << 2) | ((battery & 0x0F) << 6) | ((gsm & 0x0F) << 10)
}

fn vehicle_status(o: &LocationOpts) -> u16 {
    let b = |v: bool| v as u16;
    let state = o.vehicle_state as u16 & 0x03;
    (state % 2)
        | (b(o.motion) << 1)
        // 电机锁 2 / 休眠 4 固定为 0，蓝牙…尾箱锁 6..11 固定为 1，与模拟器一致
        | (b(o.acc_on) << 3)
        | (1 << 6)
        | (1 << 7)
        | (1 << 8)
        | (1 << 9)
        | (1 << 10)
        | (1 << 11)
        | (b(o.helmet_lock_unlocked) << 12)
        | (state << 13)
        | (b(o.trunk_latch) << 15)
}

fn smart_helmet(helmet_lock_unlocked: bool, helmet_present: bool) -> Vec<u8> {
    let result =
        (helmet_lock_unlocked as u8) | ((helmet_present as u8) << 1) | (1 << 2) | (1 << 5) | (1 << 6);
    let mut v = Vec::with_capacity(28);
    v.push(result);
    v.extend_from_slice(&[0u8; 16]); // 4 个保留 u32
    v.extend_from_slice(&[0u8; 3]); // 3 个故障位
    v.extend_from_slice(&100u16.to_be_bytes()); // 电压
    v.extend_from_slice(b"ABCDEF"); // tkId
    v
}

pub fn location(device_no: &str, o: &LocationOpts, unix_time: u32) -> Vec<u8> {
    let (lng, lat) = gcj02_to_wgs84(o.lng, o.lat);
    let mut body = Vec::with_capacity(96);
    body.extend_from_slice(&bcd_encode(device_no, 9));
    body.extend_from_slice(&unix_time.to_be_bytes());
    body.extend_from_slice(&coord_raw(lat).to_be_bytes());
    body.extend_from_slice(&coord_raw(lng).to_be_bytes());
    body.extend_from_slice(&90u16.to_be_bytes()); // 海拔
    body.push(25); // 方向
    body.push(20); // GPS 速度
    body.extend_from_slice(&signal_status(2, 9, 9, 9).to_be_bytes());
    for _ in 0..4 {
        body.extend_from_slice(&u16::MAX.to_be_bytes()); // mcc / mnc / lac / cellId
    }
    body.push(o.soc);
    body.extend_from_slice(&u16::MAX.to_be_bytes()); // 可用剩余容量
    body.push(90); // SOH
    body.extend_from_slice(&u16::MAX.to_be_bytes()); // 循环次数
    body.push(255); // 本次循环电流
    body.push(255); // 控制器温度
    body.extend_from_slice(&100u16.to_be_bytes()); // 电压 0.1V
    body.extend_from_slice(&10000u32.to_be_bytes()); // 总里程
    body.extend_from_slice(&5000u32.to_be_bytes()); // 单次里程
    body.extend_from_slice(&o.speed.to_be_bytes());
    body.extend_from_slice(&vehicle_status(o).to_be_bytes());
    body.push(1); // TLV 个数
    body.extend_from_slice(&tlv(
        tlv_tag::SMART_HELMET,
        &smart_helmet(o.helmet_lock_unlocked, o.helmet_present),
    ));
    wrap(cmd::LOCATION, &body)
}

pub fn bms(device_no: &str, soc: u8, battery_no: &str, unix_time: u32) -> Vec<u8> {
    let mut body = Vec::with_capacity(96);
    body.extend_from_slice(&bcd_encode(device_no, 9));
    body.extend_from_slice(&unix_time.to_be_bytes());
    body.push(soc); // 相对 SOC
    body.extend_from_slice(&4000u16.to_be_bytes()); // 可用剩余容量
    body.push(soc); // 绝对 SOC
    body.extend_from_slice(&5000u16.to_be_bytes()); // 绝对容量
    body.push(85); // SOH
    body.extend_from_slice(&20u16.to_be_bytes()); // 内部温度
    body.extend_from_slice(&90u16.to_be_bytes()); // 电流
    body.extend_from_slice(&18500u16.to_be_bytes()); // 电压 mV
    body.extend_from_slice(&40u16.to_be_bytes()); // 循环次数
    for i in 0..14u16 {
        body.extend_from_slice(&(4000 + i * 100).to_be_bytes()); // 单体电压
    }
    body.extend_from_slice(&u16::MAX.to_be_bytes()); // 充电间隔
    body.extend_from_slice(&u16::MAX.to_be_bytes()); // 最大充电间隔
    body.extend_from_slice(&fixed_ascii(battery_no, 16)); // 电池条码
    body.extend_from_slice(&u16::MAX.to_be_bytes()); // BMS 版本
    body.extend_from_slice(&fixed_ascii("1111111111111111", 16)); // 电池厂商
    wrap(cmd::BMS, &body)
}

pub fn alarm(device_no: &str, alarm_type: u8, unix_time: u32) -> Vec<u8> {
    let mut body = Vec::with_capacity(24);
    body.extend_from_slice(&bcd_encode(device_no, 9));
    body.extend_from_slice(&unix_time.to_be_bytes());
    body.push(alarm_type);
    body.push(42); // 控制码
    body.extend_from_slice(&[0b1110_1001, 0b0010_0100, 0b0001_0010, 0]); // 故障位 0..4
    body.extend_from_slice(&0u16.to_be_bytes()); // 故障位 4（u16）
    body.extend_from_slice(&[0, 0, 0b0011_1001]); // 故障位 5..8
    body.extend_from_slice(&100u16.to_be_bytes()); // 加速度
    wrap(cmd::ALARM, &body)
}

/// 预还车应答：网关据这 6 个 TLV 判定头盔 / 尾箱 / 停车姿态
pub fn reply_pre_return(msg_id: &str, o: &PreReturnOpts) -> Vec<u8> {
    let (lng, lat) = gcj02_to_wgs84(o.lng, o.lat);

    let mut vertical_parking = Vec::with_capacity(4);
    vertical_parking.extend_from_slice(&((o.deflection_angle * 10.0) as u16).to_be_bytes());
    vertical_parking.extend_from_slice(&201u16.to_be_bytes()); // 横滚角 20.1

    // 支持项位图：头盔锁舌/佩戴/摄像头/蓝牙/尾箱/GPS/车辆信息检测
    let support: u32 = (1 << 0) | (1 << 2) | (1 << 4) | (1 << 6) | (1 << 8) | (1 << 9) | (1 << 10);
    let mut pre_return = Vec::with_capacity(20);
    pre_return.extend_from_slice(&support.to_be_bytes());
    pre_return.extend_from_slice(&[0u8; 16]);

    let mut gps = Vec::with_capacity(14);
    gps.extend_from_slice(&coord_raw(lat).to_be_bytes());
    gps.extend_from_slice(&coord_raw(lng).to_be_bytes());
    gps.extend_from_slice(&100u16.to_be_bytes()); // 海拔
    gps.push(45); // 方向
    gps.push(0); // GPS 速度
    gps.extend_from_slice(&signal_status(3, 9, 9, 9).to_be_bytes());

    // 尾箱：逻辑状态 1 / 开关 0 / 在位
    let trunk: u32 = 1 | ((o.trunk_latch as u32) << 2);

    let mut vehicle_info = Vec::with_capacity(20);
    vehicle_info.extend_from_slice(&o.speed.to_be_bytes());
    vehicle_info.extend_from_slice(&[0u8; 16]);

    let mut body = Vec::with_capacity(160);
    body.push(o.success as u8);
    body.push(6); // TLV 个数
    body.extend_from_slice(&tlv(tlv_tag::VERTICAL_PARKING, &vertical_parking));
    body.extend_from_slice(&tlv(tlv_tag::PRE_RETURN_VERTICAL_RESULT, &pre_return));
    body.extend_from_slice(&tlv(tlv_tag::GPS, &gps));
    body.extend_from_slice(&tlv(
        tlv_tag::SMART_HELMET,
        &smart_helmet(o.helmet_lock_unlocked, o.helmet_present),
    ));
    body.extend_from_slice(&tlv(tlv_tag::TRUNK_LOCK_RESULT, &trunk.to_be_bytes()));
    body.extend_from_slice(&tlv(tlv_tag::VEHICLE_INFO, &vehicle_info));
    body.extend_from_slice(msg_id.as_bytes());
    wrap(cmd::REPLY, &body)
}

/// 中控上报的是 WGS84，而调试时手上拿到的坐标基本都是高德 GCJ02
pub fn gcj02_to_wgs84(lng: f64, lat: f64) -> (f64, f64) {
    const PI: f64 = 3.1415926535897932384626;
    const A: f64 = 6378245.0;
    const EE: f64 = 0.00669342162296594323;

    if !(73.66..135.05).contains(&lng) || !(3.86..53.55).contains(&lat) {
        return (lng, lat);
    }

    fn transform_lat(lng: f64, lat: f64) -> f64 {
        const PI: f64 = 3.1415926535897932384626;
        let mut ret = -100.0 + 2.0 * lng + 3.0 * lat + 0.2 * lat * lat
            + 0.1 * lng * lat
            + 0.2 * lng.abs().sqrt();
        ret += (20.0 * (6.0 * lng * PI).sin() + 20.0 * (2.0 * lng * PI).sin()) * 2.0 / 3.0;
        ret += (20.0 * (lat * PI).sin() + 40.0 * (lat / 3.0 * PI).sin()) * 2.0 / 3.0;
        ret += (160.0 * (lat / 12.0 * PI).sin() + 320.0 * (lat * PI / 30.0).sin()) * 2.0 / 3.0;
        ret
    }

    fn transform_lng(lng: f64, lat: f64) -> f64 {
        const PI: f64 = 3.1415926535897932384626;
        let mut ret =
            300.0 + lng + 2.0 * lat + 0.1 * lng * lng + 0.1 * lng * lat + 0.1 * lng.abs().sqrt();
        ret += (20.0 * (6.0 * lng * PI).sin() + 20.0 * (2.0 * lng * PI).sin()) * 2.0 / 3.0;
        ret += (20.0 * (lng * PI).sin() + 40.0 * (lng / 3.0 * PI).sin()) * 2.0 / 3.0;
        ret += (150.0 * (lng / 12.0 * PI).sin() + 300.0 * (lng / 30.0 * PI).sin()) * 2.0 / 3.0;
        ret
    }

    let dlat = transform_lat(lng - 105.0, lat - 35.0);
    let dlng = transform_lng(lng - 105.0, lat - 35.0);
    let radlat = lat / 180.0 * PI;
    let magic = 1.0 - EE * radlat.sin() * radlat.sin();
    let sqrtmagic = magic.sqrt();
    let dlat = (dlat * 180.0) / ((A * (1.0 - EE)) / (magic * sqrtmagic) * PI);
    let dlng = (dlng * 180.0) / (A / sqrtmagic * radlat.cos() * PI);
    (lng * 2.0 - (lng + dlng), lat * 2.0 - (lat + dlat))
}

/// 解析 "经度,纬度" 文本。
pub fn parse_coordinates(text: &str) -> Result<(f64, f64), String> {
    let mut parts = text.split(',');
    let lng = parts
        .next()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .ok_or_else(|| format!("坐标格式应为「经度,纬度」，当前: {text}"))?;
    let lat = parts
        .next()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .ok_or_else(|| format!("坐标格式应为「经度,纬度」，当前: {text}"))?;
    Ok((lng, lat))
}

fn fixed_ascii(text: &str, len: usize) -> Vec<u8> {
    let mut out = text.as_bytes().to_vec();
    out.truncate(len);
    out.resize(len, 0);
    out
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
    /// 参数查询是 ["KEY"]，参数设置是 ["KEY=VALUE"]
    pub params: Vec<String>,
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
        if !self.params.is_empty() {
            text.push(' ');
            text.push_str(&self.params.join(","));
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
    let has_msg_id = matches!(
        command,
        cmd::CONTROL | cmd::VOICE | cmd::QUERY_PARAM | cmd::SET_PARAM
    );
    let msg_id = if has_msg_id && frame.len() >= 36 + 12 {
        let start = frame.len() - 2 - 34;
        std::str::from_utf8(&frame[start..frame.len() - 2])
            .ok()
            .map(|s| s.to_string())
    } else {
        None
    };
    let control_command = (command == cmd::CONTROL && frame.len() > 10).then(|| frame[10]);
    let params = if matches!(command, cmd::QUERY_PARAM | cmd::SET_PARAM) && msg_id.is_some() {
        std::str::from_utf8(&frame[10..frame.len() - 2 - 34])
            .map(|s| {
                s.split(';')
                    .filter(|p| !p.is_empty())
                    .map(|p| p.to_string())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    Some(Parsed {
        command,
        declared_len,
        crc_ok,
        msg_id,
        control_command,
        params,
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
