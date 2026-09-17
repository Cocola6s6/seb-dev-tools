use crate::config::MysqlConfig;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, Pool, Row};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployRequest {
    pub bike_no: String,
    pub ecu_no: String,
    pub city_id: i64,
    pub bike_type_id: i64,
    pub supplier_id: i64,
    pub dealer_id: i64,
    pub device_company_id: i64,
    pub batch_no: String,
    pub motor_no: String,
    pub frame_no: String,
    pub has_helmet: bool,
    pub has_trunk: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployResult {
    pub steps: Vec<String>,
    pub bike_existed: bool,
    pub queue_code: u64,
    pub endpoint: String,
}

fn url(cfg: &MysqlConfig) -> String {
    format!(
        "mysql://{}:{}@{}:{}/{}",
        cfg.username, cfg.password, cfg.host, cfg.port, cfg.database
    )
}

/// 创建 MySQL 连接池（强制设置 session 时区为 +08:00，与 Java 后端对齐，避免 now() 产生时差）
async fn create_pool(cfg: &MysqlConfig) -> Result<Pool<MySql>, String> {
    MySqlPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(CONNECT_TIMEOUT)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET time_zone = '+08:00'").execute(conn).await?;
                Ok(())
            })
        })
        .connect(&url(cfg))
        .await
        .map_err(|e| format!("连接 MySQL {}:{} 失败: {e}", cfg.host, cfg.port))
}

pub async fn check_health(cfg: &MysqlConfig) -> Result<bool, String> {
    let pool = create_pool(cfg).await?;
    let _ = sqlx::query("select 1")
        .fetch_one(&pool)
        .await
        .map_err(|e| format!("MySQL 检查失败: {e}"))?;
    pool.close().await;
    Ok(true)
}

pub async fn check(cfg: &MysqlConfig) -> Result<String, String> {
    let pool = create_pool(cfg).await?;
    let bikes: i64 = sqlx::query("select count(*) from bike_tb")
        .fetch_one(&pool)
        .await
        .map_err(|e| format!("读取 bike_tb 失败: {e}"))?
        .get(0);
    let queued: i64 = sqlx::query("select count(*) from bike_ecu_relation_queue_tb where executed = 0")
        .fetch_one(&pool)
        .await
        .map_err(|e| format!("读取 bike_ecu_relation_queue_tb 失败: {e}"))?
        .get(0);
    pool.close().await;
    Ok(format!(
        "{}:{}/{} 连接正常：bike_tb {} 条，待执行绑定队列 {} 条",
        cfg.host, cfg.port, cfg.database, bikes, queued
    ))
}

pub async fn find_bike(cfg: &MysqlConfig, bike_no: &str) -> Result<Option<String>, String> {
    let pool = create_pool(cfg).await?;
    let row = sqlx::query(
        "select ecu_no, city_id, road_status, business_status, deleted \
         from bike_tb where bike_no = ? limit 1",
    )
    .bind(bike_no)
    .fetch_optional(&pool)
    .await
    .map_err(|e| format!("查询 bike_tb 失败: {e}"))?;
    pool.close().await;

    let Some(r) = row else {
        return Ok(None);
    };

    let ecu_no = r.get::<String, _>("ecu_no");
    let city_id = r.get::<i32, _>("city_id");
    let road_status = r.get::<String, _>("road_status");
    let business_status = r.get::<String, _>("business_status");
    let deleted = r.get::<i8, _>("deleted");

    let redis_instance = crate::redis::instance_of(&ecu_no).await.unwrap_or(None);
    let online_str = match redis_instance {
        Some(inst) => format!("在线 (实例 {inst})"),
        None => "未上线/无心跳".to_string(),
    };

    Ok(Some(format!(
        "ecuNo={}, cityId={}, 投放状态={}, 业务状态={}, 已删除={}, 设备状态={}",
        ecu_no, city_id, road_status, business_status, deleted, online_str
    )))
}

