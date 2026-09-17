//! 工具冒充一台 ECU 主动连上 IoT 网关：设备在运营商 NAT 后面，外部无法反向连它，
//! 想收发原始报文只能自己连上去。

use crate::frame;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;

use serde::{Deserialize, Serialize};

pub const DEFAULT_GATEWAY_HOST: &str = "bike-seb-inner-test.costrip.cn";
pub const DEFAULT_GATEWAY_PORT: u16 = 32405;

const PING_INTERVAL: Duration = Duration::from_secs(60);
const MAX_BUFFERED: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrameLog {
    pub dir: String,
    pub summary: String,
    pub hex: String,
}

impl FrameLog {
    fn sys(summary: impl Into<String>) -> Self {
        Self {
            dir: "sys".into(),
            summary: summary.into(),
            hex: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceState {
    pub connected: bool,
    pub endpoint: String,
    pub device_no: String,
    /// 收到下发指令时是否自动回应答
    pub auto_reply: bool,
}

#[derive(Default)]
struct Shared {
    connected: AtomicBool,
    auto_reply: AtomicBool,
    /// 每次连接自增；旧的读/心跳任务发现代数变了就退出
    generation: Mutex<u64>,
    endpoint: Mutex<String>,
    device_no: Mutex<String>,
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
}

pub struct DeviceLink {
    shared: Arc<Shared>,
    writer: Arc<tokio::sync::Mutex<Option<OwnedWriteHalf>>>,
}

impl Default for DeviceLink {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceLink {
    pub fn new() -> Self {
        let shared = Arc::new(Shared::default());
        shared.auto_reply.store(true, Ordering::Relaxed);
        Self {
            shared,
            writer: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub fn state(&self) -> DeviceState {
        DeviceState {
            connected: self.shared.connected.load(Ordering::Relaxed),
            endpoint: self.shared.endpoint.lock().unwrap().clone(),
            device_no: self.shared.device_no.lock().unwrap().clone(),
            auto_reply: self.shared.auto_reply.load(Ordering::Relaxed),
        }
    }

    pub fn set_auto_reply(&self, on: bool) {
        self.shared.auto_reply.store(on, Ordering::Relaxed);
    }

    pub fn drain(&self) -> Vec<FrameLog> {
        std::mem::take(&mut *self.shared.buffer.lock().unwrap())
    }

    pub async fn connect(
        &self,
        host: &str,
        port: u16,
        device_no: &str,
        soft_version: &str,
        heartbeat: bool,
    ) -> Result<(), String> {
        let host = host.trim();
        let device_no = device_no.trim();
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
        *self.shared.device_no.lock().unwrap() = device_no.to_string();
        self.shared.connected.store(true, Ordering::Relaxed);
        *self.writer.lock().await = Some(write_half);
        self.shared.push(FrameLog::sys(format!("已连接 {endpoint}")));

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        self.send(&frame::login(device_no, soft_version, now), "登录")
            .await?;

        {
            let shared = self.shared.clone();
            let writer = self.writer.clone();
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
                                });
                                // 自动回应答
                                if shared.auto_reply.load(Ordering::Relaxed) {
                                    if let Some(msg_id) =
                                        parsed.as_ref().and_then(|p| p.msg_id.clone())
                                    {
                                        let data = frame::reply(&msg_id, true);
                                        let mut guard = writer.lock().await;
                                        if let Some(w) = guard.as_mut() {
                                            if w.write_all(&data).await.is_ok() {
                                                shared.push(FrameLog {
                                                    dir: "up".into(),
                                                    summary: format!("自动应答 msgId={msg_id}"),
                                                    hex: frame::to_hex(&data),
                                                });
                                            }
                                        }
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
            let shared = self.shared.clone();
            let writer = self.writer.clone();
            let device_no = device_no.to_string();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(PING_INTERVAL).await;
                    if shared.generation() != generation || !shared.connected.load(Ordering::Relaxed)
                    {
                        return;
                    }
                    let data = frame::ping(&device_no, 255, 0.1);
                    let mut guard = writer.lock().await;
                    let Some(w) = guard.as_mut() else { return };
                    if w.write_all(&data).await.is_ok() {
                        shared.push(FrameLog {
                            dir: "up".into(),
                            summary: "心跳".into(),
                            hex: frame::to_hex(&data),
                        });
                    } else {
                        return;
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn send(&self, data: &[u8], summary: impl Into<String>) -> Result<(), String> {
        let mut guard = self.writer.lock().await;
        let Some(writer) = guard.as_mut() else {
            return Err("尚未连接网关".to_string());
        };
        writer
            .write_all(data)
            .await
            .map_err(|e| format!("发送失败: {e}"))?;
        self.shared.push(FrameLog {
            dir: "up".into(),
            summary: summary.into(),
            hex: frame::to_hex(data),
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
            self.shared.push(FrameLog::sys("已断开连接"));
        }
    }
}
