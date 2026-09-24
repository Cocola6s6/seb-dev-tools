//! 帧结构：`AA AA | len(2) | 00 | cmd(1) | 01 00 00 03 | body | crc16-x25(2)`
//! len = body 长度 + 12；CRC 覆盖去掉帧头 AAAA 之后到 body 末尾。

use crate::config::ClientPayloadSettings;
use crate::semantic::{opt_u16, opt_u8, time_text, Field, Frame, FrameBuilder, TBIT_HEADER_LEN};

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
    pub const BORROW_VEHICLE: u8 = 11;
    pub const PRE_RETURN_VERTICAL_CONDITION: u8 = 12;
    pub const PRE_RETURN_VERTICAL_RESULT: u8 = 13;
    pub const REMOTE_DELAY_COMMAND: u8 = 15;
    pub const GPS: u8 = 16;
    pub const UNIVERSAL_HELMET: u8 = 17;
    pub const CUSTOM_VOICE: u8 = 21;
    pub const TRUNK_LOCK_RESULT: u8 = 32;
    pub const VEHICLE_INFO: u8 = 33;
}

/// 1~18 抄自泰比特协议附录七；21 起是中台自己扩的，设备协议里没有
fn tlv_name(tag: u8) -> String {
    let name = match tag {
        1 => "VEHICLE_STATUS_EX 扩展车辆状态",
        2 => "HDOP_LITTLE_ENDIAN 全球定位系统 HDOP(小端)",
        3 => "COPS COPS",
        4 => "HDOP_BIG_ENDIAN 全球定位系统 HDOP(大端)",
        5 => "摄像头识别结果",
        6 => "RFID_IDENTIFY_RESULT RFID 识别结果",
        7 => "BLUETOOTH_ROAD 蓝牙道钉数据",
        8 => "DEVICE_STATUS 整车状态信息",
        tlv_tag::VERTICAL_PARKING => "VERTICAL_PARKING 垂直停车数据",
        tlv_tag::SMART_HELMET => "SMART_HELMET_RECOGNITION_RESULT 智能通信头盔锁识别结果",
        tlv_tag::BORROW_VEHICLE => "BORROW_VEHICLE 借车操作控制",
        tlv_tag::PRE_RETURN_VERTICAL_CONDITION => "PRE_RETURN_VERTICAL_CONDITION 预还车-还车条件",
        tlv_tag::PRE_RETURN_VERTICAL_RESULT => "PRE_RETURN_VERTICAL_RESULT 预还车结果",
        14 => "BATTERY_NO 电池 SN 编号",
        tlv_tag::REMOTE_DELAY_COMMAND => "远程延时控制指令",
        tlv_tag::GPS => "GPS GPS 信息",
        tlv_tag::UNIVERSAL_HELMET => "UNIVERSAL_HELMET_RECOGNITION_RESULT 通用头盔识别结果",
        18 => "蓝牙头盔识别结果",
        tlv_tag::CUSTOM_VOICE => "CUSTOM_VOICE 定制语音",
        23 => "BMS 电池信息",
        27 => "BMS_CELL_VOLTAGE_STATUS 电芯电压状态",
        tlv_tag::TRUNK_LOCK_RESULT => "TRUNK_LOCK_RESULT 尾箱锁识别结果",
        tlv_tag::VEHICLE_INFO => "VEHICLE_INFO 车辆信息",
        _ => return format!("未知类型 {tag}"),
    };
    format!("{name}({tag})")
}

/// 中台把这些 bit 打在一个 int 里下发，字节区间共用，语义逐位挂
fn push_tlv_bits(
    value: &[u8],
    offset: usize,
    start: usize,
    bits: &[(&str, &str)],
    fields: &mut Vec<Field>,
) {
    let Some(chunk) = value.get(offset..offset + 4) else {
        return;
    };
    let v = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    let (s, e) = (start + offset, start + offset + 4);
    for (i, (key, label)) in bits.iter().enumerate() {
        let on = v >> i & 1 == 1;
        fields.push(Field::new(*key, *label, if on { "是" } else { "否" }, s, e));
    }
}

