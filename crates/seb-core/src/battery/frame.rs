//! CosPower 电池协议的常量、打包与解析。

use crate::config::BatteryPayloadSettings;
use crate::semantic::{Field, Frame, FrameBuilder, BATTERY_HEADER_LEN};
use chrono::{Datelike, Local, Timelike};

pub mod message_type {
    pub const PING: u8 = 0x00;
    pub const LOGIN: u8 = 0x01;
    pub const MONITOR: u8 = 0x02;
    pub const CONTROL: u8 = 0x03;
    pub const LOGOUT: u8 = 0x04;
}

pub mod monitor_subtype {
    pub const LOCATION: u8 = 0x01;
    pub const RUNTIME: u8 = 0x02;
    pub const ALARM: u8 = 0x03;
}

pub mod reply_tag {
    pub const SUCCESS: u8 = 0x01;
    pub const FAIL: u8 = 0x02;
    pub const REPEAT: u8 = 0x03;
    pub const NON_REPLY: u8 = 0xFE;
}
// ==================== 坐标转换 ====================

pub fn parse_coordinates(s: &str) -> Option<(f64, f64)> {
    let (lng, lat) = s.split_once(',')?;
    Some((lng.trim().parse().ok()?, lat.trim().parse().ok()?))
}

pub fn format_lat_lng(coords: &str) -> (String, String) {
    let (lng, lat) = parse_coordinates(coords).unwrap_or((108.375256, 22.767133));
    let lat_prefix = if lat >= 0.0 { 'N' } else { 'S' };
    let lng_prefix = if lng >= 0.0 { 'E' } else { 'W' };
    let lat_val = (lat.abs() * 1_000_000.0).round() as i64;
    let lng_val = (lng.abs() * 1_000_000.0).round() as i64;
    (
        format!("{lat_prefix}{lat_val:09}"),
        format!("{lng_prefix}{lng_val:09}"),
    )
}

// ==================== 协议打包与校验 ====================

fn current_time_bytes() -> [u8; 6] {
    let now = Local::now();
    let year = (now.year() % 100) as u8;
    let month = now.month() as u8;
    let day = now.day() as u8;
    let hour = now.hour() as u8;
    let minute = now.minute() as u8;
    let second = now.second() as u8;
    [year, month, day, hour, minute, second]
}

fn pad_pack_id(pack_id: &str) -> [u8; 16] {
    let mut out = [b' '; 16];
    let bytes = pack_id.as_bytes();
    let len = bytes.len().min(16);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

fn ascii_pad<const N: usize>(s: &str) -> [u8; N] {
    let mut out = [b'0'; N];
    let bytes = s.as_bytes();
    let len = bytes.len().min(N);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

fn builder() -> FrameBuilder {
    FrameBuilder::new(BATTERY_HEADER_LEN)
}

fn put_time(b: &mut FrameBuilder) {
    let t = current_time_bytes();
    let text = format!(
        "20{:02}-{:02}-{:02} {:02}:{:02}:{:02}",
        t[0], t[1], t[2], t[3], t[4], t[5]
    );
    b.put("time", "设备时间", text, &t);
}

fn yes_no(v: bool) -> &'static str {
    if v {
        "是"
    } else {
        "否"
    }
}

/// 告警字节里每个 bit 都是中台的一个字段，字节只写一次，置位的挂成字段
fn put_alarm_bits(b: &mut FrameBuilder, byte: u8, bits: &[(u8, &str, &str)]) {
    let start = b.offset();
    b.raw(&[byte]);
    for (bit, key, label) in bits {
        if byte & (1 << bit) != 0 {
            b.field(*key, *label, "是", start, start + 1);
        }
    }
}

fn message_type_name(t: u8) -> &'static str {
    match t {
        message_type::PING => "心跳",
        message_type::LOGIN => "登录",
        message_type::MONITOR => "监测",
        message_type::CONTROL => "控制",
        message_type::LOGOUT => "登出",
        _ => "未知",
    }
}

