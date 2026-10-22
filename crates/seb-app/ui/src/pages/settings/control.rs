use crate::state::{ControlGlobalSettings, JswSettings, MysqlConfig};
use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub struct ControlSection {
    exchange_control: Signal<String>,
    exchange_query: Signal<String>,
    exchange_set: Signal<String>,
    exchange_voice: Signal<String>,
    flink_high_url: Signal<String>,
    flink_iot_url: Signal<String>,

    // MySQL 直连配置（用于内网环境）
    mysql_host: Signal<String>,
    mysql_port: Signal<String>,
    mysql_username: Signal<String>,
    mysql_password: Signal<String>,
    mysql_database: Signal<String>,

    // JSW 配置（用于外网和正式环境）
    jsw_base_url: Signal<String>,
    jsw_token: Signal<String>,
    jsw_server_code: Signal<String>,
    jsw_database: Signal<String>,
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

            mysql_host: create_signal(String::new()),
            mysql_port: create_signal(String::new()),
            mysql_username: create_signal(String::new()),
            mysql_password: create_signal(String::new()),
            mysql_database: create_signal(String::new()),

            jsw_base_url: create_signal(String::new()),
            jsw_token: create_signal(String::new()),
            jsw_server_code: create_signal(String::new()),
            jsw_database: create_signal(String::new()),
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

        s.mysql_host.set(c.mysql.host);
        s.mysql_port.set(c.mysql.port.to_string());
        s.mysql_username.set(c.mysql.username);
        s.mysql_password.set(c.mysql.password);
        s.mysql_database.set(c.mysql.database);

        s.jsw_base_url.set(c.jsw.base_url);
        s.jsw_token.set(c.jsw.token);
        s.jsw_server_code.set(c.jsw.server_code);
        s.jsw_database.set(c.jsw.database);
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

            mysql: MysqlConfig {
                host: s.mysql_host.get_clone().trim().to_string(),
                port: s.mysql_port.get_clone().trim().parse().unwrap_or(40022),
                username: s.mysql_username.get_clone().trim().to_string(),
                password: s.mysql_password.get_clone().trim().to_string(),
                database: s.mysql_database.get_clone().trim().to_string(),
            },
            jsw: JswSettings {
                base_url: s.jsw_base_url.get_clone().trim().to_string(),
                token: s.jsw_token.get_clone().trim().to_string(),
                server_code: s.jsw_server_code.get_clone().trim().to_string(),
                database: s.jsw_database.get_clone().trim().to_string(),
            },
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
                    div(class="section-title") { "内网数据库配置 (直连 MySQL)" }
                    div(class="grid grid-3", style="margin-bottom: 12px;") {
                        div(class="field") {
                            label { "MySQL 地址 (Host)" }
                            input(r#type="text", placeholder="10.12.55.40", bind:value=s.mysql_host)
                        }
                        div(class="field") {
                            label { "MySQL 端口 (Port)" }
                            input(r#type="text", placeholder="40022", bind:value=s.mysql_port)
                        }
                        div(class="field") {
                            label { "默认数据库名 (Database)" }
                            input(r#type="text", placeholder="seb_goods_db", bind:value=s.mysql_database)
                        }
                    }
                    div(class="grid grid-2") {
                        div(class="field") {
                            label { "用户名 (Username)" }
                            input(r#type="text", placeholder="dev_code_test", bind:value=s.mysql_username)
                        }
                        div(class="field") {
                            label { "密码 (Password)" }
                            input(r#type="text", placeholder="密码", bind:value=s.mysql_password)
                        }
                    }
                }

                div(class="section") {
                    div(class="section-title") { "外网与正式数据库配置 (JSW 查询)" }
                    div(class="grid grid-2", style="margin-bottom: 12px;") {
                        div(class="field") {
                            label { "JSW 服务地址 (Base URL)" }
                            input(r#type="text", placeholder="https://jsw.gydev.cn", bind:value=s.jsw_base_url)
                        }
                        div(class="field") {
                            label { "JSW 数据库名称 (Database)" }
                            input(r#type="text", placeholder="seb_goods_db", bind:value=s.jsw_database)
                        }
                    }
                    div(class="grid grid-2") {
                        div(class="field") {
                            label { "JSW 访问令牌 (Token)" }
                            input(r#type="text", placeholder="jsw_48312d76...", bind:value=s.jsw_token)
                        }
                        div(class="field") {
                            label { "JSW 服务器编码 (Server Code)" }
                            input(r#type="text", placeholder="1006", bind:value=s.jsw_server_code)
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