fn push_tlv_value(tag: u8, value: &[u8], start: usize, fields: &mut Vec<Field>) {
    match tag {
        tlv_tag::BORROW_VEHICLE => {
            push_tlv_bits(
                value,
                0,
                start,
                &[
                    ("openHelmetLock", "借车后打开头盔锁"),
                    ("safetyRiding", "借车后需要安全骑行"),
                    ("checkPassenger", "载人检测"),
                    ("openTrunkLock", "借车后打开尾箱锁"),
                ],
                fields,
            );
            // rideConditions：安全骑行的限制条件
            push_tlv_bits(
                value,
                4,
                start,
                &[
                    ("helmetOpen", "头盔锁打开"),
                    ("helmetTaken", "头盔取出"),
                    ("helmetWorn", "头盔佩戴"),
                    ("trunkLockClose", "尾箱锁关闭"),
                ],
                fields,
            );
        }
        tlv_tag::PRE_RETURN_VERTICAL_CONDITION => {
            push_tlv_bits(
                value,
                0,
                start,
                &[
                    ("helmetLockLatchClosed", "头盔锁锁舌关闭"),
                    ("helmetLockBoltInPlace", "头盔锁锁销在位"),
                    ("helmetNotWorn", "头盔未佩戴"),
                    ("helmetLockMatching", "头盔锁与当前头盔匹配"),
                    ("cameraRecognitionResult", "摄像头识别结果"),
                    ("rfidRecognitionResult", "RFID 识别结果"),
                    ("bluetoothRecognitionResult", "蓝牙道钉识别结果"),
                    ("imuVerticalParkingDetection", "IMU 垂直停车检测"),
                    ("trunkLockClosed", "尾箱锁关闭"),
                    ("gpsInfo", "GPS"),
                    ("vehicleInfo", "车辆信息"),
                ],
                fields,
            );
            push_tlv_bits(
                value,
                4,
                start,
                &[
                    ("closeHelmetLock", "关闭头盔锁"),
                    ("closeTrunkLock", "关闭尾箱锁"),
                ],
                fields,
            );
        }
        tlv_tag::REMOTE_DELAY_COMMAND if value.len() >= 6 => {
            let cmd = value[0];
            let name = crate::CONTROL_TYPES
                .iter()
                .find(|(_, v)| *v as u8 == cmd)
                .map(|(n, _)| format!("{n}(0x{cmd:02x})"))
                .unwrap_or_else(|| format!("0x{cmd:02x}"));
            fields.push(Field::new("remoteCmd", "延时执行的指令", name, start, start + 1));
            let secs = u32::from_be_bytes([value[1], value[2], value[3], value[4]]);
            fields.push(Field::new(
                "delayTime",
                "延时时间",
                format!("{secs} 秒"),
                start + 1,
                start + 5,
            ));
            let kind = if value[5] == 1 { "取消延时" } else { "延时" };
            fields.push(Field::new(
                "delayType",
                "延时指令类型",
                kind,
                start + 5,
                start + 6,
            ));
        }
        tlv_tag::CUSTOM_VOICE if value.len() >= 3 => {
            fields.push(Field::new(
                "voiceId",
                "语音 ID",
                value[0].to_string(),
                start,
                start + 1,
            ));
            fields.push(Field::new(
                "volume",
                "音量",
                value[1].to_string(),
                start + 1,
                start + 2,
            ));
            fields.push(Field::new(
                "linkLength",
                "链接长度",
                value[2].to_string(),
                start + 2,
                start + 3,
            ));
            if value.len() > 3 {
                fields.push(Field::new(
                    "mp3Link",
                    "语音链接",
                    String::from_utf8_lossy(&value[3..]),
                    start + 3,
                    start + value.len(),
                ));
            }
        }
        _ if !value.is_empty() => {
            fields.push(Field::new(
                "value",
                "TLV 内容",
                to_hex(value),
                start,
                start + value.len(),
            ));
        }
        _ => {}
    }
}

/// msgId 长度不固定（协议只要求 30~40 字节），得按各命令的体结构往后切，不能倒着数固定长度
fn msg_id_start(command: u8, frame: &[u8]) -> Option<usize> {
    let end = frame.len() - 2;
    let body = TBIT_HEADER_LEN;
    let start = match command {
        cmd::CONTROL => control_body_end(frame, end),
        cmd::VOICE => body + 1,
        // 参数之间用分号分隔，msgId 里不会有分号
        cmd::QUERY_PARAM | cmd::SET_PARAM => frame[body..end]
            .iter()
            .rposition(|b| *b == b';')
            .map_or(body, |i| body + i + 1),
        _ => return None,
    };
    (start < end).then_some(start)
}

/// TLV 走到头就是 msgId 起点；长度对不上说明包不规整，退回按 34 字节倒数
fn control_body_end(frame: &[u8], end: usize) -> usize {
    let fallback = end.saturating_sub(34);
    let count_at = TBIT_HEADER_LEN + 1;
    if count_at >= end {
        return fallback;
    }
    let mut pos = count_at + 1;
    for _ in 0..frame[count_at] {
        if pos + 2 > end {
            return fallback;
        }
        pos += 2 + frame[pos + 1] as usize;
        if pos > end {
            return fallback;
        }
    }
    pos
}

