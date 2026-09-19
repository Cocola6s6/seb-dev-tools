//! CosPower 电池协议的常量、打包与解析。

use crate::config::BatteryPayloadSettings;
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
    let (lng, lat) = parse_coordinates(coords).unwrap_or((108.38, 22.77));
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

fn pad_iccid(iccid: &str) -> [u8; 20] {
    let mut out = [b'0'; 20];
    let bytes = iccid.as_bytes();
    let len = bytes.len().min(20);
    out[..len].copy_from_slice(&bytes[..len]);
    out
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
) -> Vec<u8> {
    let mut body = Vec::with_capacity(32);
    body.extend_from_slice(&current_time_bytes());
    body.extend_from_slice(&pad_iccid(iccid));
    body.push(hw_maj);
    body.push(hw_min);
    body.push(hw_rev);
    body.push(sw_maj);
    body.push(sw_min);
    body.push(sw_rev);

    build_frame(message_type::LOGIN, reply_tag::NON_REPLY, pack_id, 0x01, &body)
}

pub fn build_location_frame(
    pack_id: &str,
    lat_str: &str,
    lng_str: &str,
    speed_str: &str,
    azimuth_str: &str,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(40);
    body.extend_from_slice(&current_time_bytes());
    body.push(monitor_subtype::LOCATION);
    body.push(b'A'); // 'A' 有效

    let mut lat_buf = [b'0'; 10];
    let b = lat_str.as_bytes();
    lat_buf[..b.len().min(10)].copy_from_slice(&b[..b.len().min(10)]);
    body.extend_from_slice(&lat_buf);

    let mut lng_buf = [b'0'; 10];
    let b = lng_str.as_bytes();
    lng_buf[..b.len().min(10)].copy_from_slice(&b[..b.len().min(10)]);
    body.extend_from_slice(&lng_buf);

    let mut spd_buf = [b'0'; 5];
    let b = speed_str.as_bytes();
    spd_buf[..b.len().min(5)].copy_from_slice(&b[..b.len().min(5)]);
    body.extend_from_slice(&spd_buf);

    let mut az_buf = [b'0'; 5];
    let b = azimuth_str.as_bytes();
    az_buf[..b.len().min(5)].copy_from_slice(&b[..b.len().min(5)]);
    body.extend_from_slice(&az_buf);

    body.push(0); // gpsCount
    body.push(0); // bdCount

    build_frame(message_type::MONITOR, reply_tag::SUCCESS, pack_id, 0x01, &body)
}

pub fn build_alarm_frame(pack_id: &str, p: &BatteryPayloadSettings) -> Vec<u8> {
    let mut body = Vec::with_capacity(13);
    body.extend_from_slice(&current_time_bytes());
    body.push(monitor_subtype::ALARM);

    body.push(p.alarm_byte_one); // 过压/欠压/过流等
    body.push(p.alarm_byte_two); // 短路/超时/低温/MOS过温
    body.push(p.alarm_byte_three); // heatStatus / portOverTemperature
    body.push(p.alarm_byte_four); // chargeMos / disChargeMos / heatFilmMos

    body.extend_from_slice(&p.alarm_fault_code.to_be_bytes());

    build_frame(message_type::MONITOR, reply_tag::NON_REPLY, pack_id, 0x01, &body)
}

pub fn build_runtime_frame(pack_id: &str, p: &BatteryPayloadSettings) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&current_time_bytes());
    body.push(monitor_subtype::RUNTIME);

    body.extend_from_slice(&p.total_voltage.to_be_bytes());
    body.extend_from_slice(&p.current.to_be_bytes());
    body.push(p.battery_status);

    let cells = parse_u16_list(&p.cell_voltages);
    body.push(cells.len() as u8);
    for v in &cells {
        body.extend_from_slice(&v.to_be_bytes());
    }

    for list in [
        &p.battery_temperatures,
        &p.heat_film_temperatures,
        &p.environment_temperatures,
        &p.mos_temperatures,
    ] {
        let temps = parse_u8_list(list);
        body.push(temps.len() as u8);
        body.extend_from_slice(&temps);
    }

    body.extend_from_slice(&p.max_cell_voltage.to_be_bytes());
    body.extend_from_slice(&p.min_cell_voltage.to_be_bytes());
    body.extend_from_slice(&p.avg_cell_voltage.to_be_bytes());

    body.push(p.max_battery_temperature);
    body.push(p.min_battery_temperature);

    body.extend_from_slice(&p.total_discharge_capacity.to_be_bytes());
    body.extend_from_slice(&p.rated_capacity.to_be_bytes());
    body.extend_from_slice(&p.remain_capacity.to_be_bytes());

    body.push(p.soc);
    body.push(p.status_info);

    body.extend_from_slice(&p.cycle_count.to_be_bytes());
    body.push(p.max_cell_no);
    body.push(p.min_cell_no);
    body.push(p.temperature_no);

    // 3 bytes reverse_ext ("000000")
    body.push(0x00);
    body.push(0x00);
    body.push(0x00);

    build_frame(message_type::MONITOR, reply_tag::SUCCESS, pack_id, 0x01, &body)
}

