use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DeployDefaults {
    pub bike_type_id: i64,
    pub supplier_id: i64,
    pub dealer_id: i64,
    pub device_company_id: i64,
    pub battery_type_id: i64,
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
            battery_type_id: 0,
            has_helmet: true,
            has_trunk: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientGlobalSettings {
    pub qr_url_template: String,
    pub default_inner_gw: String,
    pub default_test_gw: String,
    pub default_prod_gw: String,
    pub default_soft_version: String,
    pub default_coordinates: String,
    pub default_soc: u8,
    pub default_speed: u8,
    pub default_deflection_angle: u16,
    pub default_heartbeat_interval: u64,
    pub payload: ClientPayloadSettings,
}

impl Default for ClientGlobalSettings {
    fn default() -> Self {
        Self {
            qr_url_template: "https://gycx.cn?s={bike_no}".to_string(),
            default_inner_gw: "bike-seb-inner-test.costrip.cn:32405".to_string(),
            default_test_gw: "bike-seb-test.costrip.cn:8514".to_string(),
            default_prod_gw: "bike-seb.costrip.cn:8514".to_string(),
            default_soft_version: "1.0.0".to_string(),
            default_coordinates: "108.38,22.77".to_string(),
            default_soc: 98,
            default_speed: 0,
            default_deflection_angle: 0,
            default_heartbeat_interval: 60,
            payload: ClientPayloadSettings::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientPayloadSettings {
    pub altitude: u16,
    pub heading: u8,
    pub gps_speed: u8,
    pub gps_status: u16,
    pub satellites: u16,
    pub backup_battery: u16,
    pub gsm_signal: u16,
    pub mcc: u16,
    pub mnc: u16,
    pub lac: u16,
    pub cell_id: u16,
    pub available_capacity: u16,
    pub soh: u8,
    pub charge_cycles: u16,
    pub ride_current: u8,
    pub controller_temperature: u8,
    pub voltage: u16,
    pub total_mileage: u32,
    pub trip_mileage: u32,

    pub bms_remain_capacity: u16,
    pub bms_full_capacity: u16,
    pub bms_soh: u8,
    pub bms_temperature: u16,
    pub bms_current: u16,
    pub bms_voltage: u16,
    pub bms_cycle_count: u16,
    pub bms_cell_voltages: String,
    pub bms_charge_interval: u16,
    pub bms_max_charge_interval: u16,
    pub bms_version: u16,
    pub bms_manufacturer: String,
}

impl Default for ClientPayloadSettings {
    fn default() -> Self {
        Self {
            altitude: 90,
            heading: 25,
            gps_speed: 20,
            gps_status: 2,
            satellites: 9,
            backup_battery: 9,
            gsm_signal: 9,
            mcc: u16::MAX,
            mnc: u16::MAX,
            lac: u16::MAX,
            cell_id: u16::MAX,
            available_capacity: u16::MAX,
            soh: 90,
            charge_cycles: u16::MAX,
            ride_current: 255,
            controller_temperature: 255,
            voltage: 100,
            total_mileage: 10000,
            trip_mileage: 5000,

            bms_remain_capacity: 4000,
            bms_full_capacity: 5000,
            bms_soh: 85,
            bms_temperature: 20,
            bms_current: 90,
            bms_voltage: 18500,
            bms_cycle_count: 40,
            bms_cell_voltages: (0..14u16)
                .map(|i| (4000 + i * 100).to_string())
                .collect::<Vec<_>>()
                .join(","),
            bms_charge_interval: u16::MAX,
            bms_max_charge_interval: u16::MAX,
            bms_version: u16::MAX,
            bms_manufacturer: "1111111111111111".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BatteryGlobalSettings {
    pub qr_url_template: String,
    pub default_inner_gw: String,
    pub default_test_gw: String,
    pub default_prod_gw: String,
    pub default_iccid: String,
    pub default_coordinates: String,
    pub default_hw_version: String,
    pub default_sw_version: String,
    pub default_heartbeat_interval: u64,
    pub payload: BatteryPayloadSettings,
}

/// 与 seb-core config::BatteryPayloadSettings 对应
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BatteryPayloadSettings {
    pub speed: u16,
    pub azimuth: u16,

    pub total_voltage: u16,
    pub current: u16,
    pub battery_status: u8,
    pub cell_voltages: String,
    pub battery_temperatures: String,
    pub heat_film_temperatures: String,
    pub environment_temperatures: String,
    pub mos_temperatures: String,
    pub max_cell_voltage: u16,
    pub min_cell_voltage: u16,
    pub avg_cell_voltage: u16,
    pub max_battery_temperature: u8,
    pub min_battery_temperature: u8,
    pub total_discharge_capacity: u32,
    pub rated_capacity: u16,
    pub remain_capacity: u16,
    pub soc: u8,
    pub status_info: u8,
    pub cycle_count: u16,
    pub max_cell_no: u8,
    pub min_cell_no: u8,
    pub temperature_no: u8,

    pub alarm_byte_one: u8,
    pub alarm_byte_two: u8,
    pub alarm_byte_three: u8,
    pub alarm_byte_four: u8,
    pub alarm_fault_code: u16,
}

impl Default for BatteryPayloadSettings {
    fn default() -> Self {
        Self {
            speed: 137,
            azimuth: 23971,

            total_voltage: 5500,
            current: 2300,
            battery_status: 1,
            cell_voltages: "3000,2000,1000".to_string(),
            battery_temperatures: "75".to_string(),
            heat_film_temperatures: "65".to_string(),
            environment_temperatures: "44".to_string(),
            mos_temperatures: String::new(),
            max_cell_voltage: 3000,
            min_cell_voltage: 1000,
            avg_cell_voltage: 2000,
            max_battery_temperature: 75,
            min_battery_temperature: 65,
            total_discharge_capacity: 2500,
            rated_capacity: 800,
            remain_capacity: 200,
            soc: 100,
            status_info: 0x05,
            cycle_count: 10,
            max_cell_no: 2,
            min_cell_no: 4,
            temperature_no: 34,

            alarm_byte_one: 0x00,
            alarm_byte_two: 0x00,
            alarm_byte_three: 0x01,
            alarm_byte_four: 0x05,
            alarm_fault_code: 0,
        }
    }
}

impl Default for BatteryGlobalSettings {
    fn default() -> Self {
        Self {
            qr_url_template: "https://cosbike.net.cn/qr?{battery_no}".to_string(),
            default_inner_gw: "10.12.55.31:32402".to_string(),
            default_test_gw: "140.143.180.28:28081".to_string(),
            default_prod_gw: "140.143.214.51:28081".to_string(),
            default_iccid: "89860409081870640660".to_string(),
            default_coordinates: "108.38,22.77".to_string(),
            default_hw_version: "2.1.3".to_string(),
            default_sw_version: "3.2.1".to_string(),
            default_heartbeat_interval: 60,
            payload: BatteryPayloadSettings::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ControlGlobalSettings {
    pub exchange_control: String,
    pub exchange_query: String,
    pub exchange_set: String,
    pub exchange_voice: String,
    pub flink_high_url: String,
    pub flink_iot_url: String,
}

impl Default for ControlGlobalSettings {
    fn default() -> Self {
        Self {
            exchange_control: "seb.command.test".to_string(),
            exchange_query: "seb.query.command.test".to_string(),
            exchange_set: "seb.set.command.test".to_string(),
            exchange_voice: "seb.voice.command.test".to_string(),
            flink_high_url: "http://10.12.55.240/flink-operator/seb-flink-bike-iot-high/#/overview".to_string(),
            flink_iot_url: "http://10.12.55.240/flink-operator/seb-flink-bike-iot/#/overview".to_string(),
        }
    }
}


const DEFAULT_HOSTS_INNER: &str = include_str!("../../../../seb-core/src/hosts/inner.hosts");
const DEFAULT_HOSTS_UAT: &str = include_str!("../../../../seb-core/src/hosts/uat.hosts");

/// 三击百宝箱触发什么
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TripleClickAction {
    #[default]
    Egg,
    Hosts,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GlobalSettings {
    pub client: ClientGlobalSettings,
    pub battery: BatteryGlobalSettings,
    pub control: ControlGlobalSettings,
    /// 鸡蛋框：一行一个地址，敲开时随机取一个。能当图片加载的直接展示，其余交给浏览器打开
    pub egg_url: String,
    pub triple_click: TripleClickAction,
    pub hosts_inner: String,
    pub hosts_uat: String,
    pub hosts_prod: String,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            client: ClientGlobalSettings::default(),
            battery: BatteryGlobalSettings::default(),
            control: ControlGlobalSettings::default(),
            egg_url: String::new(),
            triple_click: TripleClickAction::default(),
            hosts_inner: DEFAULT_HOSTS_INNER.to_string(),
            hosts_uat: DEFAULT_HOSTS_UAT.to_string(),
            hosts_prod: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub instance: String,
    pub device_no: String,
    pub bike_no: String,
    pub battery_no: String,
    pub city_id: i64,
    pub last_seen_version: String,
    pub deploy: DeployDefaults,
    pub settings: GlobalSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            instance: "0".into(),
            device_no: String::new(),
            bike_no: String::new(),
            battery_no: String::new(),
            city_id: 0,
            last_seen_version: String::new(),
            deploy: DeployDefaults::default(),
            settings: GlobalSettings::default(),
        }
    }
}
