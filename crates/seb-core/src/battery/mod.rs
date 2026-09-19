//! 电池客户端模拟器：对接 battery-iot-center 的 CosPower 协议。
//! 精简设计：保留关键参数（环境、电池编号、ICCID、定位坐标），其它默认写死。

mod frame;

pub use frame::*;

use crate::config::BatteryPayloadSettings;
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
pub const DEFAULT_COORDINATES: &str = "108.38,22.77";

const DEFAULT_PING_SECS: u64 = 60;
const MAX_BUFFERED: usize = 500;


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
    pub host: String,
    pub port: u16,
    pub iccid: String,
    pub coordinates: String,
    pub heartbeat: bool,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            battery_no: DEFAULT_BATTERY_NO.to_string(),
            host: DEFAULT_BATTERY_HOST.to_string(),
            port: DEFAULT_BATTERY_PORT,
            iccid: DEFAULT_ICCID.to_string(),
            coordinates: DEFAULT_COORDINATES.to_string(),
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

    /// 改配置不影响已经建立的链路，返回值表示服务端地址是否变了
    pub fn set_config(&self, cfg: BatteryConfig) -> bool {
        let mut cur = self.shared.cfg.lock().unwrap();
        let moved = cur.host != cfg.host || cur.port != cfg.port;
        *cur = cfg;
        moved
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

        self.send_login().await?;

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

        if heartbeat {
            let interval = ping_interval();
            let shared = self.shared.clone();
            let writer = self.writer.clone();
            let link_battery_no = battery_no.to_string();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(interval).await;
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
        let settings = crate::config::load().settings.battery;
        let (hw_major, hw_minor, hw_rev) = version_triplet(&settings.default_hw_version, (2, 1, 3));
        let (sw_major, sw_minor, sw_rev) = version_triplet(&settings.default_sw_version, (3, 2, 1));
        let data = build_login_frame(
            &cfg.battery_no,
            &cfg.iccid,
            hw_major,
            hw_minor,
            hw_rev,
            sw_major,
            sw_minor,
            sw_rev,
        );
        self.send(&data, "电池登录包 (Login)").await
    }

    pub async fn send_location(&self) -> Result<(), String> {
        let cfg = self.config();
        let p = payload_settings();
        let (lat_str, lng_str) = format_lat_lng(&cfg.coordinates);
        let data = build_location_frame(
            &cfg.battery_no,
            &lat_str,
            &lng_str,
            &format!("{:05}", p.speed),
            &format!("{:05}", p.azimuth),
        );
        self.send(&data, format!("电池位置上报 ({lat_str}, {lng_str})")).await
    }

    pub async fn send_alarm(&self) -> Result<(), String> {
        let cfg = self.config();
        let data = build_alarm_frame(&cfg.battery_no, &payload_settings());
        self.send(&data, "电池告警上报 (Alarm)").await
    }

    pub async fn send_runtime(&self) -> Result<(), String> {
        let cfg = self.config();
        let data = build_runtime_frame(&cfg.battery_no, &payload_settings());
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

fn payload_settings() -> BatteryPayloadSettings {
    crate::config::load().settings.battery.payload
}

/// "2.1.3" 拆成三段，缺位或非法时回落到内置值
fn version_triplet(raw: &str, fallback: (u8, u8, u8)) -> (u8, u8, u8) {
    let mut it = raw.split('.').map(|s| s.trim().parse::<u8>());
    match (it.next(), it.next(), it.next()) {
        (Some(Ok(a)), Some(Ok(b)), Some(Ok(c))) => (a, b, c),
        _ => fallback,
    }
}

fn ping_interval() -> Duration {
    let secs = crate::config::load().settings.battery.default_heartbeat_interval;
    Duration::from_secs(if secs == 0 { DEFAULT_PING_SECS } else { secs })
}
// ==================== 多电池设备池 ====================

#[derive(Default)]
pub struct BatteryFleet {
    links: Mutex<Vec<Arc<BatteryLink>>>,
}

impl BatteryFleet {
    /// upsert 会把新电池插到队首，倒着喂才能让落盘顺序原样恢复
    pub fn seed(&self, configs: Vec<BatteryConfig>) {
        for cfg in configs.into_iter().rev() {
            self.upsert(cfg);
        }
    }

    /// 返回值：服务端地址是否变了，变了说明换了环境，旧链路得断开
    pub fn upsert(&self, cfg: BatteryConfig) -> (Arc<BatteryLink>, bool) {
        let mut links = self.links.lock().unwrap();
        match links.iter().find(|l| l.battery_no() == cfg.battery_no) {
            Some(link) => {
                let moved = link.set_config(cfg);
                (link.clone(), moved)
            }
            None => {
                let link = Arc::new(BatteryLink::new(cfg));
                links.insert(0, link.clone());
                (link, false)
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

