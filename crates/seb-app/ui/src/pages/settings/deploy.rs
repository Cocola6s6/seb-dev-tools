use crate::components::Check;
use crate::state::DeployDefaults;
use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub struct DeploySection {
    bike_type_id: Signal<String>,
    supplier_id: Signal<String>,
    dealer_id: Signal<String>,
    device_company_id: Signal<String>,
    battery_type_id: Signal<String>,
    has_helmet: Signal<bool>,
    has_trunk: Signal<bool>,
}

impl DeploySection {
    pub fn new() -> Self {
        Self {
            bike_type_id: create_signal(String::new()),
            supplier_id: create_signal(String::new()),
            dealer_id: create_signal(String::new()),
            device_company_id: create_signal(String::new()),
            battery_type_id: create_signal(String::new()),
            has_helmet: create_signal(true),
            has_trunk: create_signal(true),
        }
    }

    pub fn load(self, d: DeployDefaults) {
        let s = self;
        s.bike_type_id.set(d.bike_type_id.to_string());
        s.supplier_id.set(d.supplier_id.to_string());
        s.dealer_id.set(d.dealer_id.to_string());
        s.device_company_id.set(d.device_company_id.to_string());
        s.battery_type_id.set(d.battery_type_id.to_string());
        s.has_helmet.set(d.has_helmet);
        s.has_trunk.set(d.has_trunk);
    }

    pub fn collect(self) -> DeployDefaults {
        let s = self;
        DeployDefaults {
            bike_type_id: s.bike_type_id.get_clone().trim().parse().unwrap_or(0),
            supplier_id: s.supplier_id.get_clone().trim().parse().unwrap_or(0),
            dealer_id: s.dealer_id.get_clone().trim().parse().unwrap_or(0),
            device_company_id: s.device_company_id.get_clone().trim().parse().unwrap_or(0),
            battery_type_id: s.battery_type_id.get_clone().trim().parse().unwrap_or(0),
            has_helmet: s.has_helmet.get(),
            has_trunk: s.has_trunk.get(),
        }
    }

    pub fn view(self) -> View {
        let s = self;
        view! {
            div(class="section") {
                div(class="section-title") { "默认值" }
                div(class="grid grid-4") {
                    div(class="field") {
                        label { "默认车型 ID (bikeTypeId)" }
                        input(r#type="text", placeholder="0", bind:value=s.bike_type_id)
                    }
                    div(class="field") {
                        label { "默认供应商 ID (supplierId)" }
                        input(r#type="text", placeholder="0", bind:value=s.supplier_id)
                    }
                    div(class="field") {
                        label { "默认加盟商 ID (dealerId)" }
                        input(r#type="text", placeholder="0", bind:value=s.dealer_id)
                    }
                    div(class="field") {
                        label { "默认设备公司 ID (deviceCompanyId)" }
                        input(r#type="text", placeholder="0", bind:value=s.device_company_id)
                    }
                    div(class="field") {
                        label { "默认电池类型 ID (batteryTypeId)" }
                        input(r#type="text", placeholder="0", bind:value=s.battery_type_id)
                    }
                }
                div(class="row checks", style="margin-top:14px") {
                    Check(label="默认配备智能头盔锁", checked=s.has_helmet)
                    Check(label="默认配备智能后备箱锁", checked=s.has_trunk)
                }
            }
        }
    }
}
