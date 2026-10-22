use crate::config::JswSettings;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

pub const DEFAULT_JSW_BASE_URL: &str = "https://jsw.gydev.cn";
pub const DEFAULT_JSW_TOKEN: &str = "jsw_48312d76bbc5e7cc33c1e17543c25a70165551c9";
pub const SEB_SERVER_CODE: &str = "1006";
pub const SEB_DATABASE: &str = "seb_goods_db";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);

fn resolve_endpoint(cfg: &JswSettings) -> (String, String, String, String) {
    let base_url = if !cfg.base_url.trim().is_empty() {
        cfg.base_url.trim().trim_end_matches('/').to_string()
    } else {
        std::env::var("JSW_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_JSW_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string()
    };

    let token = if !cfg.token.trim().is_empty() {
        cfg.token.trim().to_string()
    } else {
        std::env::var("JSW_ACCESS_TOKEN").unwrap_or_else(|_| DEFAULT_JSW_TOKEN.to_string())
    };

    let server_code = if !cfg.server_code.trim().is_empty() {
        cfg.server_code.trim().to_string()
    } else {
        SEB_SERVER_CODE.to_string()
    };

    let database = if !cfg.database.trim().is_empty() {
        cfg.database.trim().to_string()
    } else {
        SEB_DATABASE.to_string()
    };

    (base_url, token, server_code, database)
}

#[derive(Deserialize, Debug)]
struct JswResponse {
    #[serde(default)]
    status: bool,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Vec<serde_json::Value>,
}

fn sanitize_sql_val(val: &str) -> String {
    val.replace('\\', "\\\\").replace('\'', "''")
}

/// 通过 JSW HTTP API 在指定数据库（默认 1006 / seb_goods_db）上执行只读 SQL 查询
pub async fn query_seb_db_with_cfg(
    cfg: &JswSettings,
    sql: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let (base_url, token, server_code, database) = resolve_endpoint(cfg);
    let url = format!("{base_url}/database/query/{server_code}/{database}");

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "text/plain")
        .body(sql.to_string())
        .send()
        .await
        .map_err(|e| format!("请求 JSW 失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("JSW 响应 HTTP {}", resp.status()));
    }

    let result: JswResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析 JSW 响应失败: {e}"))?;

    if !result.status {
        return Err(format!("JSW 查询错误: {}", result.message));
    }

    Ok(result.data)
}

pub async fn query_seb_db(sql: &str) -> Result<Vec<serde_json::Value>, String> {
    query_seb_db_with_cfg(&JswSettings::default(), sql).await
}

/// 通过 JSW 查询并解析车辆、ECU及电池信息（用于外网和正式环境）
pub async fn resolve_bike_or_ecu_with_cfg(
    cfg: &JswSettings,
    query: &str,
) -> Result<Option<crate::db::BikeResolveInfo>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(None);
    }

    let safe_q = sanitize_sql_val(q);
    let mut found_bike: Option<String> = None;
    let mut found_ecu: Option<String> = None;

    // 1. 优先按 bike_no 精确匹配 bike_tb
    let sql1 = format!(
        "select bike_no, ecu_no from bike_tb where bike_no = '{safe_q}' and deleted = 0 limit 1"
    );
    if let Ok(rows) = query_seb_db_with_cfg(cfg, &sql1).await {
        if let Some(r) = rows.first() {
            let b = r.get("bike_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            let e = r.get("ecu_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if !b.is_empty() {
                found_bike = Some(b);
            }
            if !e.is_empty() {
                found_ecu = Some(crate::device::normalize_ecu_no(&e));
            }
        }
    }

    // 2. 若 bike_tb 查到车辆但没有 ecu_no，或者未查到车辆，再查 bike_ecu_relation_tb
    if found_bike.is_none() || found_ecu.is_none() {
        let sql2 = format!(
            "select bike_no, ecu_no from bike_ecu_relation_tb where bike_no = '{safe_q}' and deleted = 0 order by id desc limit 1"
        );
        if let Ok(rows) = query_seb_db_with_cfg(cfg, &sql2).await {
            if let Some(r) = rows.first() {
                let b = r.get("bike_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                let e = r.get("ecu_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                if found_bike.is_none() && !b.is_empty() {
                    found_bike = Some(b);
                }
                if found_ecu.is_none() && !e.is_empty() {
                    found_ecu = Some(crate::device::normalize_ecu_no(&e));
                }
            }
        }
    }

    // 3. 如果按 bike_no 依然没有查到，尝试按 ecu_no 查找
    if found_bike.is_none() || found_ecu.is_none() {
        let norm_q = crate::device::normalize_ecu_no(q);
        if !norm_q.is_empty() {
            let safe_norm = sanitize_sql_val(&norm_q);
            let sql3 = format!(
                "select bike_no, ecu_no from bike_tb where ecu_no = '{safe_norm}' and deleted = 0 limit 1"
            );
            if let Ok(rows) = query_seb_db_with_cfg(cfg, &sql3).await {
                if let Some(r) = rows.first() {
                    let b = r.get("bike_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    let e = r.get("ecu_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if !b.is_empty() {
                        found_bike = Some(b);
                    }
                    found_ecu = Some(crate::device::normalize_ecu_no(if !e.is_empty() { &e } else { &norm_q }));
                }
            }

            if found_bike.is_none() || found_ecu.is_none() {
                let sql4 = format!(
                    "select bike_no, ecu_no from bike_ecu_relation_tb where ecu_no = '{safe_norm}' and deleted = 0 order by id desc limit 1"
                );
                if let Ok(rows) = query_seb_db_with_cfg(cfg, &sql4).await {
                    if let Some(r) = rows.first() {
                        let b = r.get("bike_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                        let e = r.get("ecu_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                        if found_bike.is_none() && !b.is_empty() {
                            found_bike = Some(b);
                        }
                        found_ecu = Some(crate::device::normalize_ecu_no(if !e.is_empty() { &e } else { &norm_q }));
                    }
                }
            }
        }
    }

    let (bike_no, ecu_no) = match (found_bike, found_ecu) {
        (Some(b), Some(e)) => (b, e),
        (Some(b), None) => (b.clone(), String::new()),
        (None, Some(e)) => (String::new(), e),
        (None, None) => return Ok(None),
    };

    if ecu_no.is_empty() {
        return Ok(None);
    }

    // 4. 查绑定的电池编号
    let mut battery_no = None;
    if !bike_no.is_empty() {
        let safe_bike = sanitize_sql_val(&bike_no);
        let bat_sql = format!(
            "select battery_no from bike_battery_tb where bike_no = '{safe_bike}' limit 1"
        );
        if let Ok(rows) = query_seb_db_with_cfg(cfg, &bat_sql).await {
            if let Some(r) = rows.first() {
                let bat = r.get("battery_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                if !bat.is_empty() {
                    battery_no = Some(bat);
                }
            }
        }
    }

    Ok(Some(crate::db::BikeResolveInfo {
        bike_no,
        ecu_no,
        battery_no,
    }))
}

pub async fn resolve_bike_or_ecu(query: &str) -> Result<Option<crate::db::BikeResolveInfo>, String> {
    resolve_bike_or_ecu_with_cfg(&JswSettings::default(), query).await
}

/// 通过 JSW 批量反查 ECU 对应的车牌号
pub async fn batch_find_bike_nos_by_ecus_with_cfg(
    cfg: &JswSettings,
    ecu_nos: &[String],
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let valid_ecus: Vec<String> = ecu_nos
        .iter()
        .map(|s| crate::device::normalize_ecu_no(s))
        .filter(|s| !s.is_empty())
        .collect();
    if valid_ecus.is_empty() {
        return map;
    }

    for chunk in valid_ecus.chunks(50) {
        let in_clause = chunk
            .iter()
            .map(|e| format!("'{}'", sanitize_sql_val(e)))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "select ecu_no, bike_no from bike_tb where ecu_no in ({in_clause}) and deleted = 0"
        );
        if let Ok(rows) = query_seb_db_with_cfg(cfg, &sql).await {
            for r in rows {
                let ecu = r.get("ecu_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                let bike = r.get("bike_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                if !ecu.is_empty() && !bike.is_empty() {
                    map.insert(ecu, bike);
                }
            }
        }
    }

    // 针对未命中的，查 bike_ecu_relation_tb
    let missing: Vec<String> = valid_ecus
        .iter()
        .filter(|e| !map.contains_key(*e))
        .cloned()
        .collect();
    if !missing.is_empty() {
        for chunk in missing.chunks(50) {
            let in_clause = chunk
                .iter()
                .map(|e| format!("'{}'", sanitize_sql_val(e)))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "select ecu_no, bike_no from bike_ecu_relation_tb where ecu_no in ({in_clause}) and deleted = 0"
            );
            if let Ok(rows) = query_seb_db_with_cfg(cfg, &sql).await {
                for r in rows {
                    let ecu = r.get("ecu_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    let bike = r.get("bike_no").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if !ecu.is_empty() && !bike.is_empty() {
                        map.insert(ecu, bike);
                    }
                }
            }
        }
    }

    map
}

pub async fn batch_find_bike_nos_by_ecus(ecu_nos: &[String]) -> HashMap<String, String> {
    batch_find_bike_nos_by_ecus_with_cfg(&JswSettings::default(), ecu_nos).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jsw_resolve_bike() {
        let res = resolve_bike_or_ecu("A50001075104").await.expect("JSW query should succeed");
        assert!(res.is_some(), "应当解析到车辆");
        let info = res.unwrap();
        assert_eq!(info.bike_no, "A50001075104");
        assert_eq!(info.ecu_no, "019830198");
        assert_eq!(info.battery_no, Some("CMAH030B2E7S0029".to_string()));

        let res_ecu = resolve_bike_or_ecu("019830198").await.expect("JSW query should succeed");
        assert!(res_ecu.is_some(), "应当解析到车辆");
        let info_ecu = res_ecu.unwrap();
        assert_eq!(info_ecu.bike_no, "A50001075104");

        let map = batch_find_bike_nos_by_ecus(&["019830198".to_string()]).await;
        assert_eq!(map.get("019830198").map(|s| s.as_str()), Some("A50001075104"));
    }
}
