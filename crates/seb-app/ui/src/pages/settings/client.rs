use crate::state::{ClientGlobalSettings, ClientPayloadSettings};
use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub struct ClientSection {
    qr_url_template: Signal<String>,
    default_inner_gw: Signal<String>,
    default_test_gw: Signal<String>,
    default_prod_gw: Signal<String>,
    default_soft_version: Signal<String>,
    default_coordinates: Signal<String>,
    default_soc: Signal<String>,
    default_speed: Signal<String>,
    default_deflection_angle: Signal<String>,
    default_heartbeat_interval: Signal<String>,

    cp_altitude: Signal<String>,
    cp_heading: Signal<String>,
    cp_gps_speed: Signal<String>,
    cp_gps_status: Signal<String>,
    cp_satellites: Signal<String>,
    cp_backup_battery: Signal<String>,
    cp_gsm_signal: Signal<String>,
    cp_mcc: Signal<String>,
    cp_mnc: Signal<String>,
    cp_lac: Signal<String>,
    cp_cell_id: Signal<String>,
    cp_available_capacity: Signal<String>,
    cp_soh: Signal<String>,
    cp_charge_cycles: Signal<String>,
    cp_ride_current: Signal<String>,
    cp_controller_temperature: Signal<String>,
    cp_voltage: Signal<String>,
    cp_total_mileage: Signal<String>,
    cp_trip_mileage: Signal<String>,

    cp_bms_remain_capacity: Signal<String>,
    cp_bms_full_capacity: Signal<String>,
    cp_bms_soh: Signal<String>,
    cp_bms_temperature: Signal<String>,
    cp_bms_current: Signal<String>,
    cp_bms_voltage: Signal<String>,
    cp_bms_cycle_count: Signal<String>,
    cp_bms_cell_voltages: Signal<String>,
    cp_bms_charge_interval: Signal<String>,
    cp_bms_max_charge_interval: Signal<String>,
    cp_bms_version: Signal<String>,
    cp_bms_manufacturer: Signal<String>,
}

impl ClientSection {
    pub fn new() -> Self {
        Self {
            qr_url_template: create_signal(String::new()),
            default_inner_gw: create_signal(String::new()),
            default_test_gw: create_signal(String::new()),
            default_prod_gw: create_signal(String::new()),
            default_soft_version: create_signal(String::new()),
            default_coordinates: create_signal(String::new()),
            default_soc: create_signal(String::new()),
            default_speed: create_signal(String::new()),
            default_deflection_angle: create_signal(String::new()),
            default_heartbeat_interval: create_signal(String::new()),

            cp_altitude: create_signal(String::new()),
            cp_heading: create_signal(String::new()),
            cp_gps_speed: create_signal(String::new()),
            cp_gps_status: create_signal(String::new()),
            cp_satellites: create_signal(String::new()),
            cp_backup_battery: create_signal(String::new()),
            cp_gsm_signal: create_signal(String::new()),
            cp_mcc: create_signal(String::new()),
            cp_mnc: create_signal(String::new()),
            cp_lac: create_signal(String::new()),
            cp_cell_id: create_signal(String::new()),
            cp_available_capacity: create_signal(String::new()),
            cp_soh: create_signal(String::new()),
            cp_charge_cycles: create_signal(String::new()),
            cp_ride_current: create_signal(String::new()),
            cp_controller_temperature: create_signal(String::new()),
            cp_voltage: create_signal(String::new()),
            cp_total_mileage: create_signal(String::new()),
            cp_trip_mileage: create_signal(String::new()),

            cp_bms_remain_capacity: create_signal(String::new()),
            cp_bms_full_capacity: create_signal(String::new()),
            cp_bms_soh: create_signal(String::new()),
            cp_bms_temperature: create_signal(String::new()),
            cp_bms_current: create_signal(String::new()),
            cp_bms_voltage: create_signal(String::new()),
            cp_bms_cycle_count: create_signal(String::new()),
            cp_bms_cell_voltages: create_signal(String::new()),
            cp_bms_charge_interval: create_signal(String::new()),
            cp_bms_max_charge_interval: create_signal(String::new()),
            cp_bms_version: create_signal(String::new()),
            cp_bms_manufacturer: create_signal(String::new()),
        }
    }

