use crate::state::ControlGlobalSettings;
use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub struct ControlSection {
    exchange_control: Signal<String>,
    exchange_query: Signal<String>,
    exchange_set: Signal<String>,
    exchange_voice: Signal<String>,
    flink_high_url: Signal<String>,
    flink_iot_url: Signal<String>,
}

impl ControlSection {
    pub fn new() -> Self {
        Self {
            exchange_control: create_signal(String::new()),
            exchange_query: create_signal(String::new()),
            exchange_set: create_signal(String::new()),
            exchange_voice: create_signal(String::new()),
            flink_high_url: create_signal(String::new()),
            flink_iot_url: create_signal(String::new()),
        }
    }

    pub fn load(self, c: ControlGlobalSettings) {
        let s = self;
        s.exchange_control.set(c.exchange_control);
        s.exchange_query.set(c.exchange_query);
        s.exchange_set.set(c.exchange_set);
        s.exchange_voice.set(c.exchange_voice);
        s.flink_high_url.set(c.flink_high_url);
        s.flink_iot_url.set(c.flink_iot_url);
    }

    pub fn collect(self) -> ControlGlobalSettings {
        let s = self;
        ControlGlobalSettings {
            exchange_control: s.exchange_control.get_clone().trim().to_string(),
            exchange_query: s.exchange_query.get_clone().trim().to_string(),
            exchange_set: s.exchange_set.get_clone().trim().to_string(),
            exchange_voice: s.exchange_voice.get_clone().trim().to_string(),
            flink_high_url: s.flink_high_url.get_clone().trim().to_string(),
            flink_iot_url: s.flink_iot_url.get_clone().trim().to_string(),
        }
    }

    pub fn view(self) -> View {
        let s = self;
        view! {
            div {
                div(class="section") {
                    div(class="section-title") { "指令路由与消息队列配置" }
                    div(class="grid grid-4") {
                        div(class="field") {
                            label { "控制命令 Exchange" }
                            input(r#type="text", placeholder="seb.command.test", bind:value=s.exchange_control)
                        }
                        div(class="field") {
                            label { "参数查询 Exchange" }
                            input(r#type="text", placeholder="seb.query.command.test", bind:value=s.exchange_query)
                        }
                        div(class="field") {
                            label { "参数设置 Exchange" }
                            input(r#type="text", placeholder="seb.set.command.test", bind:value=s.exchange_set)
                        }
                        div(class="field") {
                            label { "语音播报 Exchange" }
                            input(r#type="text", placeholder="seb.voice.command.test", bind:value=s.exchange_voice)
                        }
                    }
                }

                div(class="section") {
                    div(class="section-title") { "Flink 监控看板 URL" }
                    div(class="grid grid-2") {
                        div(class="field") {
                            label { "在线/心跳看板 URL" }
                            input(r#type="text", placeholder="http://10.12.55.240/flink-operator/seb-flink-bike-iot-high/#/overview", bind:value=s.flink_high_url)
                        }
                        div(class="field") {
                            label { "定位/遥测看板 URL" }
                            input(r#type="text", placeholder="http://10.12.55.240/flink-operator/seb-flink-bike-iot/#/overview", bind:value=s.flink_iot_url)
                        }
                    }
                }
            }
        }
    }
}
