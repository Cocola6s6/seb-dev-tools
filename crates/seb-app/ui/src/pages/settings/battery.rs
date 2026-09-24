use crate::state::{BatteryGlobalSettings, BatteryPayloadSettings};
use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub struct BatterySection {
    qr_url_template: Signal<String>,
    default_inner_gw: Signal<String>,
    default_test_gw: Signal<String>,
    default_prod_gw: Signal<String>,
    default_iccid: Signal<String>,
    default_coordinates: Signal<String>,
    default_hw_version: Signal<String>,
    default_sw_version: Signal<String>,
    default_heartbeat_interval: Signal<String>,
    bp_speed: Signal<String>,
    bp_azimuth: Signal<String>,
    bp_total_voltage: Signal<String>,
    bp_current: Signal<String>,
    bp_battery_status: Signal<String>,
    bp_cell_voltages: Signal<String>,
    bp_battery_temperatures: Signal<String>,
    bp_heat_film_temperatures: Signal<String>,
    bp_environment_temperatures: Signal<String>,
    bp_mos_temperatures: Signal<String>,
    bp_max_cell_voltage: Signal<String>,
    bp_min_cell_voltage: Signal<String>,
    bp_avg_cell_voltage: Signal<String>,
    bp_max_battery_temperature: Signal<String>,
    bp_min_battery_temperature: Signal<String>,
    bp_total_discharge_capacity: Signal<String>,
    bp_rated_capacity: Signal<String>,
    bp_remain_capacity: Signal<String>,
    bp_soc: Signal<String>,
    bp_status_info: Signal<String>,
    bp_cycle_count: Signal<String>,
    bp_max_cell_no: Signal<String>,
    bp_min_cell_no: Signal<String>,
    bp_temperature_no: Signal<String>,
    bp_alarm_byte_one: Signal<String>,
    bp_alarm_byte_two: Signal<String>,
    bp_alarm_byte_three: Signal<String>,
    bp_alarm_byte_four: Signal<String>,
    bp_alarm_fault_code: Signal<String>,
}

impl BatterySection {
    pub fn new() -> Self {
        Self {
            qr_url_template: create_signal(String::new()),
            default_inner_gw: create_signal(String::new()),
            default_test_gw: create_signal(String::new()),
            default_prod_gw: create_signal(String::new()),
            default_iccid: create_signal(String::new()),
            default_coordinates: create_signal(String::new()),
            default_hw_version: create_signal(String::new()),
            default_sw_version: create_signal(String::new()),
            default_heartbeat_interval: create_signal(String::new()),
            bp_speed: create_signal(String::new()),
            bp_azimuth: create_signal(String::new()),
            bp_total_voltage: create_signal(String::new()),
            bp_current: create_signal(String::new()),
            bp_battery_status: create_signal(String::new()),
            bp_cell_voltages: create_signal(String::new()),
            bp_battery_temperatures: create_signal(String::new()),
            bp_heat_film_temperatures: create_signal(String::new()),
            bp_environment_temperatures: create_signal(String::new()),
            bp_mos_temperatures: create_signal(String::new()),
            bp_max_cell_voltage: create_signal(String::new()),
            bp_min_cell_voltage: create_signal(String::new()),
            bp_avg_cell_voltage: create_signal(String::new()),
            bp_max_battery_temperature: create_signal(String::new()),
            bp_min_battery_temperature: create_signal(String::new()),
            bp_total_discharge_capacity: create_signal(String::new()),
            bp_rated_capacity: create_signal(String::new()),
            bp_remain_capacity: create_signal(String::new()),
            bp_soc: create_signal(String::new()),
            bp_status_info: create_signal(String::new()),
            bp_cycle_count: create_signal(String::new()),
            bp_max_cell_no: create_signal(String::new()),
            bp_min_cell_no: create_signal(String::new()),
            bp_temperature_no: create_signal(String::new()),
            bp_alarm_byte_one: create_signal(String::new()),
            bp_alarm_byte_two: create_signal(String::new()),
            bp_alarm_byte_three: create_signal(String::new()),
            bp_alarm_byte_four: create_signal(String::new()),
            bp_alarm_fault_code: create_signal(String::new()),
        }
    }

