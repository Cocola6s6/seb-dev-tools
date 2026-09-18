//! 电池客户端模拟器：对接 battery-iot-center 的 CosPower 协议。
//! 精简设计：保留关键参数（环境、电池编号、ICCID、定位坐标），其它默认写死。

use chrono::{Datelike, Local, Timelike};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;

pub const DEFAULT_BATTERY_HOST: &str = "10.12.55.31";
pub const DEFAULT_BATTERY_PORT: u16 = 32402;

pub const DEFAULT_BATTERY_NO: &str = "CMAH030799497009";
pub const DEFAULT_ICCID: &str = "89860409081870640660";
pub const DEFAULT_COORDINATES: &str = "116.302928,40.054926";

const PING_INTERVAL: Duration = Duration::from_secs(60);
const MAX_BUFFERED: usize = 500;

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryFrameLog {
    pub dir: String,
    pub summary: String,
    pub hex: String,
}

impl BatteryFrameLog {
    pub fn sys(summary: impl Into<String>) -> Self {
        Self {
            dir: "sys".into(),
            summary: summary.into(),
            hex: String::new(),
        }
    }
}

/// 电池配置信息：仅保留关键字段
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BatteryConfig {
    pub battery_no: String,
    pub env: String,
    pub host: String,
    pub port: u16,
    pub iccid: String,
    pub coordinates: String,
    pub hw_major_version: u8,
    pub hw_minor_version: u8,
    pub hw_rev_version: u8,
    pub sw_major_version: u8,
    pub sw_minor_version: u8,
    pub sw_rev_version: u8,
    pub heartbeat: bool,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            battery_no: DEFAULT_BATTERY_NO.to_string(),
            env: "内网".to_string(),
            host: DEFAULT_BATTERY_HOST.to_string(),
            port: DEFAULT_BATTERY_PORT,
            iccid: DEFAULT_ICCID.to_string(),
            coordinates: DEFAULT_COORDINATES.to_string(),
            hw_major_version: 2,
            hw_minor_version: 1,
            hw_rev_version: 3,
            sw_major_version: 3,
            sw_minor_version: 2,
            sw_rev_version: 1,
            heartbeat: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryState {
    pub connected: bool,
    pub endpoint: String,
    pub config: BatteryConfig,
}

#[derive(Default)]
struct Shared {
    connected: AtomicBool,
    generation: Mutex<u64>,
    endpoint: Mutex<String>,
    cfg: Mutex<BatteryConfig>,
    buffer: Mutex<Vec<BatteryFrameLog>>,
}

impl Shared {
    fn push(&self, log: BatteryFrameLog) {
        let mut buf = self.buffer.lock().unwrap();
        buf.push(log);
        let len = buf.len();
        if len > MAX_BUFFERED {
            buf.drain(0..len - MAX_BUFFERED);
        }
    }

    fn generation(&self) -> u64 {
        *self.generation.lock().unwrap()
    }

    fn cfg(&self) -> BatteryConfig {
        self.cfg.lock().unwrap().clone()
    }
}

pub struct BatteryLink {
    shared: Arc<Shared>,
    writer: Arc<tokio::sync::Mutex<Option<OwnedWriteHalf>>>,
}

impl BatteryLink {
    pub fn new(cfg: BatteryConfig) -> Self {
        let shared = Arc::new(Shared::default());
        *shared.cfg.lock().unwrap() = cfg;
        Self {
            shared,
            writer: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub fn state(&self) -> BatteryState {
        BatteryState {
            connected: self.shared.connected.load(Ordering::Relaxed),
            endpoint: self.shared.endpoint.lock().unwrap().clone(),
            config: self.shared.cfg(),
        }
    }

    pub fn config(&self) -> BatteryConfig {
        self.shared.cfg()
    }

    pub fn set_config(&self, cfg: BatteryConfig) {
        *self.shared.cfg.lock().unwrap() = cfg;
    }

    pub fn battery_no(&self) -> String {
        self.shared.cfg.lock().unwrap().battery_no.clone()
    }

    pub fn connected(&self) -> bool {
        self.shared.connected.load(Ordering::Relaxed)
    }

    pub fn note(&self, text: impl Into<String>) {
        self.shared.push(BatteryFrameLog::sys(text));
    }

    pub fn drain(&self) -> Vec<BatteryFrameLog> {
        std::mem::take(&mut *self.shared.buffer.lock().unwrap())
    }

    pub async fn connect(&self) -> Result<(), String> {
        let cfg = self.config();
        let (port, heartbeat) = (cfg.port, cfg.heartbeat);
        let host = cfg.host.trim();
        let battery_no = cfg.battery_no.trim();
        if host.is_empty() {
            return Err("电池服务网关地址不能为空".to_string());
        }
        if battery_no.is_empty() {
            return Err("电池编号不能为空".to_string());
        }
        self.disconnect().await;

        let endpoint = format!("{host}:{port}");
        let stream = TcpStream::connect((host, port))
            .await
            .map_err(|e| format!("连接 {endpoint} 失败: {e}"))?;
        stream.set_nodelay(true).ok();
        let (mut read_half, write_half) = stream.into_split();

        let generation = {
            let mut g = self.shared.generation.lock().unwrap();
            *g += 1;
            *g
        };
        *self.shared.endpoint.lock().unwrap() = endpoint.clone();
        self.shared.connected.store(true, Ordering::Relaxed);
        *self.writer.lock().await = Some(write_half);
        self.shared.push(BatteryFrameLog::sys(format!("已连接 {endpoint}")));

        // 连接后自动发送登录包
        self.send_login().await?;

        // 读循环
        {
            let shared = self.shared.clone();
            let writer = self.writer.clone();
            let link_battery_no = battery_no.to_string();
            tokio::spawn(async move {
                let mut pending: Vec<u8> = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    if shared.generation() != generation {
                        return;
                    }
                    match read_half.read(&mut chunk).await {
                        Ok(0) => {
                            if shared.generation() == generation {
                                shared.connected.store(false, Ordering::Relaxed);
                                shared.push(BatteryFrameLog::sys("连接已被服务端关闭"));
                            }
                            return;
                        }
                        Ok(n) => {
                            pending.extend_from_slice(&chunk[..n]);
                            let (frames, used) = split_battery_frames(&pending);
                            pending.drain(0..used);
                            for f in frames {
                                let summary = parse_battery_frame_summary(&f);
                                shared.push(BatteryFrameLog {
                                    dir: "down".into(),
                                    summary,
                                    hex: to_hex(&f),
                                });

                                // 如果是服务端控制命令 (0x03)，自动回复
                                if f.len() >= 26 && f[2] == message_type::CONTROL {
                                    let ack = build_control_reply(&link_battery_no);
                                    let mut guard = writer.lock().await;
                                    if let Some(w) = guard.as_mut() {
                                        if w.write_all(&ack).await.is_ok() {
                                            shared.push(BatteryFrameLog {
                                                dir: "up".into(),
                                                summary: "控制应答 (自动回复)".into(),
                                                hex: to_hex(&ack),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if shared.generation() == generation {
                                shared.connected.store(false, Ordering::Relaxed);
                                shared.push(BatteryFrameLog::sys(format!("读取失败，连接断开: {e}")));
                            }
                            return;
                        }
                    }
                }
            });
        }

        // 心跳循环
        if heartbeat {
            let shared = self.shared.clone();
            let writer = self.writer.clone();
            let link_battery_no = battery_no.to_string();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(PING_INTERVAL).await;
                    if shared.generation() != generation || !shared.connected.load(Ordering::Relaxed) {
                        return;
                    }
                    let data = build_ping_frame(&link_battery_no);
                    let mut guard = writer.lock().await;
                    let Some(w) = guard.as_mut() else { return };
                    if w.write_all(&data).await.is_ok() {
                        shared.push(BatteryFrameLog {
                            dir: "up".into(),
                            summary: "心跳 (Ping)".into(),
                            hex: to_hex(&data),
                        });
                    } else {
                        return;
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn send_login(&self) -> Result<(), String> {
        let cfg = self.config();
        let data = build_login_frame(
            &cfg.battery_no,
            &cfg.iccid,
            cfg.hw_major_version,
            cfg.hw_minor_version,
            cfg.hw_rev_version,
            cfg.sw_major_version,
            cfg.sw_minor_version,
            cfg.sw_rev_version,
        );
        self.send(&data, "电池登录包 (Login)").await
    }

    pub async fn send_location(&self) -> Result<(), String> {
        let cfg = self.config();
        let (lat_str, lng_str) = format_lat_lng(&cfg.coordinates);
        let data = build_location_frame(&cfg.battery_no, &lat_str, &lng_str, "00137", "23971");
        self.send(&data, format!("电池位置上报 ({lat_str}, {lng_str})")).await
    }

    pub async fn send_alarm(&self) -> Result<(), String> {
        let cfg = self.config();
        let data = build_alarm_frame(&cfg.battery_no);
        self.send(&data, "电池告警上报 (Alarm)").await
    }

    pub async fn send_runtime(&self) -> Result<(), String> {
        let cfg = self.config();
        let data = build_runtime_frame(&cfg.battery_no);
        self.send(&data, "电池遥测上报 (Runtime)").await
    }

    pub async fn send_ping(&self) -> Result<(), String> {
        let cfg = self.config();
        let data = build_ping_frame(&cfg.battery_no);
        self.send(&data, "心跳 (Ping)").await
    }

    pub async fn send_logout(&self) -> Result<(), String> {
        let cfg = self.config();
        let data = build_logout_frame(&cfg.battery_no);
        self.send(&data, "电池登出包 (Logout)").await
    }

    pub async fn send(&self, data: &[u8], summary: impl Into<String>) -> Result<(), String> {
        let mut guard = self.writer.lock().await;
        let Some(writer) = guard.as_mut() else {
            return Err("尚未连接电池网关".to_string());
        };
        writer
            .write_all(data)
            .await
            .map_err(|e| format!("发送失败: {e}"))?;
        self.shared.push(BatteryFrameLog {
            dir: "up".into(),
            summary: summary.into(),
            hex: to_hex(data),
        });
        Ok(())
    }

    pub async fn disconnect(&self) {
        let was = self.shared.connected.swap(false, Ordering::Relaxed);
        {
            let mut g = self.shared.generation.lock().unwrap();
            *g += 1;
        }
        if let Some(mut w) = self.writer.lock().await.take() {
            let _ = w.shutdown().await;
        }
        if was {
            self.shared.push(BatteryFrameLog::sys("已断开连接"));
        }
    }
}

// ==================== 坐标转换 ====================

pub fn parse_coordinates(s: &str) -> Option<(f64, f64)> {
    let (lng, lat) = s.split_once(',')?;
    Some((lng.trim().parse().ok()?, lat.trim().parse().ok()?))
}

pub fn format_lat_lng(coords: &str) -> (String, String) {
    let (lng, lat) = parse_coordinates(coords).unwrap_or((116.302928, 40.054926));
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

pub fn build_alarm_frame(pack_id: &str) -> Vec<u8> {
    let mut body = Vec::with_capacity(13);
    body.extend_from_slice(&current_time_bytes());
    body.push(monitor_subtype::ALARM);

    // 默认正常工况告警参数：
    // 各项保护正常 (0)
    // 加热状态: 只加热 (0x01)
    // 充电MOS: 闭合 (0x01), 放电MOS: 断开 (0x00), 加热膜MOS: 闭合 (0x04) -> 0x05
    body.push(0x00); // byteOne (过压/欠压/过流等)
    body.push(0x00); // byteTwo (短路/超时/低温/MOS过温)
    body.push(0x01); // byteThree (heatStatus=1, portOverTemperature=0)
    body.push(0x05); // byteFour (chargeMos=1, disChargeMos=0, heatFilmMos=1)

    // 2 bytes 故障码 ("0000")
    body.push(0x00);
    body.push(0x00);

    build_frame(message_type::MONITOR, reply_tag::NON_REPLY, pack_id, 0x01, &body)
}

pub fn build_runtime_frame(pack_id: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&current_time_bytes());
    body.push(monitor_subtype::RUNTIME);

    body.extend_from_slice(&5500u16.to_be_bytes()); // 电池总电压 5500
    body.extend_from_slice(&2300u16.to_be_bytes()); // 电池电流 2300
    body.push(1); // 电池状态 (1=闭合)

    body.push(3); // 电芯电压数量: 3
    body.extend_from_slice(&3000u16.to_be_bytes()); // voltages[0]
    body.extend_from_slice(&2000u16.to_be_bytes()); // voltages[1]
    body.extend_from_slice(&1000u16.to_be_bytes()); // voltages[2]

    body.push(1); // 电池温度数量: 1
    body.push(75); // temperatures[0] (75°C)

    body.push(1); // 加热膜温度数量: 1
    body.push(65); // heatFilmTemperatures[0] (65°C)

    body.push(1); // 环境温度数量: 1
    body.push(44); // environmentTemperatures[0] (44°C)

    body.push(0); // MOS温度数: 0

    body.extend_from_slice(&3000u16.to_be_bytes()); // 最高单体电压 3000
    body.extend_from_slice(&1000u16.to_be_bytes()); // 最低单体电压 1000
    body.extend_from_slice(&2000u16.to_be_bytes()); // 平均单体电压 2000

    body.push(75); // 最高电池温度 75°C
    body.push(65); // 最低电池温度 65°C

    body.extend_from_slice(&2500u32.to_be_bytes()); // 累计放电容量 2500
    body.extend_from_slice(&800u16.to_be_bytes()); // 额定容量 800
    body.extend_from_slice(&200u16.to_be_bytes()); // 剩余容量 200

    body.push(100); // SOC: 100%
    body.push(0x05); // statusInfo (充电MOS闭合、放电MOS断开、加热MOS断开)

    body.extend_from_slice(&10u16.to_be_bytes()); // 循环次数: 10
    body.push(2); // 最高单体电压编号: 2
    body.push(4); // 最低单体电压编号: 4
    body.push(34); // 最高/最低电池温度编号: 34

    // 3 bytes reverse_ext ("000000")
    body.push(0x00);
    body.push(0x00);
    body.push(0x00);

    build_frame(message_type::MONITOR, reply_tag::SUCCESS, pack_id, 0x01, &body)
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

// ==================== 多电池设备池 ====================

#[derive(Default)]
pub struct BatteryFleet {
    links: Mutex<Vec<Arc<BatteryLink>>>,
}

impl BatteryFleet {
    pub fn seed(&self, configs: Vec<BatteryConfig>) {
        for cfg in configs {
            self.upsert(cfg);
        }
    }

    pub fn upsert(&self, cfg: BatteryConfig) -> Arc<BatteryLink> {
        let mut links = self.links.lock().unwrap();
        match links.iter().find(|l| l.battery_no() == cfg.battery_no) {
            Some(link) => {
                link.set_config(cfg);
                link.clone()
            }
            None => {
                let link = Arc::new(BatteryLink::new(cfg));
                links.push(link.clone());
                link
            }
        }
    }

    pub fn get(&self, battery_no: &str) -> Result<Arc<BatteryLink>, String> {
        self.links
            .lock()
            .unwrap()
            .iter()
            .find(|l| l.battery_no() == battery_no)
            .cloned()
            .ok_or_else(|| format!("电池 {battery_no} 不在模拟清单里"))
    }

    pub fn remove(&self, battery_no: &str) -> Option<Arc<BatteryLink>> {
        let mut links = self.links.lock().unwrap();
        let idx = links.iter().position(|l| l.battery_no() == battery_no)?;
        Some(links.remove(idx))
    }

    pub fn list(&self) -> Vec<Arc<BatteryLink>> {
        self.links.lock().unwrap().clone()
    }

    pub fn states(&self) -> Vec<BatteryState> {
        self.list().iter().map(|l| l.state()).collect()
    }

    pub fn roster(&self) -> Vec<BatteryConfig> {
        self.list().iter().map(|l| l.config()).collect()
    }
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
        let alarm = build_alarm_frame("CMAH030799497009");
        assert_eq!(alarm.len(), 26 + 13);
        assert_eq!(alarm[2], message_type::MONITOR);

        let runtime = build_runtime_frame("CMAH030799497009");
        assert_eq!(runtime[0], 0xFA);
        assert_eq!(runtime[1], 0xFB);
        assert_eq!(runtime[2], message_type::MONITOR);
    }
}