fn parse_u16_list(s: &str) -> Vec<u16> {
    s.split(',').filter_map(|v| v.trim().parse().ok()).collect()
}

fn parse_u8_list(s: &str) -> Vec<u8> {
    s.split(',').filter_map(|v| v.trim().parse().ok()).collect()
}

pub fn build_ping_frame(pack_id: &str) -> Vec<u8> {
    let body = current_time_bytes();
    build_frame(message_type::PING, reply_tag::NON_REPLY, pack_id, 0x01, &body)
}

pub fn build_logout_frame(pack_id: &str) -> Vec<u8> {
    let body = current_time_bytes();
    build_frame(message_type::LOGOUT, reply_tag::NON_REPLY, pack_id, 0x01, &body)
}

pub fn build_control_reply(pack_id: &str) -> Vec<u8> {
    let mut body = Vec::with_capacity(8);
    body.extend_from_slice(&current_time_bytes());
    body.push(0x01); // lockunlock type
    body.push(0x01); // opCode success
    build_frame(message_type::CONTROL, reply_tag::SUCCESS, pack_id, 0x01, &body)
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
    let reply_tag = frame[3];
    let pack_id = String::from_utf8_lossy(&frame[4..20]).trim().to_string();
    let tag_desc = match reply_tag {
        reply_tag::SUCCESS => "成功",
        reply_tag::FAIL => "失败",
        reply_tag::REPEAT => "重复",
        reply_tag::NON_REPLY => "无需回复",
        _ => "未知",
    };

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

pub fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_login_frame() {
        let frame = build_login_frame("CMAH030799497009", "89860409081870640660", 2, 1, 3, 3, 2, 1);
        assert_eq!(frame[0], 0xFA);
        assert_eq!(frame[1], 0xFB);
        assert_eq!(frame[2], message_type::LOGIN);
        assert_eq!(frame[frame.len() - 2], 0xFB);
        assert_eq!(frame[frame.len() - 1], 0xFA);
        assert_eq!(frame.len(), 26 + 32);

        let (frames, used) = split_battery_frames(&frame);
        assert_eq!(frames.len(), 1);
        assert_eq!(used, frame.len());
    }

    #[test]
    fn test_build_location_frame() {
        let (lat_str, lng_str) = format_lat_lng("116.302928,40.054926");
        let frame = build_location_frame("CMAH030799497009", &lat_str, &lng_str, "00137", "23971");
        assert_eq!(frame[0], 0xFA);
        assert_eq!(frame[1], 0xFB);
        assert_eq!(frame[2], message_type::MONITOR);
        assert_eq!(frame[frame.len() - 2], 0xFB);
        assert_eq!(frame[frame.len() - 1], 0xFA);
        assert_eq!(frame.len(), 26 + 40);
    }

    #[test]
    fn test_build_alarm_and_runtime_frames() {
        let p = BatteryPayloadSettings::default();
        let alarm = build_alarm_frame("CMAH030799497009", &p);
        assert_eq!(alarm.len(), 26 + 13);
        assert_eq!(alarm[2], message_type::MONITOR);

        let runtime = build_runtime_frame("CMAH030799497009", &p);
        assert_eq!(runtime[0], 0xFA);
        assert_eq!(runtime[1], 0xFB);
        assert_eq!(runtime[2], message_type::MONITOR);
    }
}