fn reply_tag_name(t: u8) -> &'static str {
    match t {
        reply_tag::SUCCESS => "成功",
        reply_tag::FAIL => "失败",
        reply_tag::REPEAT => "重复",
        reply_tag::NON_REPLY => "无需回复",
        _ => "未知",
    }
}

/// 帧头 `FA FB | 报文类型 | 应答标志 | 电池编号(16) | 加密方式 | 内容长度(2)`
/// 和帧尾 `校验和 | FB FA`。校验和是 index 2 到 body 末尾的异或
pub fn envelope_fields(frame: &[u8]) -> (Vec<Field>, Vec<Field>) {
    if frame.len() < BATTERY_HEADER_LEN + 3 {
        return (Vec::new(), Vec::new());
    }
    let declared = u16::from_be_bytes([frame[21], frame[22]]);
    let actual = frame.len() - BATTERY_HEADER_LEN - 3;
    let length = if declared as usize == actual {
        format!("{declared} 字节")
    } else {
        format!("{declared} 字节（实际 {actual}）")
    };
    let head = vec![
        Field::new("startFlag", "起始位", to_hex(&frame[0..2]), 0, 2),
        Field::new(
            "messageType",
            "报文类型",
            format!("{}(0x{:02X})", message_type_name(frame[2]), frame[2]),
            2,
            3,
        ),
        Field::new(
            "replyTag",
            "应答标志",
            format!("{}(0x{:02X})", reply_tag_name(frame[3]), frame[3]),
            3,
            4,
        ),
        Field::new(
            "packId",
            "电池编号",
            String::from_utf8_lossy(&frame[4..20]).trim().to_string(),
            4,
            20,
        ),
        Field::new("encryptType", "加密方式", frame[20].to_string(), 20, 21),
        Field::new("bodyLength", "内容长度", length, 21, BATTERY_HEADER_LEN),
    ];
    let at = frame.len() - 3;
    let got = frame[at];
    let want = frame[2..at].iter().fold(0u8, |acc, b| acc ^ b);
    let checksum = if got == want {
        format!("0x{got:02X} 校验通过")
    } else {
        format!("0x{got:02X} 校验不通过（应为 0x{want:02X}）")
    };
    (
        head,
        vec![
            Field::new("checksum", "校验和", checksum, at, at + 1),
            Field::new("endFlag", "结束位", to_hex(&frame[at + 1..]), at + 1, frame.len()),
        ],
    )
}

/// 所有上行帧都走它成帧，帧头帧尾的语义就不会漏
fn seal(b: FrameBuilder, message_type: u8, reply_tag: u8, pack_id: &str, encrypt: u8) -> Frame {
    b.finish(|body| build_frame(message_type, reply_tag, pack_id, encrypt, body))
        .with_envelope(envelope_fields)
}

fn build_frame(
    message_type: u8,
    reply_tag: u8,
    pack_id: &str,
    encrypt_type: u8,
    body: &[u8],
) -> Vec<u8> {
    let length = body.len() as u16;
    let mut frame = Vec::with_capacity(26 + body.len());

    // 0..2: START (0xFA 0xFB)
    frame.push(0xFA);
    frame.push(0xFB);

    // 2: message_type
    frame.push(message_type);

    // 3: reply_tag
    frame.push(reply_tag);

    // 4..20: pack_id (16B)
    frame.extend_from_slice(&pad_pack_id(pack_id));

    // 20: encrypt_type
    frame.push(encrypt_type);

    // 21..23: length (2B BE)
    frame.extend_from_slice(&length.to_be_bytes());

    // 23..23+len: body
    frame.extend_from_slice(body);

    // Checksum: XOR from index 2 to end of body
    let mut checksum = 0u8;
    for &b in &frame[2..] {
        checksum ^= b;
    }
    frame.push(checksum);

    // END (0xFB 0xFA)
    frame.push(0xFB);
    frame.push(0xFA);

    frame
}

