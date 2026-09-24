use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub const REDIS_HOST: &str = "10.12.54.11";
pub const REDIS_PORT: u16 = 63602;
pub const REDIS_PASSWORD: &str = "Passw0rd";

const KEY_PREFIX: &str = "ecu:instance:id:";
pub const DEVICE_SERIAL_NO_PREFIX: &str = "bike:device:serial:no:";
pub const BIKE_CITY_PREFIX: &str = "bike:city:";
const TIMEOUT: Duration = Duration::from_secs(5);

pub fn redis_cfg() -> (String, u16, String, u8) {
    (REDIS_HOST.to_string(), REDIS_PORT, REDIS_PASSWORD.to_string(), 0)
}

pub fn endpoint() -> String {
    let (host, port, _, db) = redis_cfg();
    format!("{host}:{port}/{db}")
}

pub async fn check_health() -> Result<bool, String> {
    let (host, port, password, _) = redis_cfg();
    let stream = match tokio::time::timeout(Duration::from_secs(3), TcpStream::connect((host.as_str(), port))).await {
        Ok(Ok(s)) => s,
        _ => return Ok(false),
    };
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    for args in [
        vec!["AUTH", password.as_str()],
        vec!["PING"],
    ] {
        if writer.write_all(&encode(&args)).await.is_err() {
            return Ok(false);
        }
    }
    if writer.flush().await.is_err() {
        return Ok(false);
    }

    let _ = read_reply(&mut reader).await;
    match read_reply(&mut reader).await {
        Ok(Reply::Simple(s)) if s.to_uppercase() == "PONG" => Ok(true),
        _ => Ok(false),
    }
}

/// 查中控当前所在的网关实例号（决定下发指令的路由键后缀）；键不存在返回 `None`。
pub async fn instance_of(device_no: &str) -> Result<Option<String>, String> {
    let device_no = crate::device::normalize_ecu_no(device_no);
    if device_no.is_empty() {
        return Err("中控设备序列号 (DeviceNo) 不能为空".to_string());
    }
    let key = format!("{KEY_PREFIX}{device_no}");
    tokio::time::timeout(TIMEOUT, get(&key))
        .await
        .map_err(|_| format!("Redis {} 连接超时", endpoint()))?
}

/// 一键接入时补齐中控上线所需的缓存：`bike:device:serial:no:{ecuNo}` 是
/// seb-iot-server 鉴权中控登录的依据，缺了就连不上。
pub async fn sync_bike_deploy_cache(ecu_no: &str, bike_no: &str, city_id: i64) -> Result<(), String> {
    let ecu_no = crate::device::normalize_ecu_no(ecu_no);
    let ecu_key = format!("{DEVICE_SERIAL_NO_PREFIX}{ecu_no}");
    let city_key = format!("{BIKE_CITY_PREFIX}{bike_no}");
    let inst_key = format!("{KEY_PREFIX}{ecu_no}");
    let city_val = city_id.to_string();
    tokio::time::timeout(
        TIMEOUT,
        mset(&[(&ecu_key, bike_no), (&city_key, &city_val), (&inst_key, "0")]),
    )
    .await
    .map_err(|_| format!("Redis {} 写入超时", endpoint()))?
}

pub async fn mset(pairs: &[(&str, &str)]) -> Result<(), String> {
    if pairs.is_empty() {
        return Ok(());
    }
    let (host, port, password, db) = redis_cfg();
    let stream = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| format!("连接 Redis {} 失败: {e}", endpoint()))?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut mset_args = vec!["MSET"];
    for (k, v) in pairs {
        mset_args.push(k);
        mset_args.push(v);
    }

    let db_str = db.to_string();
    for args in [
        vec!["AUTH", password.as_str()],
        vec!["SELECT", db_str.as_str()],
        mset_args,
    ] {
        writer
            .write_all(&encode(&args))
            .await
            .map_err(|e| format!("发送 Redis 命令失败: {e}"))?;
    }
    writer
        .flush()
        .await
        .map_err(|e| format!("刷新 Redis 连接失败: {e}"))?;

    for _ in 0..3 {
        let _ = read_reply(&mut reader).await?;
    }
    Ok(())
}

pub async fn get(key: &str) -> Result<Option<String>, String> {
    let (host, port, password, db) = redis_cfg();
    let stream = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| format!("连接 Redis {} 失败: {e}", endpoint()))?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let db_str = db.to_string();
    for args in [
        vec!["AUTH", password.as_str()],
        vec!["SELECT", db_str.as_str()],
        vec!["GET", key],
    ] {
        writer
            .write_all(&encode(&args))
            .await
            .map_err(|e| format!("发送 Redis 命令失败: {e}"))?;
    }
    writer
        .flush()
        .await
        .map_err(|e| format!("发送 Redis 命令失败: {e}"))?;

    // 三条命令的回复按序读取，只有最后一条 GET 的结果有用
    check(read_reply(&mut reader).await?, "AUTH")?;
    check(read_reply(&mut reader).await?, "SELECT")?;
    match read_reply(&mut reader).await? {
        Reply::Error(e) => Err(format!("Redis GET 返回错误: {e}")),
        Reply::Nil => Ok(None),
        Reply::Bulk(v) | Reply::Simple(v) => Ok(Some(v)),
    }
}

/// RESP 数组编码：`*N\r\n$len\r\narg\r\n…`
fn encode(args: &[&str]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", args.len()).into_bytes();
    for a in args {
        out.extend_from_slice(format!("${}\r\n{a}\r\n", a.len()).as_bytes());
    }
    out
}

enum Reply {
    Simple(String),
    Bulk(String),
    Nil,
    Error(String),
}

fn check(reply: Reply, what: &str) -> Result<(), String> {
    match reply {
        Reply::Error(e) => Err(format!("Redis {what} 失败: {e}")),
        _ => Ok(()),
    }
}

async fn read_reply(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> Result<Reply, String> {
    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .await
        .map_err(|e| format!("读取 Redis 响应失败: {e}"))?;
    if n == 0 {
        return Err("Redis 连接被对端关闭".to_string());
    }
    let line = line.trim_end_matches(['\r', '\n']).to_string();
    let (tag, rest) = line.split_at(1);
    match tag {
        "+" | ":" => Ok(Reply::Simple(rest.to_string())),
        "-" => Ok(Reply::Error(rest.to_string())),
        "$" => {
            let len: i64 = rest
                .parse()
                .map_err(|_| format!("Redis 响应格式异常: {line}"))?;
            if len < 0 {
                return Ok(Reply::Nil);
            }
            // 正文之后还有 \r\n
            let mut buf = vec![0u8; len as usize + 2];
            reader
                .read_exact(&mut buf)
                .await
                .map_err(|e| format!("读取 Redis 响应失败: {e}"))?;
            buf.truncate(len as usize);
            String::from_utf8(buf)
                .map(Reply::Bulk)
                .map_err(|_| "Redis 响应不是合法 UTF-8".to_string())
        }
        _ => Err(format!("不支持的 Redis 响应类型: {line}")),
    }
}
