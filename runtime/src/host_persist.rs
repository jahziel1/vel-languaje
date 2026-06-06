use serde_json::Value as JsonValue;

use crate::host::VelHost;

impl VelHost {
    pub fn persist_get_num(&self, key: Option<String>, default: f64) -> f64 {
        key.and_then(|k| self.persist_store.get(&k))
            .and_then(|v| v.as_f64())
            .unwrap_or(default)
    }

    pub fn persist_get_bool(&self, key: Option<String>, default: i32) -> i32 {
        key.and_then(|k| self.persist_store.get(&k))
            .and_then(|v| v.as_bool())
            .map(|b| if b { 1 } else { 0 })
            .unwrap_or(default)
    }

    pub fn persist_set_num(&mut self, key: Option<String>, val: f64) {
        if let Some(k) = key {
            self.persist_store.insert(k, serde_json::json!(val));
            self.flush_persist();
        }
    }

    pub fn persist_set_bool(&mut self, key: Option<String>, val: i32) {
        if let Some(k) = key {
            self.persist_store.insert(k, JsonValue::Bool(val != 0));
            self.flush_persist();
        }
    }

    fn flush_persist(&self) {
        #[cfg(feature = "renderer")]
        if let Ok(json) = serde_json::to_string(&self.persist_store) {
            let _ = std::fs::write(".vel_persist.json", json);
        }
    }
}
