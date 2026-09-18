use seb_core::db::{self, BikeDetail, DeployOptions, DeployRequest, DeployResult};

#[tauri::command]
pub async fn load_deploy_options() -> DeployOptions {
    db::load_deploy_options(&seb_core::config::mysql()).await
}

#[tauri::command]
pub async fn bike_deploy(
    bike_no: String,
    ecu_no: String,
    battery_no: String,
    battery_type_id: i64,
    battery_pid: String,
    city_id: i64,
    bike_type_id: i64,
    supplier_id: i64,
    dealer_id: i64,
    device_company_id: i64,
    batch_no: String,
    motor_no: String,
    frame_no: String,
    has_helmet: bool,
    has_trunk: bool,
) -> Result<DeployResult, String> {
    let req = DeployRequest {
        bike_no,
        ecu_no,
        battery_no,
        battery_type_id,
        battery_pid,
        city_id,
        bike_type_id,
        supplier_id,
        dealer_id,
        device_company_id,
        batch_no,
        motor_no,
        frame_no,
        has_helmet,
        has_trunk,
    };
    db::deploy(&seb_core::config::mysql(), &req).await
}

#[tauri::command]
pub async fn bike_lookup(bike_no: String) -> Result<Option<BikeDetail>, String> {
    let bike_no = bike_no.trim().to_string();
    if bike_no.is_empty() {
        return Err("车辆编号 (bikeNo) 不能为空".to_string());
    }
    db::find_bike(&seb_core::config::mysql(), &bike_no).await
}