    pub fn load(self, b: BatteryGlobalSettings) {
        let s = self;
        s.qr_url_template.set(b.qr_url_template);
        s.default_inner_gw.set(b.default_inner_gw);
        s.default_test_gw.set(b.default_test_gw);
        s.default_prod_gw.set(b.default_prod_gw);
        s.default_iccid.set(b.default_iccid);
        s.default_coordinates.set(b.default_coordinates);
        s.default_hw_version.set(b.default_hw_version);
        s.default_sw_version.set(b.default_sw_version);
        s.default_heartbeat_interval.set(b.default_heartbeat_interval.to_string());

        let p = b.payload;
        s.bp_speed.set(p.speed.to_string());
        s.bp_azimuth.set(p.azimuth.to_string());
        s.bp_total_voltage.set(p.total_voltage.to_string());
        s.bp_current.set(p.current.to_string());
        s.bp_battery_status.set(p.battery_status.to_string());
        s.bp_cell_voltages.set(p.cell_voltages);
        s.bp_battery_temperatures.set(p.battery_temperatures);
        s.bp_heat_film_temperatures.set(p.heat_film_temperatures);
        s.bp_environment_temperatures.set(p.environment_temperatures);
        s.bp_mos_temperatures.set(p.mos_temperatures);
        s.bp_max_cell_voltage.set(p.max_cell_voltage.to_string());
        s.bp_min_cell_voltage.set(p.min_cell_voltage.to_string());
        s.bp_avg_cell_voltage.set(p.avg_cell_voltage.to_string());
        s.bp_max_battery_temperature.set(p.max_battery_temperature.to_string());
        s.bp_min_battery_temperature.set(p.min_battery_temperature.to_string());
        s.bp_total_discharge_capacity.set(p.total_discharge_capacity.to_string());
        s.bp_rated_capacity.set(p.rated_capacity.to_string());
        s.bp_remain_capacity.set(p.remain_capacity.to_string());
        s.bp_soc.set(p.soc.to_string());
        s.bp_status_info.set(p.status_info.to_string());
        s.bp_cycle_count.set(p.cycle_count.to_string());
        s.bp_max_cell_no.set(p.max_cell_no.to_string());
        s.bp_min_cell_no.set(p.min_cell_no.to_string());
        s.bp_temperature_no.set(p.temperature_no.to_string());
        s.bp_alarm_byte_one.set(p.alarm_byte_one.to_string());
        s.bp_alarm_byte_two.set(p.alarm_byte_two.to_string());
        s.bp_alarm_byte_three.set(p.alarm_byte_three.to_string());
        s.bp_alarm_byte_four.set(p.alarm_byte_four.to_string());
        s.bp_alarm_fault_code.set(p.alarm_fault_code.to_string());
    }