    pub fn load(self, c: ClientGlobalSettings) {
        let s = self;
        s.qr_url_template.set(c.qr_url_template);
        s.default_inner_gw.set(c.default_inner_gw);
        s.default_test_gw.set(c.default_test_gw);
        s.default_prod_gw.set(c.default_prod_gw);
        s.default_soft_version.set(c.default_soft_version);
        s.default_coordinates.set(c.default_coordinates);
        s.default_soc.set(c.default_soc.to_string());
        s.default_speed.set(c.default_speed.to_string());
        s.default_deflection_angle.set(c.default_deflection_angle.to_string());
        s.default_heartbeat_interval.set(c.default_heartbeat_interval.to_string());

        let p = c.payload;
        s.cp_altitude.set(p.altitude.to_string());
        s.cp_heading.set(p.heading.to_string());
        s.cp_gps_speed.set(p.gps_speed.to_string());
        s.cp_gps_status.set(p.gps_status.to_string());
        s.cp_satellites.set(p.satellites.to_string());
        s.cp_backup_battery.set(p.backup_battery.to_string());
        s.cp_gsm_signal.set(p.gsm_signal.to_string());
        s.cp_mcc.set(p.mcc.to_string());
        s.cp_mnc.set(p.mnc.to_string());
        s.cp_lac.set(p.lac.to_string());
        s.cp_cell_id.set(p.cell_id.to_string());
        s.cp_available_capacity.set(p.available_capacity.to_string());
        s.cp_soh.set(p.soh.to_string());
        s.cp_charge_cycles.set(p.charge_cycles.to_string());
        s.cp_ride_current.set(p.ride_current.to_string());
        s.cp_controller_temperature.set(p.controller_temperature.to_string());
        s.cp_voltage.set(p.voltage.to_string());
        s.cp_total_mileage.set(p.total_mileage.to_string());
        s.cp_trip_mileage.set(p.trip_mileage.to_string());
        s.cp_bms_remain_capacity.set(p.bms_remain_capacity.to_string());
        s.cp_bms_full_capacity.set(p.bms_full_capacity.to_string());
        s.cp_bms_soh.set(p.bms_soh.to_string());
        s.cp_bms_temperature.set(p.bms_temperature.to_string());
        s.cp_bms_current.set(p.bms_current.to_string());
        s.cp_bms_voltage.set(p.bms_voltage.to_string());
        s.cp_bms_cycle_count.set(p.bms_cycle_count.to_string());
        s.cp_bms_cell_voltages.set(p.bms_cell_voltages);
        s.cp_bms_charge_interval.set(p.bms_charge_interval.to_string());
        s.cp_bms_max_charge_interval.set(p.bms_max_charge_interval.to_string());
        s.cp_bms_version.set(p.bms_version.to_string());
        s.cp_bms_manufacturer.set(p.bms_manufacturer);
    }

    pub fn collect(self) -> ClientGlobalSettings {
        let s = self;
        ClientGlobalSettings {
            qr_url_template: s.qr_url_template.get_clone().trim().to_string(),
            default_inner_gw: s.default_inner_gw.get_clone().trim().to_string(),
            default_test_gw: s.default_test_gw.get_clone().trim().to_string(),
            default_prod_gw: s.default_prod_gw.get_clone().trim().to_string(),
            default_soft_version: s.default_soft_version.get_clone().trim().to_string(),
            default_coordinates: s.default_coordinates.get_clone().trim().to_string(),
            default_soc: s.default_soc.get_clone().trim().parse().unwrap_or(98),
            default_speed: s.default_speed.get_clone().trim().parse().unwrap_or(0),
            default_deflection_angle: s.default_deflection_angle.get_clone().trim().parse().unwrap_or(0),
            default_heartbeat_interval: s.default_heartbeat_interval.get_clone().trim().parse().unwrap_or(60),
            payload: ClientPayloadSettings {
                altitude: s.cp_altitude.get_clone().trim().parse().unwrap_or(90),
                heading: s.cp_heading.get_clone().trim().parse().unwrap_or(25),
                gps_speed: s.cp_gps_speed.get_clone().trim().parse().unwrap_or(20),
                gps_status: s.cp_gps_status.get_clone().trim().parse().unwrap_or(2),
                satellites: s.cp_satellites.get_clone().trim().parse().unwrap_or(9),
                backup_battery: s.cp_backup_battery.get_clone().trim().parse().unwrap_or(9),
                gsm_signal: s.cp_gsm_signal.get_clone().trim().parse().unwrap_or(9),
                mcc: s.cp_mcc.get_clone().trim().parse().unwrap_or(u16::MAX),
                mnc: s.cp_mnc.get_clone().trim().parse().unwrap_or(u16::MAX),
                lac: s.cp_lac.get_clone().trim().parse().unwrap_or(u16::MAX),
                cell_id: s.cp_cell_id.get_clone().trim().parse().unwrap_or(u16::MAX),
                available_capacity: s.cp_available_capacity.get_clone().trim().parse().unwrap_or(u16::MAX),
                soh: s.cp_soh.get_clone().trim().parse().unwrap_or(90),
                charge_cycles: s.cp_charge_cycles.get_clone().trim().parse().unwrap_or(u16::MAX),
                ride_current: s.cp_ride_current.get_clone().trim().parse().unwrap_or(255),
                controller_temperature: s.cp_controller_temperature.get_clone().trim().parse().unwrap_or(255),
                voltage: s.cp_voltage.get_clone().trim().parse().unwrap_or(100),
                total_mileage: s.cp_total_mileage.get_clone().trim().parse().unwrap_or(10000),
                trip_mileage: s.cp_trip_mileage.get_clone().trim().parse().unwrap_or(5000),
                bms_remain_capacity: s.cp_bms_remain_capacity.get_clone().trim().parse().unwrap_or(4000),
                bms_full_capacity: s.cp_bms_full_capacity.get_clone().trim().parse().unwrap_or(5000),
                bms_soh: s.cp_bms_soh.get_clone().trim().parse().unwrap_or(85),
                bms_temperature: s.cp_bms_temperature.get_clone().trim().parse().unwrap_or(20),
                bms_current: s.cp_bms_current.get_clone().trim().parse().unwrap_or(90),
                bms_voltage: s.cp_bms_voltage.get_clone().trim().parse().unwrap_or(18500),
                bms_cycle_count: s.cp_bms_cycle_count.get_clone().trim().parse().unwrap_or(40),
                bms_cell_voltages: s.cp_bms_cell_voltages.get_clone().trim().to_string(),
                bms_charge_interval: s.cp_bms_charge_interval.get_clone().trim().parse().unwrap_or(u16::MAX),
                bms_max_charge_interval: s.cp_bms_max_charge_interval.get_clone().trim().parse().unwrap_or(u16::MAX),
                bms_version: s.cp_bms_version.get_clone().trim().parse().unwrap_or(u16::MAX),
                bms_manufacturer: s.cp_bms_manufacturer.get_clone().trim().to_string(),
            },
        }
    }

