use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const RAW: &str = include_str!("../assets/ecu_params_dict.json");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcuParam {
    pub key: String,
    pub name: String,
    pub desc: String,
    pub default_value: Option<String>,
}

#[derive(Deserialize)]
struct RawParam {
    #[serde(default)]
    name: String,
    #[serde(default)]
    desc: String,
    #[serde(default)]
    #[serde(rename = "defaultValue")]
    default_value: Option<String>,
}

pub fn all() -> &'static [EcuParam] {
    static PARAMS: OnceLock<Vec<EcuParam>> = OnceLock::new();
    PARAMS.get_or_init(|| {
        let map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(RAW).expect("内嵌的 ecu_params_dict.json 解析失败");
        map.into_iter()
            .filter_map(|(key, value)| {
                let raw: RawParam = serde_json::from_value(value).ok()?;
                Some(EcuParam {
                    key,
                    name: raw.name,
                    desc: raw.desc,
                    default_value: raw.default_value,
                })
            })
            .collect()
    })
}

pub fn get(key: &str) -> Option<&'static EcuParam> {
    all().iter().find(|p| p.key == key)
}

pub const PRESET_HELMET_ENABLE: &[(&str, &str)] = &[
    ("DFTHELMETTYPE", "3"),
    ("DFTHELMETSUPPORTFUN", "15"),
    ("DFTHELMETWEARCHECK", "1"),
    ("DFTHELMETCTRLVEHPOWER", "1"),
];

pub const PRESET_HELMET_DISABLE: &[(&str, &str)] = &[
    ("DFTHELMETSUPPORTFUN", "3"),
    ("DFTHELMETWEARCHECK", "0"),
    ("DFTHELMETCTRLVEHPOWER", "1"),
];

pub const PRESET_HELMET_QUERY: &[&str] = &[
    "DFTHELMETSUPPORTFUN",
    "DFTHELMETWEARCHECK",
    "DFTHELMETCTRLVEHPOWER",
    "DFTHELMETTYPE",
    "DFTUNWEARMIDOUT",
];
