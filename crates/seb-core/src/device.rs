//! 工具冒充一台 ECU 主动连上 IoT 网关：设备在运营商 NAT 后面，外部无法反向连它，
//! 想收发原始报文只能自己连上去。

use crate::frame;
use crate::semantic::{Field, Frame};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;

use serde::{Deserialize, Serialize};

pub const DEFAULT_GATEWAY_HOST: &str = "bike-seb-inner-test.costrip.cn";
pub const DEFAULT_GATEWAY_PORT: u16 = 32405;

/// ECU 序列号标准为 9 位数字字符，不够 9 位时左侧自动补 0（如 7 位补 00 前缀）
pub fn normalize_ecu_no(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.len() < 9 {
        format!("{:0>9}", trimmed)
    } else {
        trimmed.to_string()
    }
}

const DEFAULT_PING_SECS: u64 = 60;
const MAX_BUFFERED: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrameLog {
    pub dir: String,
    pub summary: String,
    pub hex: String,
    pub fields: Vec<Field>,
}

impl FrameLog {
    fn sys(summary: impl Into<String>) -> Self {
        Self {
            dir: "sys".into(),
            summary: summary.into(),
            hex: String::new(),
            fields: Vec::new(),
        }
    }

    fn up(summary: impl Into<String>, f: &Frame) -> Self {
        Self {
            dir: "up".into(),
            summary: summary.into(),
            hex: frame::to_hex(&f.bytes),
            fields: f.fields.clone(),
        }
    }
}

/// 模拟车辆的当前姿态：定位上报、预还车应答都从这里取值。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SimProfile {
    /// 高德 GCJ02 的 "经度,纬度"
    pub coordinates: String,
    /// 0-借车 1-还车 2-撤防 3-运输模式
    pub vehicle_state: u8,
    pub motion: bool,
    pub soc: u8,
    pub speed: u16,
    pub helmet_lock_unlocked: bool,
    pub helmet_present: bool,
    pub trunk_latch: bool,
    pub acc_on: bool,
    pub deflection_angle: f64,
    pub battery_no: String,
    pub reply_success: bool,
    /// 回完应答后补一包定位，跟 Python 模拟器行为一致
    pub reply_with_location: bool,
}

impl Default for SimProfile {
    fn default() -> Self {
        Self {
            coordinates: "108.375256,22.767133".into(),
            vehicle_state: 0,
            motion: false,
            soc: 80,
            speed: 0,
            helmet_lock_unlocked: false,
            helmet_present: true,
            trunk_latch: true,
            acc_on: true,
            deflection_angle: 5.0,
            battery_no: String::new(),
            reply_success: true,
            reply_with_location: true,
        }
    }
}

impl SimProfile {
    fn coords(&self) -> (f64, f64) {
        frame::parse_coordinates(&self.coordinates).unwrap_or((108.375256, 22.767133))
    }

    pub fn location_opts(&self) -> frame::LocationOpts {
        let (lng, lat) = self.coords();
        frame::LocationOpts {
            lng,
            lat,
            vehicle_state: self.vehicle_state,
            motion: self.motion,
            soc: self.soc,
            speed: self.speed,
            helmet_lock_unlocked: self.helmet_lock_unlocked,
            helmet_present: self.helmet_present,
            trunk_latch: self.trunk_latch,
            acc_on: self.acc_on,
        }
    }

    pub fn pre_return_opts(&self, success: bool) -> frame::PreReturnOpts {
        let (lng, lat) = self.coords();
        frame::PreReturnOpts {
            success,
            deflection_angle: self.deflection_angle,
            lng,
            lat,
            helmet_present: self.helmet_present,
            trunk_latch: self.trunk_latch,
            speed: self.speed as u32,
            helmet_lock_unlocked: self.helmet_lock_unlocked,
        }
    }
}

