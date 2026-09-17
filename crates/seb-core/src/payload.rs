use crate::command_code;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

pub fn msg_id(ts: u64) -> String {
    format!("S00000000000{ts}111111111")
}

fn envelope(device_no: &str, ts: u64) -> Value {
    json!({
        "deviceSerialNo": device_no.trim(),
        "serialNo": 0x01,
        "msgId": msg_id(ts),
        "timestamp": ts,
    })
}

fn with(mut base: Value, fields: Vec<(&str, Value)>) -> Value {
    if let Some(obj) = base.as_object_mut() {
        for (k, v) in fields {
            obj.insert(k.to_string(), v);
        }
    }
    base
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BorrowOptions {
    pub open_helmet_lock: bool,
    pub open_trunk_lock: bool,
    pub helmet_taken: bool,
    pub helmet_worn: bool,
    pub trunk_lock_close: bool,
}

impl BorrowOptions {
    pub fn safety_riding(&self) -> bool {
        self.open_helmet_lock
            || self.open_trunk_lock
            || self.helmet_taken
            || self.helmet_worn
            || self.trunk_lock_close
    }
}

pub fn borrow(device_no: &str, opts: BorrowOptions) -> Value {
    let ts = now_millis();
    let param = json!({
        "type": 11,
        "value": {
            "helLock": {
                "checkPassenger": false,
                "openHelmetLock": opts.open_helmet_lock,
                "openTrunkLock": opts.open_trunk_lock,
                "safetyRiding": opts.safety_riding(),
            },
            "rideCondition": {
                "helmetOpen": false,
                "helmetTaken": opts.helmet_taken,
                "helmetWorn": opts.helmet_worn,
                "trunkLockClose": opts.trunk_lock_close,
            }
        }
    });
    with(
        envelope(device_no, ts),
        vec![
            ("controlCommand", json!(0x0B)),
            ("commandCode", json!(command_code::CONTROL)),
            // commandParam 在协议里是一个 JSON 字符串，不是嵌套对象
            ("commandParam", json!(param.to_string())),
        ],
    )
}

pub fn control(device_no: &str, control_command: u16) -> Value {
    let ts = now_millis();
    with(
        envelope(device_no, ts),
        vec![
            ("controlCommand", json!(control_command)),
            ("commandCode", json!(command_code::CONTROL)),
        ],
    )
}

pub fn voice(device_no: &str, voice_id: i64) -> Value {
    let ts = now_millis();
    with(
        envelope(device_no, ts),
        vec![
            ("commandParam", json!(voice_id)),
            ("commandCode", json!(command_code::VOICE)),
        ],
    )
}

pub fn ecu_query<S: AsRef<str>>(device_no: &str, keys: &[S]) -> Value {
    let ts = now_millis();
    let keys: Vec<&str> = keys.iter().map(|k| k.as_ref()).collect();
    with(
        envelope(device_no, ts),
        vec![
            ("commandCode", json!(command_code::QUERY)),
            ("commandParam", json!(json!(keys).to_string())),
        ],
    )
}

pub fn ecu_set<K: AsRef<str>, V: AsRef<str>>(device_no: &str, entries: &[(K, V)]) -> Value {
    let ts = now_millis();
    let mut map = serde_json::Map::new();
    for (k, v) in entries {
        map.insert(k.as_ref().to_string(), json!(v.as_ref()));
    }
    with(
        envelope(device_no, ts),
        vec![
            ("commandCode", json!(command_code::SET)),
            ("commandParam", json!(Value::Object(map).to_string())),
        ],
    )
}