/// 下发控制体：`控制码(1) | TLV 个数(1) | [类型(1) 长度(1) 值(len)]* | msgId`
fn push_control_tlv(frame: &[u8], msg_id_start: usize, fields: &mut Vec<Field>) {
    let count_at = TBIT_HEADER_LEN + 1;
    if count_at >= msg_id_start {
        return;
    }
    let count = frame[count_at];
    fields.push(Field::new(
        "tlvSize",
        "TLV 个数",
        count.to_string(),
        count_at,
        count_at + 1,
    ));
    let mut pos = count_at + 1;
    for _ in 0..count {
        if pos + 2 > msg_id_start {
            break;
        }
        let (tag, len) = (frame[pos], frame[pos + 1] as usize);
        let value_end = pos + 2 + len;
        if value_end > msg_id_start {
            break;
        }
        fields.push(Field::new("type", "TLV 类型", tlv_name(tag), pos, pos + 2));
        push_tlv_value(tag, &frame[pos + 2..value_end], pos + 2, fields);
        pos = value_end;
    }
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

/// 传输层头 `AA AA | 包长度(2) | 版本号 | 命令码 | 流水号 | 保留(3)` 和传输层尾 CRC。
/// 这几个 key 属于协议传输层，中台上行 DTO 里没有对应字段
pub fn envelope_fields(frame: &[u8]) -> (Vec<Field>, Vec<Field>) {
    if frame.len() < TBIT_HEADER_LEN + 2 {
        return (Vec::new(), Vec::new());
    }
    let declared = u16::from_be_bytes([frame[2], frame[3]]);
    let length = if declared as usize == frame.len() {
        format!("{declared} 字节")
    } else {
        format!("{declared} 字节（实际 {}）", frame.len())
    };
    let version = frame[4];
    let head = vec![
        Field::new("startFlag", "起始位", to_hex(&frame[0..2]), 0, 2),
        Field::new("packetLength", "包长度", length, 2, 4),
        Field::new(
            "version",
            "版本号",
            format!(
                "0x{version:02X} {}",
                if version & 1 == 1 { "加密" } else { "不加密" }
            ),
            4,
            5,
        ),
        Field::new(
            "command",
            "命令码",
            format!("{}(0x{:02X})", cmd_name(frame[5]), frame[5]),
            5,
            6,
        ),
        Field::new("serialNo", "流水号", frame[6].to_string(), 6, 7),
        Field::new("reserved", "保留", to_hex(&frame[7..TBIT_HEADER_LEN]), 7, TBIT_HEADER_LEN),
    ];
    let at = frame.len() - 2;
    let got = u16::from_be_bytes([frame[at], frame[at + 1]]);
    let want = crc16_x25(&frame[2..at]);
    let crc = if got == want {
        format!("0x{got:04X} 校验通过")
    } else {
        format!("0x{got:04X} 校验不通过（应为 0x{want:04X}）")
    };
    (head, vec![Field::new("crc", "错误校验", crc, at, at + 2)])
}

/// 所有上行帧都走它成帧，帧头帧尾的语义就不会漏
fn seal(b: FrameBuilder, command: u8) -> Frame {
    b.finish(|body| wrap(command, body)).with_envelope(envelope_fields)
}

pub const DEFAULT_SOFT_VERSION: &str = "NS_TBIT_WD-219_R.2.0.7_09-4-2023 17:26:09 &T650_G.C.OPENCPU_ZB_WM-007-008-005_GPS_ACC_BLE_ECU_N58_OTA_H-0001_S-0000";

pub fn login(device_no: &str, soft_version: &str, unix_time: u32) -> Frame {
    const IMSI: &str = "123456789011112";
    const IMEI: &str = "123456789011112";
    let mut b = FrameBuilder::new(TBIT_HEADER_LEN);
    b.put(
        "deviceSerialNo",
        "设备序列号",
        device_no,
        &bcd_encode(device_no, 9),
    );
    b.put(
        "deviceTime",
        "设备时间戳",
        time_text(unix_time),
        &unix_time.to_be_bytes(),
    );
    b.put("loginReason", "登录原因", "0 首次登录", &[0x00]);
    b.put("imsi", "IMSI", IMSI, &bcd_encode(IMSI, 15));
    b.put("imei", "IMEI", IMEI, &bcd_encode(IMEI, 15));
    b.raw(&0u16.to_be_bytes()); // retain
    b.put(
        "manufacturerCode",
        "厂商码",
        "00000000",
        &fixed_ascii("00000000", 8),
    );
    b.put(
        "softVersion",
        "软件版本",
        soft_version,
        &fixed_ascii(soft_version, 115),
    );
    seal(b, cmd::LOGIN)
}

pub fn ping(device_no: &str, soc: u8, p: &ClientPayloadSettings) -> Frame {
    let signal = signal_status(p.gps_status, p.satellites, p.backup_battery, p.gsm_signal);
    // 车辆状态位用与 Python 模拟器相同的固定组合，够维持在线即可
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
    let mut b = FrameBuilder::new(TBIT_HEADER_LEN);
    b.put(
        "deviceSerialNo",
        "设备序列号",
        device_no,
        &bcd_encode(device_no, 9),
    );
    b.put(
        "signalStatus",
        "信号状态",
        signal_status_text(signal),
        &signal.to_be_bytes(),
    );
    put_vehicle_status(&mut b, vehicle, false);
    b.put(
        "voltage",
        "电瓶电压",
        format!("{:.1} V", p.voltage as f64 / 10.0),
        &(p.voltage.saturating_mul(100)).to_be_bytes(), // 0.1V -> mV
    );
    b.put("relativeSoc", "相对 SOC", format!("{soc} %"), &[soc]);
    b.put(
        "availableRemainCapacity",
        "可用剩余容量",
        "0",
        &0u16.to_be_bytes(),
    );
    seal(b, cmd::PING)
}

/// 指令应答（命令码 0xAC）：`处理结果(1) + TLV 个数(1, 这里为 0) + 消息 ID(ASCII)`。
pub fn reply(msg_id: &str, success: bool) -> Frame {
    let mut b = FrameBuilder::new(TBIT_HEADER_LEN);
    b.put(
        "handleResult",
        "处理结果",
        handle_result_text(success as u8),
        &[success as u8],
    );
    b.raw(&[0]); // TLV 个数
    b.put("msgId", "消息 ID", msg_id, msg_id.as_bytes());
    seal(b, cmd::REPLY)
}

/// 参数查询/设置的应答：ASCII 的 "KEY=VALUE;" 拼到一起，后面接 34 字节 msgId
pub fn param_reply(command: u8, entries: &[(String, String)], msg_id: &str) -> Frame {
    let mut b = FrameBuilder::new(TBIT_HEADER_LEN);
    for (k, v) in entries {
        let text = format!("{k}={v};");
        b.put("queryResult", k.clone(), v.clone(), text.as_bytes());
    }
    b.put("msgId", "消息 ID", msg_id, msg_id.as_bytes());
    seal(b, command)
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

/// 位含义抄自中台 protocol/SignalStatus.java
fn signal_status_text(v: u16) -> String {
    let gps = match v & 0x03 {
        0 => "未定位过",
        1 => "GPS 实时定位",
        _ => "历史定位有效",
    };
    format!(
        "{gps} / 卫星 {} / 备电 {} / GSM {}",
        (v >> 2) & 0x0F,
        (v >> 6) & 0x0F,
        (v >> 10) & 0x0F
    )
}

/// D0~D15 在中台 protocol/VehicleStatus.java 里每位各是一个字段，两个字节共用同一区间。
/// 心跳包的 D12 是预留位，不代表头盔锁，传 false 跳过。
fn put_vehicle_status(b: &mut FrameBuilder, v: u16, helmet_lock_bit: bool) {
    let (s, e) = (b.offset(), b.offset() + 2);
    b.raw(&v.to_be_bytes());
    let pick = |n: u16, off: &'static str, on: &'static str| if (v >> n) & 1 == 1 { on } else { off };
    let mut put = |key: &str, label: &str, value: &str| {
        b.field(key, label, value, s, e);
    };
    put("borrowReturnStatus", "借还车状态", pick(0, "借车", "还车"));
    put("motionStatus", "运动状态", pick(1, "静止", "运动"));
    put("motorLockStatus", "电机锁", pick(2, "加锁", "解锁"));
    put("accStatus", "ACC", pick(3, "断电", "供电"));
    put(
        "sleepStatus",
        "休眠状态",
        match (v >> 4) & 0x03 {
            0 => "未休眠",
            1 => "半休眠",
            _ => "全休眠",
        },
    );
    put("bluetoothStatus", "蓝牙", pick(6, "未连接", "已连接"));
    put("batteryLockStatus", "电池锁", pick(7, "打开", "加锁"));
    put("ridingStatus", "骑行状态", pick(8, "非骑行", "骑行中"));
    put("externalPowerStatus", "外接电源", pick(9, "不在位", "在位"));
    put("handlebarStatus", "转把", pick(10, "失效", "可使用"));
    put("trunkLockStatus", "尾箱锁", pick(11, "加锁", "解锁"));
    if helmet_lock_bit {
        put("helmetLockStatus", "头盔锁", pick(12, "加锁", "解锁"));
    }
    put(
        "vehicleState",
        "车辆状态",
        match (v >> 13) & 0x03 {
            0 => "借车",
            1 => "还车",
            2 => "撤防",
            _ => "运输模式",
        },
    );
    put("trunkLatchStatus", "尾箱锁插销", pick(15, "加锁", "解锁"));
}

/// ControlReplyProtocol.handleResult
fn handle_result_text(v: u8) -> String {
    match v {
        0x00 => "0 失败",
        0x01 => "1 成功",
        0x02 => "2 运动中",
        0x03 => "3 外接电源不在位",
        _ => return format!("{v}"),
    }
    .into()
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

pub fn location(
    device_no: &str,
    o: &LocationOpts,
    unix_time: u32,
    p: &ClientPayloadSettings,
) -> Frame {
    let (lng, lat) = gcj02_to_wgs84(o.lng, o.lat);
    let signal = signal_status(p.gps_status, p.satellites, p.backup_battery, p.gsm_signal);
    let vehicle = vehicle_status(o);
    let mut b = FrameBuilder::new(TBIT_HEADER_LEN);
    b.put(
        "deviceSerialNo",
        "设备序列号",
        device_no,
        &bcd_encode(device_no, 9),
    );
    b.put(
        "deviceTime",
        "设备时间戳",
        time_text(unix_time),
        &unix_time.to_be_bytes(),
    );
    b.put(
        "latitude",
        "纬度",
        format!("{lat:.6}"),
        &coord_raw(lat).to_be_bytes(),
    );
    b.put(
        "longitude",
        "经度",
        format!("{lng:.6}"),
        &coord_raw(lng).to_be_bytes(),
    );
    b.put(
        "altitude",
        "海拔",
        format!("{} m", p.altitude),
        &p.altitude.to_be_bytes(),
    );
    b.put("direction", "方向角", format!("{}°", p.heading), &[p.heading]);
    b.put(
        "gpsSpeed",
        "GPS 速度",
        format!("{} km/h", p.gps_speed),
        &[p.gps_speed],
    );
    b.put(
        "signalStatus",
        "信号状态",
        signal_status_text(signal),
        &signal.to_be_bytes(),
    );
    b.put("mcc", "移动国家码", opt_u16(p.mcc, ""), &p.mcc.to_be_bytes());
    b.put("mnc", "移动网络码", opt_u16(p.mnc, ""), &p.mnc.to_be_bytes());
    b.put("lac", "位置区码", opt_u16(p.lac, ""), &p.lac.to_be_bytes());
    b.put(
        "cellid",
        "基站编号",
        opt_u16(p.cell_id, ""),
        &p.cell_id.to_be_bytes(),
    );
    b.put("relativeSoc", "相对 SOC", format!("{} %", o.soc), &[o.soc]);
    b.put(
        "availableRemainCapacity",
        "可用剩余容量",
        opt_u16(p.available_capacity, " mAh"),
        &p.available_capacity.to_be_bytes(),
    );
    b.put("soh", "电池健康度", opt_u8(p.soh, " %"), &[p.soh]);
    b.put(
        "cycle",
        "充电循环次数",
        opt_u16(p.charge_cycles, " 次"),
        &p.charge_cycles.to_be_bytes(),
    );
    b.put(
        "cycleCurrent",
        "骑行电流",
        opt_u8(p.ride_current, " A"),
        &[p.ride_current],
    );
    b.put(
        "controlTemp",
        "控制器温度",
        opt_u8(p.controller_temperature, " ℃"),
        &[p.controller_temperature],
    );
    b.put(
        "voltage",
        "电瓶电压",
        format!("{:.1} V", p.voltage as f64 / 10.0),
        &p.voltage.to_be_bytes(),
    );
    b.put(
        "totalMiles",
        "总里程",
        format!("{} m", p.total_mileage),
        &p.total_mileage.to_be_bytes(),
    );
    b.put(
        "singleMiles",
        "单次里程",
        format!("{} m", p.trip_mileage),
        &p.trip_mileage.to_be_bytes(),
    );
    b.put(
        "speed",
        "车速",
        format!("{} km/h", o.speed),
        &o.speed.to_be_bytes(),
    );
    put_vehicle_status(&mut b, vehicle, true);
    b.raw(&[1]); // TLV 个数
    b.put(
        "helmetRecognitionResult",
        "智能头盔",
        format!(
            "{} / {}",
            if o.helmet_lock_unlocked { "锁舌解锁" } else { "锁舌加锁" },
            if o.helmet_present { "头盔在位" } else { "头盔不在位" }
        ),
        &tlv(
            tlv_tag::SMART_HELMET,
            &smart_helmet(o.helmet_lock_unlocked, o.helmet_present),
        ),
    );
    seal(b, cmd::LOCATION)
}

pub fn bms(
    device_no: &str,
    soc: u8,
    battery_no: &str,
    unix_time: u32,
    p: &ClientPayloadSettings,
) -> Frame {
    let cells = parse_u16_list(&p.bms_cell_voltages);
    let mut b = FrameBuilder::new(TBIT_HEADER_LEN);
    b.put(
        "deviceSerialNo",
        "设备序列号",
        device_no,
        &bcd_encode(device_no, 9),
    );
    b.put(
        "deviceTime",
        "设备时间戳",
        time_text(unix_time),
        &unix_time.to_be_bytes(),
    );
    b.put("relativeSoc", "相对 SOC", format!("{soc} %"), &[soc]);
    b.put(
        "availableRemainCapacity",
        "可用剩余容量",
        format!("{} mAh", p.bms_remain_capacity),
        &p.bms_remain_capacity.to_be_bytes(),
    );
    b.put("absoluteSoc", "绝对 SOC", format!("{soc} %"), &[soc]);
    b.put(
        "absoluteCapacity",
        "满充容量",
        format!("{} mAh", p.bms_full_capacity),
        &p.bms_full_capacity.to_be_bytes(),
    );
    b.put("soh", "电池健康度", format!("{} %", p.bms_soh), &[p.bms_soh]);
    b.put(
        "internalTemp",
        "电池内部温度",
        format!("{} ℃", p.bms_temperature),
        &p.bms_temperature.to_be_bytes(),
    );
    b.put(
        "current",
        "电流",
        format!("{} mA", p.bms_current),
        &p.bms_current.to_be_bytes(),
    );
    b.put(
        "voltage",
        "电池电压",
        format!("{:.2} V", p.bms_voltage as f64 / 1000.0),
        &p.bms_voltage.to_be_bytes(),
    );
    b.put(
        "cycle",
        "充电循环次数",
        format!("{} 次", p.bms_cycle_count),
        &p.bms_cycle_count.to_be_bytes(),
    );
    // 报文里没有节数字段，只能按固定 14 节铺满
    let mut cell_bytes = Vec::with_capacity(28);
    for i in 0..14 {
        cell_bytes.extend_from_slice(&cells.get(i).copied().unwrap_or(0).to_be_bytes());
    }
    b.put(
        "batteryVoltages",
        "单体电压 1-14",
        (0..14)
            .map(|i| cells.get(i).copied().unwrap_or(0).to_string())
            .collect::<Vec<_>>()
            .join(","),
        &cell_bytes,
    );
    b.put(
        "chargingInterval",
        "充电间隔",
        p.bms_charge_interval.to_string(),
        &p.bms_charge_interval.to_be_bytes(),
    );
    b.put(
        "maxChargingInterval",
        "最大充电间隔",
        p.bms_max_charge_interval.to_string(),
        &p.bms_max_charge_interval.to_be_bytes(),
    );
    b.put(
        "barCode",
        "电池条码",
        battery_no,
        &fixed_ascii(battery_no, 16),
    );
    b.put(
        "bmsVersion",
        "BMS 版本",
        p.bms_version.to_string(),
        &p.bms_version.to_be_bytes(),
    );
    b.put(
        "batteryManufacturer",
        "电池厂商",
        &p.bms_manufacturer,
        &fixed_ascii(&p.bms_manufacturer, 16),
    );
    seal(b, cmd::BMS)
}

fn parse_u16_list(raw: &str) -> Vec<u16> {
    raw.split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

pub fn alarm(device_no: &str, alarm_type: u8, unix_time: u32) -> Frame {
    let type_text = crate::ALARM_TYPES
        .iter()
        .find(|(_, v)| *v == alarm_type)
        .map(|(name, _)| format!("{name}(0x{alarm_type:02x})"))
        .unwrap_or_else(|| format!("0x{alarm_type:02x}"));
    let mut b = FrameBuilder::new(TBIT_HEADER_LEN);
    b.put(
        "deviceSerialNo",
        "设备序列号",
        device_no,
        &bcd_encode(device_no, 9),
    );
    b.put(
        "deviceTime",
        "设备时间戳",
        time_text(unix_time),
        &unix_time.to_be_bytes(),
    );
    b.put("alarmType", "告警类型", type_text, &[alarm_type]);
    b.put("controlCode", "控制码", "42", &[42]);
    b.put(
        "faults",
        "故障位",
        "固定样例值",
        &[
            0b1110_1001,
            0b0010_0100,
            0b0001_0010,
            0,
            0,
            0,
            0,
            0,
            0b0011_1001,
        ],
    );
    b.put(
        "acceleration",
        "加速度",
        "100",
        &100u16.to_be_bytes(),
    );
    seal(b, cmd::ALARM)
}

/// 预还车应答：网关据这 6 个 TLV 判定头盔 / 尾箱 / 停车姿态
pub fn reply_pre_return(msg_id: &str, o: &PreReturnOpts) -> Frame {
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

    let mut b = FrameBuilder::new(TBIT_HEADER_LEN);
    b.put(
        "handleResult",
        "处理结果",
        handle_result_text(o.success as u8),
        &[o.success as u8],
    );
    b.raw(&[6]); // TLV 个数
    b.put(
        "verticalParking",
        "停车姿态",
        format!("偏转角 {:.1}° / 横滚角 20.1°", o.deflection_angle),
        &tlv(tlv_tag::VERTICAL_PARKING, &vertical_parking),
    );
    b.put(
        "preReturnVerticalResult",
        "预还车检测项",
        "锁舌/佩戴/摄像头/蓝牙/尾箱/GPS/车辆信息",
        &tlv(tlv_tag::PRE_RETURN_VERTICAL_RESULT, &pre_return),
    );
    b.put(
        "gps",
        "定位",
        format!("{lng:.6},{lat:.6}"),
        &tlv(tlv_tag::GPS, &gps),
    );
    b.put(
        "helmetRecognitionResult",
        "智能头盔",
        format!(
            "{} / {}",
            if o.helmet_lock_unlocked { "锁舌解锁" } else { "锁舌加锁" },
            if o.helmet_present { "头盔在位" } else { "头盔不在位" }
        ),
        &tlv(
            tlv_tag::SMART_HELMET,
            &smart_helmet(o.helmet_lock_unlocked, o.helmet_present),
        ),
    );
    b.put(
        "trunkLockResult",
        "尾箱锁",
        if o.trunk_latch { "插销解锁" } else { "插销加锁" },
        &tlv(tlv_tag::TRUNK_LOCK_RESULT, &trunk.to_be_bytes()),
    );
    b.put(
        "vehicleInfo",
        "车辆信息",
        format!("车速 {}", o.speed),
        &tlv(tlv_tag::VEHICLE_INFO, &vehicle_info),
    );
    b.put("msgId", "消息 ID", msg_id, msg_id.as_bytes());
    seal(b, cmd::REPLY)
}

/// 中控上报的是 WGS84，而调试时手上拿到的坐标基本都是高德 GCJ02
pub fn gcj02_to_wgs84(lng: f64, lat: f64) -> (f64, f64) {
    use std::f64::consts::PI;
    const A: f64 = 6378245.0;
    const EE: f64 = 0.006_693_421_622_965_943;

    if !(73.66..135.05).contains(&lng) || !(3.86..53.55).contains(&lat) {
        return (lng, lat);
    }

    fn transform_lat(lng: f64, lat: f64) -> f64 {
        use std::f64::consts::PI;
        let mut ret = -100.0 + 2.0 * lng + 3.0 * lat + 0.2 * lat * lat
            + 0.1 * lng * lat
            + 0.2 * lng.abs().sqrt();
        ret += (20.0 * (6.0 * lng * PI).sin() + 20.0 * (2.0 * lng * PI).sin()) * 2.0 / 3.0;
        ret += (20.0 * (lat * PI).sin() + 40.0 * (lat / 3.0 * PI).sin()) * 2.0 / 3.0;
        ret += (160.0 * (lat / 12.0 * PI).sin() + 320.0 * (lat * PI / 30.0).sin()) * 2.0 / 3.0;
        ret
    }

    fn transform_lng(lng: f64, lat: f64) -> f64 {
        use std::f64::consts::PI;
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
    pub fields: Vec<Field>,
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
    // 下发指令尾部是消息 ID，长度不固定，得按报文体结构切
    let id_at = msg_id_start(command, frame);
    let msg_id = id_at.and_then(|start| {
        std::str::from_utf8(&frame[start..frame.len() - 2])
            .ok()
            .map(|s| s.to_string())
    });
    let control_command = (command == cmd::CONTROL && frame.len() > 10).then(|| frame[10]);
    let params = if let (cmd::QUERY_PARAM | cmd::SET_PARAM, Some(end)) = (command, id_at) {
        std::str::from_utf8(&frame[10..end])
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
    let (head, tail) = envelope_fields(frame);
    let mut fields = head;
    let body = TBIT_HEADER_LEN;
    let body_end = frame.len() - 2;
    match command {
        // 4.3.01（2）服务器应答：登陆结果 1 + 服务器时间 4
        cmd::LOGIN_REPLY if body + 5 <= body_end => {
            let result = match frame[body] {
                0 => "成功",
                1 => "失败",
                2 => "无效",
                3 => "非法",
                _ => "未知",
            };
            fields.push(Field::new("loginResult", "登陆应答", result, body, body + 1));
            let t = u32::from_be_bytes([
                frame[body + 1],
                frame[body + 2],
                frame[body + 3],
                frame[body + 4],
            ]);
            fields.push(Field::new(
                "serverTime",
                "服务器时间",
                time_text(t),
                body + 1,
                body + 5,
            ));
        }
        // 4.3.12（1）平台下发：提示音指令 1 + MsgID
        cmd::VOICE if body < body_end => {
            fields.push(Field::new(
                "voiceCommand",
                "提示音指令",
                frame[body].to_string(),
                body,
                body + 1,
            ));
        }
        _ => {}
    }
    if let Some(c) = control_command {
        let name = crate::CONTROL_TYPES
            .iter()
            .find(|(_, v)| *v as u8 == c)
            .map(|(n, _)| format!("{n}(0x{c:02x})"))
            .unwrap_or_else(|| format!("0x{c:02x}"));
        fields.push(Field::new(
            "commandCode",
            "控制命令",
            name,
            TBIT_HEADER_LEN,
            TBIT_HEADER_LEN + 1,
        ));
        if let Some(end) = id_at {
            push_control_tlv(frame, end, &mut fields);
        }
    }
    if !params.is_empty() {
        let mut start = TBIT_HEADER_LEN;
        for p in &params {
            let end = start + p.len();
            let (key, value) = match p.split_once('=') {
                Some((k, v)) => (k.to_string(), v.to_string()),
                None => (p.clone(), String::new()),
            };
            fields.push(Field::new("param", key, value, start, end));
            start = end + 1; // 分号
        }
    }
    if let (Some(id), Some(start)) = (&msg_id, id_at) {
        fields.push(Field::new("msgId", "消息 ID", id, start, frame.len() - 2));
    }
    fields.extend(tail);
    Some(Parsed {
        command,
        declared_len,
        crc_ok,
        msg_id,
        control_command,
        params,
        fields,
    })
}

pub fn split_frames(buf: &[u8]) -> (Vec<Vec<u8>>, usize) {
    let mut frames = Vec::new();
    let mut pos = 0;
    while let Some(start) = (pos..buf.len().saturating_sub(1)).find(|&i| buf[i] == 0xAA && buf[i + 1] == 0xAA) {
        if buf.len() - start < 4 {
            pos = start;
            break;
        }
        let total = u16::from_be_bytes([buf[start + 2], buf[start + 3]]) as usize;
        if !(12..=8192).contains(&total) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 语义字段必须按顺序落在帧体内，否则高亮和翻译就对不上字节。
    /// 一段字节塞多个 bit 语义时允许重复挂同一区间。
    fn check_spans(f: &Frame) {
        let mut cursor = 0;
        let mut prev = (0, 0);
        for field in &f.fields {
            assert!(
                (field.start >= cursor || (field.start, field.end) == prev)
                    && field.end > field.start,
                "{} 区间乱序: {:?}",
                field.key,
                (field.start, field.end)
            );
            assert!(field.end <= f.bytes.len(), "{} 越过了帧尾", field.key);
            cursor = field.end;
            prev = (field.start, field.end);
        }
        // 帧头帧尾也要有语义，光看 hex 排查不动
        assert_eq!(f.fields.first().map(|x| x.key.as_str()), Some("startFlag"));
        let crc = f.fields.last().expect("帧尾字段");
        assert_eq!(crc.key, "crc");
        assert!(crc.value.contains("校验通过"), "{}", crc.value);
        assert_eq!((crc.start, crc.end), (f.bytes.len() - 2, f.bytes.len()));
    }

    #[test]
    fn frame_fields_stay_inside_body() {
        let p = ClientPayloadSettings::default();
        let o = LocationOpts {
            lng: 108.38,
            lat: 22.77,
            vehicle_state: 1,
            motion: false,
            soc: 80,
            speed: 0,
            helmet_lock_unlocked: false,
            helmet_present: true,
            trunk_latch: true,
            acc_on: true,
        };
        check_spans(&login("865120060012345", DEFAULT_SOFT_VERSION, 1_758_000_000));
        check_spans(&ping("865120060012345", 80, &p));
        check_spans(&location("865120060012345", &o, 1_758_000_000, &p));
        check_spans(&bms("865120060012345", 80, "BAT001", 1_758_000_000, &p));
        check_spans(&alarm("865120060012345", 0x01, 1_758_000_000));
        check_spans(&reply("0123456789012345678901234567890123", true));
    }

    /// 中台实际用 32 字节，协议只规定 30~40，解析不能按固定长度倒数
    const MSG_ID_32: &str = "01234567890123456789012345678901";

    #[test]
    fn 解析借车下发的_tlv() {
        let mut body = vec![0x50, 0x01, tlv_tag::BORROW_VEHICLE, 24];
        body.extend_from_slice(&1u32.to_be_bytes()); // openHelmetLock
        body.extend_from_slice(&2u32.to_be_bytes()); // helmetTaken
        body.extend_from_slice(&[0u8; 16]);
        body.extend_from_slice(MSG_ID_32.as_bytes());
        let parsed = parse(&wrap(cmd::CONTROL, &body)).unwrap();
        let got = |key: &str| {
            parsed
                .fields
                .iter()
                .find(|f| f.key == key)
                .map(|f| f.value.clone())
        };
        assert_eq!(got("tlvSize").as_deref(), Some("1"));
        assert_eq!(got("openHelmetLock").as_deref(), Some("是"));
        assert_eq!(got("openTrunkLock").as_deref(), Some("否"));
        assert_eq!(got("helmetTaken").as_deref(), Some("是"));
        assert_eq!(parsed.msg_id.as_deref(), Some(MSG_ID_32));
    }

    #[test]
    fn 参数下发按分号切出_msg_id() {
        let mut body = b"SVRIP;HBTIME;".to_vec();
        body.extend_from_slice(MSG_ID_32.as_bytes());
        let parsed = parse(&wrap(cmd::QUERY_PARAM, &body)).unwrap();
        assert_eq!(parsed.params, vec!["SVRIP", "HBTIME"]);
        assert_eq!(parsed.msg_id.as_deref(), Some(MSG_ID_32));
    }

    /// 下行报文也要带帧头帧尾，空体的应答至少能看到这两截
    #[test]
    fn 下行报文解析出帧头帧尾() {
        let mut body = vec![0x00];
        body.extend_from_slice(&1_758_000_000u32.to_be_bytes());
        let parsed = parse(&wrap(cmd::LOGIN_REPLY, &body)).unwrap();
        let keys: Vec<&str> = parsed.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "startFlag",
                "packetLength",
                "version",
                "command",
                "serialNo",
                "reserved",
                "loginResult",
                "serverTime",
                "crc",
            ]
        );
        assert!(parsed.crc_ok);
        assert!(parsed.fields.last().unwrap().value.contains("校验通过"));

        // 心跳应答没有报文体，只剩帧头帧尾
        let empty = parse(&wrap(cmd::PING_REPLY, &[])).unwrap();
        assert_eq!(empty.fields.len(), 7);
    }
}
