use crate::host::VelHost;

impl VelHost {
    /// Return the number of items in the JSON array body of `reqid` (0 if not an array).
    pub fn list_count(&self, reqid: u32) -> i32 {
        let body = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.body.clone()));
        body.and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|v| v.as_array().map(|a| a.len() as i32))
            .unwrap_or(0)
    }

    /// Append the string value of `field` from array[index] in `reqid` to str_builder.
    pub fn list_item_str(&mut self, reqid: u32, index: i32, field: &str) {
        let val = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.body.clone()))
            .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|arr| {
                arr.as_array()
                    .and_then(|a| a.get(index as usize))
                    .and_then(|item| item.get(field))
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
            });
        if let Some(s) = val {
            self.str_builder.push_str(&s);
        }
    }

    /// Return the numeric value of `field` from array[index] in `reqid`.
    pub fn list_item_num(&self, reqid: u32, index: i32, field: &str) -> f64 {
        let body = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.body.clone()));
        body.and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|arr| {
                arr.as_array()
                    .and_then(|a| a.get(index as usize))
                    .and_then(|item| item.get(field))
                    .and_then(|v| match v {
                        serde_json::Value::Number(n) => n.as_f64(),
                        serde_json::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
                        serde_json::Value::String(s) => s.parse().ok(),
                        _ => None,
                    })
            })
            .unwrap_or(0.0)
    }

    /// Sum the numeric values of `field` across all items in `reqid` array.
    pub fn list_sum(&self, reqid: u32, field: &str) -> f64 {
        let body = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.body.clone()));
        body.and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|v| v.as_array().cloned())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.get(field).and_then(|v| match v {
                            serde_json::Value::Number(n) => n.as_f64(),
                            serde_json::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
                            serde_json::Value::String(s) => s.parse().ok(),
                            _ => None,
                        })
                    })
                    .sum()
            })
            .unwrap_or(0.0)
    }

    /// Return 1 if `field` from array[index] in `reqid` is truthy, 0 otherwise.
    pub fn list_item_bool(&self, reqid: u32, index: i32, field: &str) -> i32 {
        let body = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.body.clone()));
        body.and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|arr| {
                arr.as_array()
                    .and_then(|a| a.get(index as usize))
                    .and_then(|item| item.get(field))
                    .and_then(|v| match v {
                        serde_json::Value::Bool(b) => Some(if *b { 1 } else { 0 }),
                        serde_json::Value::Number(n) => Some(if n.as_f64().unwrap_or(0.0) != 0.0 {
                            1
                        } else {
                            0
                        }),
                        _ => None,
                    })
            })
            .unwrap_or(0)
    }
}
