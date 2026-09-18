use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub const FLINK_HOST: &str = "10.12.55.240";
pub const FLINK_PORT: u16 = 80;

pub const FLINK_HIGH_URL: &str = "http://10.12.55.240/flink-operator/seb-flink-bike-iot-high/#/overview";
pub const FLINK_IOT_URL: &str = "http://10.12.55.240/flink-operator/seb-flink-bike-iot/#/overview";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlinkJobStatus {
    pub name: String,
    pub running: bool,
    pub state: String,
    pub taskmanagers: u32,
    pub slots_total: u32,
    pub slots_available: u32,
    pub tasks_running: u32,
    pub tasks_total: u32,
    pub dashboard_url: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlinkState {
    pub high: FlinkJobStatus,
    pub iot: FlinkJobStatus,
}

#[derive(Deserialize, Debug, Default)]
struct FlinkOverview {
    #[serde(default)]
    taskmanagers: u32,
    #[serde(rename = "slots-total", default)]
    slots_total: u32,
    #[serde(rename = "slots-available", default)]
    slots_available: u32,
}

#[derive(Deserialize, Debug, Default)]
struct FlinkJobsOverview {
    #[serde(default)]
    jobs: Vec<FlinkJobEntry>,
}

#[derive(Deserialize, Debug)]
struct FlinkJobEntry {
    name: String,
    state: String,
    #[serde(default)]
    tasks: Option<FlinkTasksEntry>,
}

#[derive(Deserialize, Debug)]
struct FlinkTasksEntry {
    #[serde(default)]
    running: u32,
    #[serde(default)]
    total: u32,
}

async fn fetch_http_json(path: &str) -> Result<String, String> {
    let mut stream = tokio::time::timeout(
        Duration::from_secs(3),
        TcpStream::connect((FLINK_HOST, FLINK_PORT)),
    )
    .await
    .map_err(|_| "连接 Flink 服务器超时".to_string())?
    .map_err(|e| format!("连接 Flink 失败: {e}"))?;

    let req = format!("GET {path} HTTP/1.1\r\nHost: {FLINK_HOST}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| format!("发送 HTTP 请求失败: {e}"))?;

    let mut resp = Vec::new();
    stream
        .read_to_end(&mut resp)
        .await
        .map_err(|e| format!("读取 HTTP 响应失败: {e}"))?;

    let text = String::from_utf8_lossy(&resp);
    if let Some((_, body)) = text.split_once("\r\n\r\n") {
        Ok(body.to_string())
    } else {
        Ok(text.to_string())
    }
}

pub async fn query_job(name: &str, base_path: &str, dashboard_url: &str, desc: &str) -> FlinkJobStatus {
    let mut status = FlinkJobStatus {
        name: name.to_string(),
        running: false,
        state: "OFFLINE".to_string(),
        taskmanagers: 0,
        slots_total: 0,
        slots_available: 0,
        tasks_running: 0,
        tasks_total: 0,
        dashboard_url: dashboard_url.to_string(),
        description: desc.to_string(),
    };

    let overview_path = format!("{base_path}/overview");
    let jobs_path = format!("{base_path}/jobs/overview");

    let (overview_res, jobs_res) = tokio::join!(
        fetch_http_json(&overview_path),
        fetch_http_json(&jobs_path),
    );

    if let Ok(body) = overview_res {
        if let Ok(overview) = serde_json::from_str::<FlinkOverview>(&body) {
            status.taskmanagers = overview.taskmanagers;
            status.slots_total = overview.slots_total;
            status.slots_available = overview.slots_available;
        }
    }

    if let Ok(body) = jobs_res {
        if let Ok(jobs_overview) = serde_json::from_str::<FlinkJobsOverview>(&body) {
            if let Some(job) = jobs_overview.jobs.into_iter().find(|j| j.name == name) {
                status.state = job.state.clone();
                status.running = job.state.to_uppercase() == "RUNNING" && status.taskmanagers > 0;
                if let Some(tasks) = job.tasks {
                    status.tasks_running = tasks.running;
                    status.tasks_total = tasks.total;
                }
            } else if status.taskmanagers > 0 {
                status.state = "NO_JOB".to_string();
            }
        }
    }

    status
}

pub async fn check_flink_state() -> FlinkState {
    let (high, iot) = tokio::join!(
        query_job(
            "seb-flink-bike-iot-high",
            "/flink-operator/seb-flink-bike-iot-high",
            FLINK_HIGH_URL,
            "处理设备上下线、登录鉴权、心跳与高优先级控制/告警流",
        ),
        query_job(
            "seb-flink-bike-iot",
            "/flink-operator/seb-flink-bike-iot",
            FLINK_IOT_URL,
            "处理 GPS 经纬度轨迹、偏向角、速度上报与 BMS 电池遥测流",
        ),
    );

    FlinkState { high, iot }
}