pub fn build_login_frame(
    pack_id: &str,
    iccid: &str,
    hw_maj: u8,
    hw_min: u8,
    hw_rev: u8,
    sw_maj: u8,
    sw_min: u8,
    sw_rev: u8,
) -> Frame {
    let mut b = builder();
    put_time(&mut b);
    b.put("iccId", "ICCID", iccid, &ascii_pad::<20>(iccid));
    b.put(
        "hardwareVersion",
        "硬件版本",
        format!("{hw_maj}.{hw_min}.{hw_rev}"),
        &[hw_maj, hw_min, hw_rev],
    );
    b.put(
        "softwareVersion",
        "软件版本",
        format!("{sw_maj}.{sw_min}.{sw_rev}"),
        &[sw_maj, sw_min, sw_rev],
    );
    seal(b, message_type::LOGIN, reply_tag::NON_REPLY, pack_id, 0x01)
}

pub fn build_location_frame(
    pack_id: &str,
    lat_str: &str,
    lng_str: &str,
    speed_str: &str,
    azimuth_str: &str,
) -> Frame {
    let mut b = builder();
    put_time(&mut b);
    b.raw(&[monitor_subtype::LOCATION]);
    b.put("isValid", "定位有效性", "A 有效", b"A");
    b.put("latitudeStr", "纬度", lat_str, &ascii_pad::<10>(lat_str));
    b.put("longitudeStr", "经度", lng_str, &ascii_pad::<10>(lng_str));
    b.put("speedStr", "速度", speed_str, &ascii_pad::<5>(speed_str));
    b.put("azimuthStr", "方位角", azimuth_str, &ascii_pad::<5>(azimuth_str));
    b.put("gpsCount", "GPS 星数", "0", &[0]);
    b.put("bdCount", "北斗星数", "0", &[0]);
    seal(b, message_type::MONITOR, reply_tag::SUCCESS, pack_id, 0x01)
}

pub fn build_alarm_frame(pack_id: &str, p: &BatteryPayloadSettings) -> Frame {
    let mut b = builder();
    put_time(&mut b);
    b.raw(&[monitor_subtype::ALARM]);

    put_alarm_bits(
        &mut b,
        p.alarm_byte_one,
        &[
            (0, "overVoltage", "过压"),
            (1, "underVoltage", "欠压"),
            (2, "chargeOverTemperature", "充电高温"),
            (3, "disChargeOverTemperature", "放电高温"),
            (4, "heatFilmOverTemperature", "加热膜高温"),
            (5, "disChargeOverCurrentOne", "放电过流一级"),
            (6, "disChargeOverCurrentTwo", "放电过流二级"),
            (7, "disChargeOverCurrentThree", "放电过流三级"),
        ],
    );
    put_alarm_bits(
        &mut b,
        p.alarm_byte_two,
        &[
            (0, "shortCircuit", "短路"),
            (1, "chargeOverCurrent", "充电过流"),
            (2, "heatTimeout", "加热超时"),
            (3, "chargeUnderTemperature", "充电低温"),
            (4, "disChargeUnderTemperature", "放电低温"),
            (5, "mosOverTemperature", "MOS 高温"),
        ],
    );

    let three = p.alarm_byte_three;
    let start = b.offset();
    b.raw(&[three]);
    b.field("heatStatus", "加热状态", (three & 0x03).to_string(), start, start + 1);
    b.field(
        "portOverTemperature",
        "端口高温",
        yes_no(three & 0x08 != 0),
        start,
        start + 1,
    );

    let four = p.alarm_byte_four;
    let start = b.offset();
    b.raw(&[four]);
    b.field("chargeMosStatus", "充电 MOS", yes_no(four & 0x01 != 0), start, start + 1);
    b.field("disChargeMosStatus", "放电 MOS", yes_no(four & 0x02 != 0), start, start + 1);
    b.field("heatFilmMosStatus", "加热膜 MOS", yes_no(four & 0x04 != 0), start, start + 1);

    b.put(
        "faultCode",
        "故障码",
        format!("{:04X}", p.alarm_fault_code),
        &p.alarm_fault_code.to_be_bytes(),
    );

    seal(b, message_type::MONITOR, reply_tag::NON_REPLY, pack_id, 0x01)
}

