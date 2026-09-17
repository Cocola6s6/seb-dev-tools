use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MQ_HOST: &str = "10.12.54.12";
pub const MQ_PORT: u16 = 5672;
pub const MQ_USERNAME: &str = "qkswq";
pub const MQ_PASSWORD: &str = "qkswq";

pub const MYSQL_HOST: &str = "10.12.55.40";
pub const MYSQL_PORT: u16 = 40022;
pub const MYSQL_USERNAME: &str = "dev_code_test";
pub const MYSQL_PASSWORD: &str = "BA6B2030B21A22F7";
pub const MYSQL_DATABASE: &str = "seb_goods_db";

pub const DEFAULT_DEVICE_NO: &str = "019552878";

pub fn mq_endpoint() -> String {
    format!("{MQ_HOST}:{MQ_PORT}")
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MysqlConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl Default for MysqlConfig {
    fn default() -> Self {
        Self {
            host: MYSQL_HOST.to_string(),
            port: MYSQL_PORT,
            username: MYSQL_USERNAME.to_string(),
            password: MYSQL_PASSWORD.to_string(),
            database: MYSQL_DATABASE.to_string(),
        }
    }
}

pub fn mysql() -> MysqlConfig {
    MysqlConfig::default()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployDefaults {
    pub bike_type_id: i64,
    pub supplier_id: i64,
    pub dealer_id: i64,
    pub device_company_id: i64,
    pub has_helmet: bool,
    pub has_trunk: bool,
}

impl Default for DeployDefaults {
    fn default() -> Self {
        Self {
            bike_type_id: 0,
            supplier_id: 0,
            dealer_id: 0,
            device_company_id: 0,
            has_helmet: true,
            has_trunk: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub instance: String,
    pub device_no: String,
    pub bike_no: String,
    #[serde(default)]
    pub city_id: i64,
    #[serde(default)]
    pub deploy: DeployDefaults,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            instance: "0".to_string(),
            device_no: DEFAULT_DEVICE_NO.to_string(),
            bike_no: String::new(),
            city_id: 0,
            deploy: DeployDefaults::default(),
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("无法定位用户配置目录")?
        .join("seb-dev-tools");
    std::fs::create_dir_all(&dir).with_context(|| format!("创建配置目录失败: {}", dir.display()))?;
    Ok(dir.join("config.json"))
}

pub fn load() -> AppConfig {
    let Ok(path) = config_path() else {
        return AppConfig::default();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(cfg: &AppConfig) -> Result<()> {
    let path = config_path()?;
    let body = serde_json::to_string_pretty(cfg)?;
    std::fs::write(&path, body).with_context(|| format!("写入配置失败: {}", path.display()))?;
    Ok(())
}