pub async fn deploy(cfg: &MysqlConfig, req: &DeployRequest) -> Result<DeployResult, String> {
    let bike_no = req.bike_no.trim();
    let ecu_no = req.ecu_no.trim();
    if bike_no.is_empty() {
        return Err("车辆编号 (bikeNo) 不能为空".to_string());
    }
    if ecu_no.is_empty() {
        return Err("中控设备序列号 (ecuNo) 不能为空".to_string());
    }
    if req.city_id < 0 {
        return Err("城市 ID 不能小于 0".to_string());
    }

    let goods_pool = create_pool(cfg).await?;
    let mut tx = goods_pool
        .begin()
        .await
        .map_err(|e| format!("开启事务失败: {e}"))?;
    let mut steps = Vec::new();

    let existed: Option<i64> = sqlx::query("select code from bike_tb where bike_no = ? limit 1")
        .bind(bike_no)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("查询 bike_tb 失败: {e}"))?
        .map(|r| r.get::<i32, _>("code") as i64);

    if existed.is_some() {
        let affected = sqlx::query(
            "update bike_tb set ecu_no = ?, city_id = ?, bike_type_id = ?, supplier_id = ?, \
             dealer_id = ?, device_company_id = ?, batch_no = ?, motor_no = ?, frame_no = ?, \
             has_helmet = ?, has_trunk = ?, road_status = 'put-in', business_status = 'normal', \
             dev_ops_status = 'normal', deleted = 0, put_time = now(), update_time = now(3) \
             where bike_no = ?",
        )
        .bind(ecu_no)
        .bind(req.city_id)
        .bind(req.bike_type_id)
        .bind(req.supplier_id)
        .bind(req.dealer_id)
        .bind(req.device_company_id)
        .bind(req.batch_no.trim())
        .bind(req.motor_no.trim())
        .bind(req.frame_no.trim())
        .bind(i8::from(req.has_helmet))
        .bind(i8::from(req.has_trunk))
        .bind(bike_no)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("更新 bike_tb 失败: {e}"))?
        .rows_affected();
        steps.push(format!(
            "bike_tb: 车辆已存在，更新为 ecuNo={ecu_no} / cityId={} / road_status=put-in（影响 {affected} 行）",
            req.city_id
        ));
    } else {
        let code = sqlx::query(
            "insert into bike_tb (bike_no, bike_type_id, city_id, supplier_id, dealer_id, \
             device_company_id, batch_no, ecu_no, motor_no, frame_no, road_status, \
             dev_ops_status, business_status, has_helmet, has_trunk, production_date, \
             put_time, create_time, update_time) \
             values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'put-in', 'normal', 'normal', ?, ?, \
             curdate(), now(), now(3), now(3))",
        )
        .bind(bike_no)
        .bind(req.bike_type_id)
        .bind(req.city_id)
        .bind(req.supplier_id)
        .bind(req.dealer_id)
        .bind(req.device_company_id)
        .bind(req.batch_no.trim())
        .bind(ecu_no)
        .bind(req.motor_no.trim())
        .bind(req.frame_no.trim())
        .bind(i8::from(req.has_helmet))
        .bind(i8::from(req.has_trunk))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("写入 bike_tb 失败: {e}"))?
        .last_insert_id();
        steps.push(format!("bike_tb: 新增车辆 {bike_no}（code={code}）"));
    }

    let queue_code = sqlx::query(
        "insert into bike_ecu_relation_queue_tb (bike_no, ecu_no, city_id, executed, create_time) \
         values (?, ?, ?, 1, now(3))",
    )
    .bind(bike_no)
    .bind(ecu_no)
    .bind(req.city_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("写入 bike_ecu_relation_queue_tb 失败: {e}"))?
    .last_insert_id();
    steps.push(format!(
        "bike_ecu_relation_queue_tb: 写入绑定队列（code={queue_code}, executed=1）"
    ));

    tx.commit().await.map_err(|e| format!("提交事务失败: {e}"))?;
    goods_pool.close().await;

    let mut report_cfg = cfg.clone();
    report_cfg.database = "seb_report_db".to_string();
    if let Ok(report_pool) = create_pool(&report_cfg).await {
        let report_exist: Option<i64> = sqlx::query("select code from bike_ecu_relation_tb where bike_no = ? limit 1")
            .bind(bike_no)
            .fetch_optional(&report_pool)
            .await
            .unwrap_or(None)
            .map(|r| r.get::<i32, _>("code") as i64);

        if report_exist.is_some() {
            let _ = sqlx::query(
                "update bike_ecu_relation_tb set ecu_no = ?, city_id = ?, deleted = 0, restart_status = 'not_need_restart', update_time = now(3) where bike_no = ?",
            )
            .bind(ecu_no)
            .bind(req.city_id)
            .bind(bike_no)
            .execute(&report_pool)
            .await;
            steps.push("seb_report_db.bike_ecu_relation_tb: 更新中控绑定关系".to_string());
        } else {
            let _ = sqlx::query(
                "insert into bike_ecu_relation_tb (bike_no, ecu_no, city_id, deleted, restart_status, create_time, update_time) values (?, ?, ?, 0, 'not_need_restart', now(3), now(3))",
            )
            .bind(bike_no)
            .bind(ecu_no)
            .bind(req.city_id)
            .execute(&report_pool)
            .await;
            steps.push("seb_report_db.bike_ecu_relation_tb: 写入中控绑定关系".to_string());
        }

        // 服务端下发指令前查 bike_basic_tb 判在线（last_connect_status_time 在 5 分钟内），
        // 生产链路由 seb-iot-receiver 异步写入；调试环境直写，否则会被拦为「车辆已离线」
        let basic_exist: Option<i64> = sqlx::query("select code from bike_basic_tb where device_serial_no = ? limit 1")
            .bind(ecu_no)
            .fetch_optional(&report_pool)
            .await
            .unwrap_or(None)
            .map(|r| r.get::<i32, _>("code") as i64);

        if basic_exist.is_some() {
            let _ = sqlx::query(
                "update bike_basic_tb set bike_no = ?, connect = 1, login_time = now(), last_connect_status_time = now(), update_time = now(3), server_instance_id = '0' where device_serial_no = ?",
            )
            .bind(bike_no)
            .bind(ecu_no)
            .execute(&report_pool)
            .await;
            steps.push("seb_report_db.bike_basic_tb: 更新设备基础登录状态（connect=1）".to_string());
        } else {
            let _ = sqlx::query(
                "insert into bike_basic_tb (bike_no, device_serial_no, login_reason, imei, imsi, manufacturer_code, soft_version, connect, device_time, receive_time, login_time, last_connect_status_time, create_time, update_time, server_instance_id) \
                 values (?, ?, 0, '', '', 'DEV', '1.0.0', 1, now(), now(), now(), now(), now(3), now(3), '0')",
            )
            .bind(bike_no)
            .bind(ecu_no)
            .execute(&report_pool)
            .await;
            steps.push("seb_report_db.bike_basic_tb: 写入初始设备基础信息（connect=1）".to_string());
        }

        report_pool.close().await;
    }

    match crate::redis::sync_bike_deploy_cache(ecu_no, bike_no, req.city_id).await {
        Ok(()) => {
            steps.push(format!(
                "Redis: 写入鉴权缓存 bike:device:serial:no:{ecu_no} -> {bike_no} 与 bike:city:{bike_no} -> {}",
                req.city_id
            ));
        }
        Err(e) => {
            steps.push(format!("Redis 写入警告: {e}"));
        }
    }

    Ok(DeployResult {
        steps,
        bike_existed: existed.is_some(),
        queue_code,
        endpoint: format!("{}:{}/{}", cfg.host, cfg.port, cfg.database),
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployOptionItem {
    pub id: i64,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployOptions {
    pub cities: Vec<DeployOptionItem>,
    pub bike_types: Vec<DeployOptionItem>,
    pub suppliers: Vec<DeployOptionItem>,
    pub dealers: Vec<DeployOptionItem>,
    pub device_companies: Vec<DeployOptionItem>,
}

pub fn default_deploy_options() -> DeployOptions {
    DeployOptions {
        cities: vec![DeployOptionItem { id: 0, label: "0".into() }],
        bike_types: vec![DeployOptionItem { id: 0, label: "0".into() }],
        suppliers: vec![DeployOptionItem { id: 0, label: "0".into() }],
        dealers: vec![DeployOptionItem { id: 0, label: "0".into() }],
        device_companies: vec![DeployOptionItem { id: 0, label: "0".into() }],
    }
}

pub async fn load_deploy_options(cfg: &MysqlConfig) -> DeployOptions {
    let mut defaults = default_deploy_options();
    let pool = match create_pool(cfg).await {
        Ok(p) => p,
        Err(_) => return defaults,
    };

    if let Ok(rows) = sqlx::query("SELECT code, name FROM seb_organization_db.custom_city_tb ORDER BY code ASC")
        .fetch_all(&pool)
        .await
    {
        let items: Vec<DeployOptionItem> = rows
            .into_iter()
            .map(|r| {
                let id = r.get::<i32, _>("code") as i64;
                let name = r.get::<String, _>("name");
                DeployOptionItem {
                    id,
                    label: format!("{id} - {name}"),
                }
            })
            .collect();
        if !items.is_empty() {
            defaults.cities = items;
        }
    }

    if let Ok(rows) = sqlx::query("SELECT code, model_type FROM seb_goods_db.bike_type_tb ORDER BY code ASC")
        .fetch_all(&pool)
        .await
    {
        let items: Vec<DeployOptionItem> = rows
            .into_iter()
            .map(|r| {
                let id = r.get::<i32, _>("code") as i64;
                let name = r.get::<String, _>("model_type");
                let name = if name.trim().is_empty() { "未命名车型".to_string() } else { name };
                DeployOptionItem {
                    id,
                    label: format!("{id} - {name}"),
                }
            })
            .collect();
        if !items.is_empty() {
            defaults.bike_types = items;
        }
    }

    if let Ok(rows) = sqlx::query("SELECT code, name FROM seb_organization_db.supplier_tb ORDER BY code ASC")
        .fetch_all(&pool)
        .await
    {
        let items: Vec<DeployOptionItem> = rows
            .into_iter()
            .map(|r| {
                let id = r.get::<i32, _>("code") as i64;
                let name = r.get::<String, _>("name");
                DeployOptionItem {
                    id,
                    label: format!("{id} - {name}"),
                }
            })
            .collect();
        if !items.is_empty() {
            defaults.suppliers = items;
        }
    }

    if let Ok(rows) = sqlx::query("SELECT code, name FROM seb_organization_db.dealer_tb ORDER BY code ASC")
        .fetch_all(&pool)
        .await
    {
        let items: Vec<DeployOptionItem> = rows
            .into_iter()
            .map(|r| {
                let id = r.get::<i32, _>("code") as i64;
                let name = r.get::<String, _>("name");
                DeployOptionItem {
                    id,
                    label: format!("{id} - {name}"),
                }
            })
            .collect();
        if !items.is_empty() {
            defaults.dealers = items;
        }
    }

    if let Ok(rows) = sqlx::query("SELECT code, company_name FROM seb_organization_db.device_company_tb ORDER BY code ASC")
        .fetch_all(&pool)
        .await
    {
        let items: Vec<DeployOptionItem> = rows
            .into_iter()
            .map(|r| {
                let id = r.get::<i32, _>("code") as i64;
                let name = r.get::<String, _>("company_name");
                DeployOptionItem {
                    id,
                    label: format!("{id} - {name}"),
                }
            })
            .collect();
        if !items.is_empty() {
            defaults.device_companies = items;
        }
    }

    pool.close().await;
    defaults
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_deploy_options() {
        let cfg = crate::config::mysql();
        let opts = load_deploy_options(&cfg).await;
        assert!(!opts.cities.is_empty());
        assert!(!opts.bike_types.is_empty());
        assert!(!opts.suppliers.is_empty());
        assert!(!opts.dealers.is_empty());
        assert!(!opts.device_companies.is_empty());
    }
}