pub fn build_runtime_frame(pack_id: &str, p: &BatteryPayloadSettings) -> Frame {
    let mut b = builder();
    put_time(&mut b);
    b.raw(&[monitor_subtype::RUNTIME]);

    b.put("totalVoltage", "总电压", p.total_voltage.to_string(), &p.total_voltage.to_be_bytes());
    b.put("current", "电流", p.current.to_string(), &p.current.to_be_bytes());
    b.put("status", "电池状态", p.battery_status.to_string(), &[p.battery_status]);

    let cells = parse_u16_list(&p.cell_voltages);
    b.put("voltageCount", "电芯数量", cells.len().to_string(), &[cells.len() as u8]);
    if !cells.is_empty() {
        let cell_bytes: Vec<u8> = cells.iter().flat_map(|v| v.to_be_bytes()).collect();
        b.put("voltages", "电芯电压", join_nums(&cells), &cell_bytes);
    }

    for (list, count_key, key, label) in [
        (&p.battery_temperatures, "temperatureCount", "temperatures", "电池温度"),
        (&p.heat_film_temperatures, "heatFilmTemperatureCount", "heatFilmTemperatures", "加热膜温度"),
        (&p.environment_temperatures, "environmentTemperatureCount", "environmentTemperatures", "环境温度"),
        (&p.mos_temperatures, "mosTemperatureCount", "mosTemperatures", "MOS 温度"),
    ] {
        let temps = parse_u8_list(list);
        b.put(count_key, format!("{label}数量"), temps.len().to_string(), &[temps.len() as u8]);
        if !temps.is_empty() {
            b.put(key, label, join_nums(&temps), &temps);
        }
    }

    b.put("highestVoltage", "最高单体电压", p.max_cell_voltage.to_string(), &p.max_cell_voltage.to_be_bytes());
    b.put("lowestVoltage", "最低单体电压", p.min_cell_voltage.to_string(), &p.min_cell_voltage.to_be_bytes());
    b.put("averageVoltage", "平均单体电压", p.avg_cell_voltage.to_string(), &p.avg_cell_voltage.to_be_bytes());
    b.put("highestTemperature", "最高电池温度", p.max_battery_temperature.to_string(), &[p.max_battery_temperature]);
    b.put("lowestTemperature", "最低电池温度", p.min_battery_temperature.to_string(), &[p.min_battery_temperature]);
    b.put("totalCapacity", "累计放电容量", p.total_discharge_capacity.to_string(), &p.total_discharge_capacity.to_be_bytes());
    b.put("ratedCapacity", "额定容量", p.rated_capacity.to_string(), &p.rated_capacity.to_be_bytes());
    b.put("remainCapacity", "剩余容量", p.remain_capacity.to_string(), &p.remain_capacity.to_be_bytes());
    b.put("soc", "SOC", format!("{}%", p.soc), &[p.soc]);
    b.put("statusInfo", "状态字节", format!("0x{:02X}", p.status_info), &[p.status_info]);
    b.put("circulateCount", "循环次数", p.cycle_count.to_string(), &p.cycle_count.to_be_bytes());
    b.put("highestVoltageNo", "最高单体编号", p.max_cell_no.to_string(), &[p.max_cell_no]);
    b.put("lowestVoltageNo", "最低单体编号", p.min_cell_no.to_string(), &[p.min_cell_no]);
    b.put("temperatureNo", "温度编号", p.temperature_no.to_string(), &[p.temperature_no]);
    b.put("wakeInfo", "唤醒信息", "0", &[0]);
    b.raw(&[0x00, 0x00]);

    seal(b, message_type::MONITOR, reply_tag::SUCCESS, pack_id, 0x01)
}