    pub fn view(self) -> View {
        let s = self;
        view! {
            div {
            div(class="section") {
                div(class="section-title") { "网关地址" }
                div(class="grid grid-3") {
                    div(class="field") {
                        label { "内网网关地址" }
                        input(r#type="text", placeholder="bike-seb-inner-test.costrip.cn:32405", bind:value=s.default_inner_gw)
                    }
                    div(class="field") {
                        label { "外网测试网关地址" }
                        input(r#type="text", placeholder="bike-seb-test.costrip.cn:8514", bind:value=s.default_test_gw)
                    }
                    div(class="field") {
                        label { "正式网关地址" }
                        input(r#type="text", placeholder="bike-seb.costrip.cn:8514", bind:value=s.default_prod_gw)
                    }
                }
            }

            div(class="section") {
                div(class="section-title") { "设备默认参数" }
                div(class="field", style="margin-bottom:12px") {
                    label { "车辆二维码 URL 模板 (支持 {bike_no} 占位符)" }
                    input(r#type="text", placeholder="https://gycx.cn?s={bike_no}", bind:value=s.qr_url_template)
                }
                div(class="grid grid-4") {
                    div(class="field") {
                        label { "软件版本号" }
                        input(r#type="text", placeholder="1.0.0", bind:value=s.default_soft_version)
                    }
                    div(class="field") {
                        label { "默认地理坐标" }
                        input(r#type="text", placeholder="108.38,22.77", bind:value=s.default_coordinates)
                    }
                    div(class="field") {
                        label { "默认电量 SOC" }
                        input(r#type="text", placeholder="98", bind:value=s.default_soc)
                    }
                    div(class="field") {
                        label { "默认车速 (km/h)" }
                        input(r#type="text", placeholder="0", bind:value=s.default_speed)
                    }
                    div(class="field") {
                        label { "默认偏转角度 (°)" }
                        input(r#type="text", placeholder="0", bind:value=s.default_deflection_angle)
                    }
                    div(class="field") {
                        label { "默认心跳间隔 (秒)" }
                        input(r#type="text", placeholder="60", bind:value=s.default_heartbeat_interval)
                    }
                }
            }

            div(class="section") {
                div(class="section-title") { "位置上报取值" }
                div(class="grid grid-4") {
                    div(class="field") {
                        label { "海拔高度 (m)" }
                        input(r#type="text", placeholder="90", bind:value=s.cp_altitude)
                    }
                    div(class="field") {
                        label { "方向 (°/2)" }
                        input(r#type="text", placeholder="25", bind:value=s.cp_heading)
                    }
                    div(class="field") {
                        label { "GPS 速度 (km/h)" }
                        input(r#type="text", placeholder="20", bind:value=s.cp_gps_speed)
                    }
                    div(class="field") {
                        label { "GPS 定位状态 (0~3)" }
                        input(r#type="text", placeholder="2", bind:value=s.cp_gps_status)
                    }
                    div(class="field") {
                        label { "锁定卫星数 (0~15)" }
                        input(r#type="text", placeholder="9", bind:value=s.cp_satellites)
                    }
                    div(class="field") {
                        label { "备用电池电量 (0~15)" }
                        input(r#type="text", placeholder="9", bind:value=s.cp_backup_battery)
                    }
                    div(class="field") {
                        label { "GSM 信号强度 (0~15)" }
                        input(r#type="text", placeholder="9", bind:value=s.cp_gsm_signal)
                    }
                    div(class="field") {
                        label { "移动国家代码 MCC" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_mcc)
                    }
                    div(class="field") {
                        label { "移动网络代码 MNC" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_mnc)
                    }
                    div(class="field") {
                        label { "位置区域码 LAC" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_lac)
                    }
                    div(class="field") {
                        label { "小区标识符" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_cell_id)
                    }
                    div(class="field") {
                        label { "可用剩余容量 (mAh)" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_available_capacity)
                    }
                    div(class="field") {
                        label { "健康状态 SOH (%)" }
                        input(r#type="text", placeholder="90", bind:value=s.cp_soh)
                    }
                    div(class="field") {
                        label { "充放电次数" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_charge_cycles)
                    }
                    div(class="field") {
                        label { "骑行电流" }
                        input(r#type="text", placeholder="255", bind:value=s.cp_ride_current)
                    }
                    div(class="field") {
                        label { "控制器温度" }
                        input(r#type="text", placeholder="255", bind:value=s.cp_controller_temperature)
                    }
                    div(class="field") {
                        label { "电瓶电压 (0.1V)" }
                        input(r#type="text", placeholder="100", bind:value=s.cp_voltage)
                    }
                    div(class="field") {
                        label { "总里程 (m)" }
                        input(r#type="text", placeholder="10000", bind:value=s.cp_total_mileage)
                    }
                    div(class="field") {
                        label { "单次里程 (m)" }
                        input(r#type="text", placeholder="5000", bind:value=s.cp_trip_mileage)
                    }
                }
            }

            div(class="section") {
                div(class="section-title") { "电池状态上报取值" }
                div(class="grid grid-4") {
                    div(class="field") {
                        label { "剩余容量 (mAh)" }
                        input(r#type="text", placeholder="4000", bind:value=s.cp_bms_remain_capacity)
                    }
                    div(class="field") {
                        label { "绝对满电容量 (mAh)" }
                        input(r#type="text", placeholder="5000", bind:value=s.cp_bms_full_capacity)
                    }
                    div(class="field") {
                        label { "健康状态 SOH (%)" }
                        input(r#type="text", placeholder="85", bind:value=s.cp_bms_soh)
                    }
                    div(class="field") {
                        label { "内部温度" }
                        input(r#type="text", placeholder="20", bind:value=s.cp_bms_temperature)
                    }
                    div(class="field") {
                        label { "实时电流 (mA)" }
                        input(r#type="text", placeholder="90", bind:value=s.cp_bms_current)
                    }
                    div(class="field") {
                        label { "电压 (mV)" }
                        input(r#type="text", placeholder="18500", bind:value=s.cp_bms_voltage)
                    }
                    div(class="field") {
                        label { "循环次数" }
                        input(r#type="text", placeholder="40", bind:value=s.cp_bms_cycle_count)
                    }
                    div(class="field") {
                        label { "充电间隔时间" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_bms_charge_interval)
                    }
                    div(class="field") {
                        label { "最大充电间隔时间" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_bms_max_charge_interval)
                    }
                    div(class="field") {
                        label { "BMS 版本号" }
                        input(r#type="text", placeholder="65535", bind:value=s.cp_bms_version)
                    }
                    div(class="field") {
                        label { "电池制造商 (16 位)" }
                        input(r#type="text", placeholder="1111111111111111", bind:value=s.cp_bms_manufacturer)
                    }
                }
                div(class="field", style="margin-top:12px") {
                    label { "单体电压 mV (逗号分隔，取前 14 节，不足补 0)" }
                    input(r#type="text", placeholder="4000,4100,4200", bind:value=s.cp_bms_cell_voltages)
                }
            }
            }
        }
    }
}