/// 一台模拟设备的全部配置，随设备清单一起落盘
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DeviceConfig {
    pub device_no: String,
    pub host: String,
    pub port: u16,
    pub soft_version: String,
    pub heartbeat: bool,
    pub auto_reply: bool,
    pub profile: SimProfile,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            device_no: String::new(),
            host: DEFAULT_GATEWAY_HOST.to_string(),
            port: DEFAULT_GATEWAY_PORT,
            soft_version: frame::DEFAULT_SOFT_VERSION.to_string(),
            heartbeat: true,
            auto_reply: true,
            profile: SimProfile::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceState {
    pub connected: bool,
    pub endpoint: String,
    /// 最近一次收到的下发指令 msgId，手动应答时用
    pub last_msg_id: Option<String>,
    pub config: DeviceConfig,
}

#[derive(Default)]
struct Shared {
    connected: AtomicBool,
    /// 每次连接自增；旧的读/心跳任务发现代数变了就退出
    generation: Mutex<u64>,
    endpoint: Mutex<String>,
    last_msg_id: Mutex<Option<String>>,
    cfg: Mutex<DeviceConfig>,
    buffer: Mutex<Vec<FrameLog>>,
}

impl Shared {
    fn push(&self, log: FrameLog) {
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

    fn cfg(&self) -> DeviceConfig {
        self.cfg.lock().unwrap().clone()
    }
}

pub struct DeviceLink {
    shared: Arc<Shared>,
    writer: Arc<tokio::sync::Mutex<Option<OwnedWriteHalf>>>,
}

impl DeviceLink {
    pub fn new(cfg: DeviceConfig) -> Self {
        let shared = Arc::new(Shared::default());
        *shared.cfg.lock().unwrap() = cfg;
        Self {
            shared,
            writer: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub fn state(&self) -> DeviceState {
        DeviceState {
            connected: self.shared.connected.load(Ordering::Relaxed),
            endpoint: self.shared.endpoint.lock().unwrap().clone(),
            last_msg_id: self.shared.last_msg_id.lock().unwrap().clone(),
            config: self.shared.cfg(),
        }
    }

    pub fn config(&self) -> DeviceConfig {
        self.shared.cfg()
    }

    /// 改配置不影响已经建立的链路，返回值表示网关地址是否变了
    pub fn set_config(&self, cfg: DeviceConfig) -> bool {
        let mut cur = self.shared.cfg.lock().unwrap();
        let moved = cur.host != cfg.host || cur.port != cfg.port;
        *cur = cfg;
        moved
    }

    pub fn device_no(&self) -> String {
        self.shared.cfg.lock().unwrap().device_no.clone()
    }

    pub fn connected(&self) -> bool {
        self.shared.connected.load(Ordering::Relaxed)
    }

    /// 往该设备的日志里记一条系统提示
    pub fn note(&self, text: impl Into<String>) {
        self.shared.push(FrameLog::sys(text));
    }

    fn current_device_no(&self) -> String {
        self.device_no()
    }

    fn profile(&self) -> SimProfile {
        self.shared.cfg.lock().unwrap().profile.clone()
    }

    pub async fn send_location(&self) -> Result<(), String> {
        let profile = self.profile();
        let data = frame::location(
            &self.current_device_no(),
            &profile.location_opts(),
            unix_now(),
            &payload_settings(),
        );
        self.send(&data, "定位上报").await
    }

    pub async fn send_bms(&self) -> Result<(), String> {
        let profile = self.profile();
        let data = frame::bms(
            &self.current_device_no(),
            profile.soc,
            &profile.battery_no,
            unix_now(),
            &payload_settings(),
        );
        self.send(&data, "BMS 电池数据").await
    }

    pub async fn send_alarm(&self, alarm_type: u8, label: &str) -> Result<(), String> {
        let data = frame::alarm(&self.current_device_no(), alarm_type, unix_now());
        self.send(&data, format!("告警[{label}] 0x{alarm_type:02X}"))
            .await
    }

    pub async fn send_ping(&self) -> Result<(), String> {
        let profile = self.profile();
        let data = frame::ping(&self.current_device_no(), profile.soc, &payload_settings());
        self.send(&data, "心跳").await
    }

    /// 手动应答：msg_id 省略时用最近一次收到的下发指令。
    pub async fn send_reply(&self, msg_id: Option<String>, success: bool) -> Result<(), String> {
        let msg_id = match msg_id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
            Some(id) => id,
            None => self
                .shared
                .last_msg_id
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| "还没有收到过下发指令，没有可应答的 msgId".to_string())?,
        };
        let data = frame::reply(&msg_id, success);
        self.send(&data, format!("手动应答 msgId={msg_id} 结果={}", if success { "成功" } else { "失败" }))
            .await
    }

    pub fn drain(&self) -> Vec<FrameLog> {
        std::mem::take(&mut *self.shared.buffer.lock().unwrap())
    }

    pub async fn connect(&self) -> Result<(), String> {
        let cfg = self.config();
        let (port, heartbeat) = (cfg.port, cfg.heartbeat);
        let configured = crate::config::load().settings.client.default_soft_version;
        let soft_version = if configured.trim().is_empty() {
            cfg.soft_version.clone()
        } else {
            configured
        };
        let host = cfg.host.trim();
        let device_no = cfg.device_no.trim();
        if host.is_empty() {
            return Err("网关地址不能为空".to_string());
        }
        if device_no.is_empty() {
            return Err("中控设备序列号不能为空".to_string());
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
        self.shared.push(FrameLog::sys(format!("已连接 {endpoint}")));

        self.send(&frame::login(device_no, &soft_version, unix_now()), "登录")
            .await?;

        {
            let shared = self.shared.clone();
            let writer = self.writer.clone();
            let reply_device_no = device_no.to_string();
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
                                shared.push(FrameLog::sys("连接已被服务端关闭"));
                            }
                            return;
                        }
                        Ok(n) => {
                            pending.extend_from_slice(&chunk[..n]);
                            let (frames, used) = frame::split_frames(&pending);
                            pending.drain(0..used);
                            for f in frames {
                                let parsed = frame::parse(&f);
                                let summary = parsed
                                    .as_ref()
                                    .map(|p| p.summary())
                                    .unwrap_or_else(|| "无法解析的报文".to_string());
                                shared.push(FrameLog {
                                    dir: "down".into(),
                                    summary,
                                    hex: frame::to_hex(&f),
                                    fields: parsed
                                        .as_ref()
                                        .map(|p| p.fields.clone())
                                        .unwrap_or_default(),
                                });
                                let Some(msg_id) = parsed.as_ref().and_then(|p| p.msg_id.clone())
                                else {
                                    continue;
                                };
                                *shared.last_msg_id.lock().unwrap() = Some(msg_id.clone());
                                let cfg = shared.cfg();
                                if !cfg.auto_reply {
                                    continue;
                                }
                                // 参数查询/设置各有专用应答，预还车要回一整套 TLV，其余回简单应答
                                let profile = cfg.profile;
                                let ok = profile.reply_success;
                                let command = parsed.as_ref().map(|p| p.command).unwrap_or(0);
                                let params =
                                    parsed.as_ref().map(|p| p.params.clone()).unwrap_or_default();
                                let is_pre_return = parsed
                                    .as_ref()
                                    .and_then(|p| p.control_command)
                                    .is_some_and(|c| c == frame::CONTROL_PRE_RETURN);
                                let (data, what) = if command == frame::cmd::QUERY_PARAM {
                                    (
                                        frame::param_reply(
                                            frame::cmd::QUERY_PARAM_REPLY,
                                            &query_param_values(&params, ok),
                                            &msg_id,
                                        ),
                                        "查询参数应答",
                                    )
                                } else if command == frame::cmd::SET_PARAM {
                                    (
                                        frame::param_reply(
                                            frame::cmd::SET_PARAM_REPLY,
                                            &set_param_results(&params, ok),
                                            &msg_id,
                                        ),
                                        "设置参数应答",
                                    )
                                } else if is_pre_return {
                                    (
                                        frame::reply_pre_return(
                                            &msg_id,
                                            &profile.pre_return_opts(ok),
                                        ),
                                        "预还车应答",
                                    )
                                } else {
                                    (frame::reply(&msg_id, ok), "自动应答")
                                };
                                let mut guard = writer.lock().await;
                                let Some(w) = guard.as_mut() else { continue };
                                if w.write_all(&data.bytes).await.is_err() {
                                    continue;
                                }
                                shared.push(FrameLog::up(
                                    format!(
                                        "{what} msgId={msg_id} 结果={}",
                                        if ok { "成功" } else { "失败" }
                                    ),
                                    &data,
                                ));
                                let is_control = matches!(command, frame::cmd::CONTROL | frame::cmd::VOICE);
                                if is_control && profile.reply_with_location {
                                    let loc = frame::location(
                                        &reply_device_no,
                                        &profile.location_opts(),
                                        unix_now(),
                                        &payload_settings(),
                                    );
                                    if w.write_all(&loc.bytes).await.is_ok() {
                                        shared.push(FrameLog::up("应答后补发定位", &loc));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if shared.generation() == generation {
                                shared.connected.store(false, Ordering::Relaxed);
                                shared.push(FrameLog::sys(format!("读取失败，连接断开: {e}")));
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
            let device_no = device_no.to_string();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(interval).await;
                    if shared.generation() != generation || !shared.connected.load(Ordering::Relaxed)
                    {
                        return;
                    }
                    let soc = shared.cfg.lock().unwrap().profile.soc;
                    let data = frame::ping(&device_no, soc, &payload_settings());
                    let mut guard = writer.lock().await;
                    let Some(w) = guard.as_mut() else { return };
                    if w.write_all(&data.bytes).await.is_ok() {
                        shared.push(FrameLog::up("心跳", &data));
                    } else {
                        return;
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn send(&self, f: &Frame, summary: impl Into<String>) -> Result<(), String> {
        let mut guard = self.writer.lock().await;
        let Some(writer) = guard.as_mut() else {
            return Err("尚未连接网关".to_string());
        };
        writer
            .write_all(&f.bytes)
            .await
            .map_err(|e| format!("发送失败: {e}"))?;
        self.shared.push(FrameLog::up(summary, f));
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
            self.shared.push(FrameLog::sys("已断开连接"));
        }
    }
}

fn payload_settings() -> crate::config::ClientPayloadSettings {
    crate::config::load().settings.client.payload
}

fn ping_interval() -> Duration {
    let secs = crate::config::load().settings.client.default_heartbeat_interval;
    Duration::from_secs(if secs == 0 { DEFAULT_PING_SECS } else { secs })
}

fn unix_now() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

/// 查询参数：回参数字典里的默认值，字典里没有默认值的按协议回 FAIL
fn query_param_values(keys: &[String], ok: bool) -> Vec<(String, String)> {
    keys.iter()
        .map(|key| {
            let value = if !ok {
                None
            } else {
                crate::ecu::all()
                    .iter()
                    .find(|p| p.key.eq_ignore_ascii_case(key))
                    .and_then(|p| p.default_value.clone())
            };
            (key.clone(), value.unwrap_or_else(|| "FAIL".to_string()))
        })
        .collect()
}

/// 设置参数：逐项回 OK / FAIL，入参形如 "KEY=VALUE"
fn set_param_results(entries: &[String], ok: bool) -> Vec<(String, String)> {
    entries
        .iter()
        .map(|entry| {
            let key = entry.split('=').next().unwrap_or(entry).to_string();
            (key, if ok { "OK" } else { "FAIL" }.to_string())
        })
        .collect()
}

/// 多台模拟设备：按 deviceNo 索引，顺序就是界面上的列表顺序
#[derive(Default)]
pub struct DeviceFleet {
    links: Mutex<Vec<Arc<DeviceLink>>>,
}

impl DeviceFleet {
    pub fn seed(&self, configs: Vec<DeviceConfig>) {
        for cfg in configs.into_iter().rev() {
            self.upsert(cfg);
        }
    }

    /// 返回值：网关地址是否变了，变了说明换了环境，旧链路得断开
    pub fn upsert(&self, cfg: DeviceConfig) -> (Arc<DeviceLink>, bool) {
        let mut links = self.links.lock().unwrap();
        match links.iter().find(|l| l.device_no() == cfg.device_no) {
            Some(link) => {
                let moved = link.set_config(cfg);
                (link.clone(), moved)
            }
            None => {
                let link = Arc::new(DeviceLink::new(cfg));
                links.insert(0, link.clone());
                (link, false)
            }
        }
    }

    pub fn get(&self, device_no: &str) -> Result<Arc<DeviceLink>, String> {
        self.links
            .lock()
            .unwrap()
            .iter()
            .find(|l| l.device_no() == device_no)
            .cloned()
            .ok_or_else(|| format!("设备 {device_no} 不在模拟清单里"))
    }

    pub fn rename(&self, old_no: &str, new_no: &str) -> Result<(Arc<DeviceLink>, bool), String> {
        let links = self.links.lock().unwrap();
        if links.iter().any(|l| l.device_no() == new_no && l.device_no() != old_no) {
            return Err(format!("设备序列号 {new_no} 已存在"));
        }
        let link = links
            .iter()
            .find(|l| l.device_no() == old_no)
            .cloned()
            .ok_or_else(|| format!("设备 {old_no} 不在模拟清单里"))?;

        let mut cfg = link.config();
        cfg.device_no = new_no.to_string();
        let moved = link.set_config(cfg);
        Ok((link, moved))
    }

    pub fn remove(&self, device_no: &str) -> Option<Arc<DeviceLink>> {
        let mut links = self.links.lock().unwrap();
        let idx = links.iter().position(|l| l.device_no() == device_no)?;
        Some(links.remove(idx))
    }

    pub fn list(&self) -> Vec<Arc<DeviceLink>> {
        self.links.lock().unwrap().clone()
    }

    pub fn states(&self) -> Vec<DeviceState> {
        self.list().iter().map(|l| l.state()).collect()
    }

    pub fn roster(&self) -> Vec<DeviceConfig> {
        self.list().iter().map(|l| l.config()).collect()
    }
}