fn join_nums<T: std::fmt::Display>(v: &[T]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn parse_u16_list(s: &str) -> Vec<u16> {
    s.split(',').filter_map(|v| v.trim().parse().ok()).collect()
}

fn parse_u8_list(s: &str) -> Vec<u8> {
    s.split(',').filter_map(|v| v.trim().parse().ok()).collect()
}

pub fn build_ping_frame(pack_id: &str) -> Frame {
    let mut b = builder();
    put_time(&mut b);
    seal(b, message_type::PING, reply_tag::NON_REPLY, pack_id, 0x01)
}

pub fn build_logout_frame(pack_id: &str) -> Frame {
    let mut b = builder();
    put_time(&mut b);
    seal(b, message_type::LOGOUT, reply_tag::NON_REPLY, pack_id, 0x01)
}

pub fn build_control_reply(pack_id: &str) -> Frame {
    let mut b = builder();
    put_time(&mut b);
    b.put("command", "控制命令", "1 开关锁", &[0x01]);
    b.put("opCode", "执行结果", "1 成功", &[0x01]);
    seal(b, message_type::CONTROL, reply_tag::SUCCESS, pack_id, 0x01)
}

// ==================== 报文切分与解析 ====================

pub fn split_battery_frames(buf: &[u8]) -> (Vec<Vec<u8>>, usize) {
    let mut frames = Vec::new();
    let mut i = 0;
    while i + 26 <= buf.len() {
        if buf[i] == 0xFA && buf[i + 1] == 0xFB {
            let length = u16::from_be_bytes([buf[i + 21], buf[i + 22]]) as usize;
            let total_len = 26 + length;
            if i + total_len <= buf.len() {
                if buf[i + total_len - 2] == 0xFB && buf[i + total_len - 1] == 0xFA {
                    frames.push(buf[i..i + total_len].to_vec());
                    i += total_len;
                    continue;
                }
            } else {
                break;
            }
        }
        i += 1;
    }
    (frames, i)
}

pub fn parse_battery_frame_summary(frame: &[u8]) -> String {
    if frame.len() < 26 {
        return "未知/不完整报文".to_string();
    }
    let msg_type = frame[2];
    let pack_id = String::from_utf8_lossy(&frame[4..20]).trim().to_string();
    let tag_desc = reply_tag_name(frame[3]);

    match msg_type {
        message_type::PING => format!("心跳响应 [设备: {pack_id}] tag={tag_desc}"),
        message_type::LOGIN => format!("登录应答 [设备: {pack_id}] tag={tag_desc}"),
        message_type::MONITOR => {
            let sub = if frame.len() > 29 { frame[29] } else { 0 };
            let sub_name = match sub {
                monitor_subtype::LOCATION => "位置",
                monitor_subtype::RUNTIME => "遥测",
                monitor_subtype::ALARM => "告警",
                _ => "监测",
            };
            format!("{sub_name}应答 [设备: {pack_id}] tag={tag_desc}")
        }
        message_type::CONTROL => format!("控制下发 [设备: {pack_id}] tag={tag_desc}"),
        message_type::LOGOUT => format!("登出响应 [设备: {pack_id}] tag={tag_desc}"),
        _ => format!("下行报文 0x{msg_type:02X} [设备: {pack_id}] tag={tag_desc}"),
    }
}

/// 下行报文的字段：帧头帧尾 + 报文体。
/// 中台 Protocol.transferTo 里应答只写 6 字节时间就返回，所以应答的报文体只有时间；
/// 平台主动下发的控制在时间后面还带控制命令和执行结果
pub fn parse_battery_frame_fields(frame: &[u8]) -> Vec<Field> {
    let (head, tail) = envelope_fields(frame);
    if head.is_empty() {
        return Vec::new();
    }
    let mut fields = head;
    let body = BATTERY_HEADER_LEN;
    let body_end = frame.len() - 3;
    if body + 6 <= body_end {
        let t = &frame[body..body + 6];
        fields.push(Field::new(
            "time",
            "设备时间",
            format!(
                "20{:02}-{:02}-{:02} {:02}:{:02}:{:02}",
                t[0], t[1], t[2], t[3], t[4], t[5]
            ),
            body,
            body + 6,
        ));
    }
    if body + 8 <= body_end {
        fields.push(Field::new(
            "command",
            "控制命令",
            frame[body + 6].to_string(),
            body + 6,
            body + 7,
        ));
        fields.push(Field::new(
            "opCode",
            "执行结果",
            frame[body + 7].to_string(),
            body + 7,
            body + 8,
        ));
    }
    fields.extend(tail);
    fields
}

pub fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 字段要顺序落在帧体内；同一个字节挂多个 bit 语义时区间会重复，只要不倒退
    fn check_spans(f: &Frame) {
        let mut cursor = 0;
        for field in &f.fields {
            assert!(field.start >= cursor && field.end > field.start, "{} 区间乱序", field.key);
            assert!(field.end <= f.bytes.len(), "{} 越过了帧尾", field.key);
            cursor = field.start;
        }
        // 帧头帧尾也要有语义，光看 hex 排查不动
        assert_eq!(f.fields.first().map(|x| x.key.as_str()), Some("startFlag"));
        assert_eq!(f.fields.last().map(|x| x.key.as_str()), Some("endFlag"));
        let sum = &f.fields[f.fields.len() - 2];
        assert_eq!(sum.key, "checksum");
        assert!(sum.value.contains("校验通过"), "{}", sum.value);
    }

    #[test]
    fn test_build_login_frame() {
        let f = build_login_frame("CMAH030799497009", "89860409081870640660", 2, 1, 3, 3, 2, 1);
        let frame = &f.bytes;
        assert_eq!(frame[0], 0xFA);
        assert_eq!(frame[1], 0xFB);
        assert_eq!(frame[2], message_type::LOGIN);
        assert_eq!(frame[frame.len() - 2], 0xFB);
        assert_eq!(frame[frame.len() - 1], 0xFA);
        assert_eq!(frame.len(), 26 + 32);
        check_spans(&f);

        let (frames, used) = split_battery_frames(frame);
        assert_eq!(frames.len(), 1);
        assert_eq!(used, frame.len());
    }

    #[test]
    fn test_build_location_frame() {
        let (lat_str, lng_str) = format_lat_lng("116.302928,40.054926");
        let f = build_location_frame("CMAH030799497009", &lat_str, &lng_str, "00137", "23971");
        let frame = &f.bytes;
        assert_eq!(frame[0], 0xFA);
        assert_eq!(frame[1], 0xFB);
        assert_eq!(frame[2], message_type::MONITOR);
        assert_eq!(frame[frame.len() - 2], 0xFB);
        assert_eq!(frame[frame.len() - 1], 0xFA);
        assert_eq!(frame.len(), 26 + 40);
        check_spans(&f);
    }

    #[test]
    fn test_build_alarm_and_runtime_frames() {
        let p = BatteryPayloadSettings::default();
        let alarm = build_alarm_frame("CMAH030799497009", &p);
        assert_eq!(alarm.bytes.len(), 26 + 13);
        assert_eq!(alarm.bytes[2], message_type::MONITOR);
        check_spans(&alarm);

        let runtime = build_runtime_frame("CMAH030799497009", &p);
        assert_eq!(runtime.bytes[0], 0xFA);
        assert_eq!(runtime.bytes[1], 0xFB);
        assert_eq!(runtime.bytes[2], message_type::MONITOR);
        check_spans(&runtime);
    }
}