    pub fn collect(self) -> BatteryGlobalSettings {
        let s = self;
        BatteryGlobalSettings {
            qr_url_template: s.qr_url_template.get_clone().trim().to_string(),
            default_inner_gw: s.default_inner_gw.get_clone().trim().to_string(),
            default_test_gw: s.default_test_gw.get_clone().trim().to_string(),
            default_prod_gw: s.default_prod_gw.get_clone().trim().to_string(),
            default_iccid: s.default_iccid.get_clone().trim().to_string(),
            default_coordinates: s.default_coordinates.get_clone().trim().to_string(),
            default_hw_version: s.default_hw_version.get_clone().trim().to_string(),
            default_sw_version: s.default_sw_version.get_clone().trim().to_string(),
            default_heartbeat_interval: s.default_heartbeat_interval.get_clone().trim().parse().unwrap_or(60),
            payload: BatteryPayloadSettings {
                speed: s.bp_speed.get_clone().trim().parse().unwrap_or(137),
                azimuth: s.bp_azimuth.get_clone().trim().parse().unwrap_or(23971),
                total_voltage: s.bp_total_voltage.get_clone().trim().parse().unwrap_or(5500),
                current: s.bp_current.get_clone().trim().parse().unwrap_or(2300),
                battery_status: s.bp_battery_status.get_clone().trim().parse().unwrap_or(1),
                cell_voltages: s.bp_cell_voltages.get_clone().trim().to_string(),
                battery_temperatures: s.bp_battery_temperatures.get_clone().trim().to_string(),
                heat_film_temperatures: s.bp_heat_film_temperatures.get_clone().trim().to_string(),
                environment_temperatures: s.bp_environment_temperatures.get_clone().trim().to_string(),
                mos_temperatures: s.bp_mos_temperatures.get_clone().trim().to_string(),
                max_cell_voltage: s.bp_max_cell_voltage.get_clone().trim().parse().unwrap_or(3000),
                min_cell_voltage: s.bp_min_cell_voltage.get_clone().trim().parse().unwrap_or(1000),
                avg_cell_voltage: s.bp_avg_cell_voltage.get_clone().trim().parse().unwrap_or(2000),
                max_battery_temperature: s.bp_max_battery_temperature.get_clone().trim().parse().unwrap_or(75),
                min_battery_temperature: s.bp_min_battery_temperature.get_clone().trim().parse().unwrap_or(65),
                total_discharge_capacity: s.bp_total_discharge_capacity.get_clone().trim().parse().unwrap_or(2500),
                rated_capacity: s.bp_rated_capacity.get_clone().trim().parse().unwrap_or(800),
                remain_capacity: s.bp_remain_capacity.get_clone().trim().parse().unwrap_or(200),
                soc: s.bp_soc.get_clone().trim().parse().unwrap_or(100),
                status_info: s.bp_status_info.get_clone().trim().parse().unwrap_or(5),
                cycle_count: s.bp_cycle_count.get_clone().trim().parse().unwrap_or(10),
                max_cell_no: s.bp_max_cell_no.get_clone().trim().parse().unwrap_or(2),
                min_cell_no: s.bp_min_cell_no.get_clone().trim().parse().unwrap_or(4),
                temperature_no: s.bp_temperature_no.get_clone().trim().parse().unwrap_or(34),
                alarm_byte_one: s.bp_alarm_byte_one.get_clone().trim().parse().unwrap_or(0),
                alarm_byte_two: s.bp_alarm_byte_two.get_clone().trim().parse().unwrap_or(0),
                alarm_byte_three: s.bp_alarm_byte_three.get_clone().trim().parse().unwrap_or(1),
                alarm_byte_four: s.bp_alarm_byte_four.get_clone().trim().parse().unwrap_or(5),
                alarm_fault_code: s.bp_alarm_fault_code.get_clone().trim().parse().unwrap_or(0),
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
                        input(r#type="text", placeholder="10.12.55.31:32402", bind:value=s.default_inner_gw)
                    }
                    div(class="field") {
                        label { "外网测试网关地址" }
                        input(r#type="text", placeholder="140.143.180.28:28081", bind:value=s.default_test_gw)
                    }
                    div(class="field") {
                        label { "正式网关地址" }
                        input(r#type="text", placeholder="140.143.214.51:28081", bind:value=s.default_prod_gw)
                    }
                }
            }

            div(class="section") {
                div(class="section-title") { "设备默认参数" }
                div(class="field", style="margin-bottom:12px") {
                    label { "电池二维码 URL 模板 (支持 {battery_no} 占位符)" }
                    input(r#type="text", placeholder="https://cosbike.net.cn/qr?{battery_no}", bind:value=s.qr_url_template)
                }
                div(class="grid grid-4") {
                    div(class="field") {
                        label { "默认 ICCID" }
                        input(r#type="text", placeholder="89860409081870640660", bind:value=s.default_iccid)
                    }
                    div(class="field") {
                        label { "默认地理坐标" }
                        input(r#type="text", placeholder="108.375256,22.767133", bind:value=s.default_coordinates)
                    }
                    div(class="field") {
                        label { "默认硬件版本" }
                        input(r#type="text", placeholder="2.1.3", bind:value=s.default_hw_version)
                    }
                    div(class="field") {
                        label { "默认软件版本" }
                        input(r#type="text", placeholder="3.2.1", bind:value=s.default_sw_version)
                    }
                    div(class="field") {
                        label { "默认心跳间隔 (秒)" }
                        input(r#type="text", placeholder="60", bind:value=s.default_heartbeat_interval)
                    }
                }
            }

            div(class="section") {
                div(class="section-title") { "定位上报取值" }
                div(class="grid grid-4") {
                        div(class="field") {
                            label { "定位速度 (speed)" }
                            input(r#type="text", placeholder="137", bind:value=s.bp_speed)
                        }
                        div(class="field") {
                            label { "定位方位角 (azimuth)" }
                            input(r#type="text", placeholder="23971", bind:value=s.bp_azimuth)
                        }
                }
            }

            div(class="section") {
                div(class="section-title") { "遥测上报取值" }
                div(class="grid grid-4") {
                        div(class="field") {
                            label { "电池总电压" }
                            input(r#type="text", placeholder="5500", bind:value=s.bp_total_voltage)
                        }
                        div(class="field") {
                            label { "电池电流" }
                            input(r#type="text", placeholder="2300", bind:value=s.bp_current)
                        }
                        div(class="field") {
                            label { "电池状态 (1=闭合)" }
                            input(r#type="text", placeholder="1", bind:value=s.bp_battery_status)
                        }
                        div(class="field") {
                            label { "电芯电压 (逗号分隔)" }
                            input(r#type="text", placeholder="3000,2000,1000", bind:value=s.bp_cell_voltages)
                        }
                        div(class="field") {
                            label { "电池温度 (逗号分隔)" }
                            input(r#type="text", placeholder="75", bind:value=s.bp_battery_temperatures)
                        }
                        div(class="field") {
                            label { "加热膜温度 (逗号分隔)" }
                            input(r#type="text", placeholder="65", bind:value=s.bp_heat_film_temperatures)
                        }
                        div(class="field") {
                            label { "环境温度 (逗号分隔)" }
                            input(r#type="text", placeholder="44", bind:value=s.bp_environment_temperatures)
                        }
                        div(class="field") {
                            label { "MOS 温度 (逗号分隔，可空)" }
                            input(r#type="text", placeholder="留空", bind:value=s.bp_mos_temperatures)
                        }
                        div(class="field") {
                            label { "最高单体电压" }
                            input(r#type="text", placeholder="3000", bind:value=s.bp_max_cell_voltage)
                        }
                        div(class="field") {
                            label { "最低单体电压" }
                            input(r#type="text", placeholder="1000", bind:value=s.bp_min_cell_voltage)
                        }
                        div(class="field") {
                            label { "平均单体电压" }
                            input(r#type="text", placeholder="2000", bind:value=s.bp_avg_cell_voltage)
                        }
                        div(class="field") {
                            label { "最高电池温度" }
                            input(r#type="text", placeholder="75", bind:value=s.bp_max_battery_temperature)
                        }
                        div(class="field") {
                            label { "最低电池温度" }
                            input(r#type="text", placeholder="65", bind:value=s.bp_min_battery_temperature)
                        }
                        div(class="field") {
                            label { "累计放电容量" }
                            input(r#type="text", placeholder="2500", bind:value=s.bp_total_discharge_capacity)
                        }
                        div(class="field") {
                            label { "额定容量" }
                            input(r#type="text", placeholder="800", bind:value=s.bp_rated_capacity)
                        }
                        div(class="field") {
                            label { "剩余容量" }
                            input(r#type="text", placeholder="200", bind:value=s.bp_remain_capacity)
                        }
                        div(class="field") {
                            label { "SOC (%)" }
                            input(r#type="text", placeholder="100", bind:value=s.bp_soc)
                        }
                        div(class="field") {
                            label { "状态字节 statusInfo" }
                            input(r#type="text", placeholder="5", bind:value=s.bp_status_info)
                        }
                        div(class="field") {
                            label { "循环次数" }
                            input(r#type="text", placeholder="10", bind:value=s.bp_cycle_count)
                        }
                        div(class="field") {
                            label { "最高单体电压编号" }
                            input(r#type="text", placeholder="2", bind:value=s.bp_max_cell_no)
                        }
                        div(class="field") {
                            label { "最低单体电压编号" }
                            input(r#type="text", placeholder="4", bind:value=s.bp_min_cell_no)
                        }
                        div(class="field") {
                            label { "最高/最低电池温度编号" }
                            input(r#type="text", placeholder="34", bind:value=s.bp_temperature_no)
                        }
                }
            }

            div(class="section") {
                div(class="section-title") { "告警上报取值" }
                div(class="grid grid-4") {
                        div(class="field") {
                            label { "告警字节一 (过压/欠压/过流)" }
                            input(r#type="text", placeholder="0", bind:value=s.bp_alarm_byte_one)
                        }
                        div(class="field") {
                            label { "告警字节二 (短路/超时/低温)" }
                            input(r#type="text", placeholder="0", bind:value=s.bp_alarm_byte_two)
                        }
                        div(class="field") {
                            label { "告警字节三 (加热状态)" }
                            input(r#type="text", placeholder="1", bind:value=s.bp_alarm_byte_three)
                        }
                        div(class="field") {
                            label { "告警字节四 (MOS 状态)" }
                            input(r#type="text", placeholder="5", bind:value=s.bp_alarm_byte_four)
                        }
                        div(class="field") {
                            label { "故障码" }
                            input(r#type="text", placeholder="0", bind:value=s.bp_alarm_fault_code)
                        }
                }
            }
            }
        }
    }
}
