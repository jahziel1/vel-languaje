use crate::host::VelHost;

impl VelHost {
    // ── Body builder ─────────────────────────────────────────────────────────

    pub fn body_begin(&mut self) {
        self.body_fields.clear();
    }

    pub fn body_field_str(&mut self, key: Option<String>, val: Option<String>) {
        if let (Some(k), Some(v)) = (key, val) {
            self.body_fields.push((k, serde_json::Value::String(v)));
        }
    }

    pub fn body_field_num(&mut self, key: Option<String>, val: f64) {
        if let Some(k) = key {
            let n =
                serde_json::Number::from_f64(val).unwrap_or_else(|| serde_json::Number::from(0));
            self.body_fields.push((k, serde_json::Value::Number(n)));
        }
    }

    pub fn body_field_bool(&mut self, key: Option<String>, val: i32) {
        if let Some(k) = key {
            self.body_fields
                .push((k, serde_json::Value::Bool(val != 0)));
        }
    }

    pub fn body_done(&mut self) {
        let map: serde_json::Map<String, serde_json::Value> = self.body_fields.drain(..).collect();
        self.pending_body = Some(serde_json::Value::Object(map).to_string());
    }

    // ── Header builder ───────────────────────────────────────────────────────

    pub fn header_begin(&mut self) {
        self.pending_headers.clear();
    }

    pub fn header_field(&mut self, key: Option<String>, val: Option<String>) {
        if let (Some(k), Some(v)) = (key, val) {
            self.pending_headers.push((k, v));
        }
    }

    pub fn header_done(&mut self) {}

    /// Add a header whose value is the current str_builder content, then clear it.
    pub fn header_field_str(&mut self, key: Option<String>) {
        if let Some(k) = key {
            let v = std::mem::take(&mut self.str_builder);
            self.pending_headers.push((k, v));
        } else {
            self.str_builder.clear();
        }
    }

    // ── URL builder ──────────────────────────────────────────────────────────

    pub fn url_begin(&mut self) {
        self.url_builder.clear();
    }

    pub fn url_lit(&mut self, s: Option<String>) {
        if let Some(s) = s {
            self.url_builder.push_str(&s);
        }
    }

    pub fn url_num(&mut self, val: f64) {
        if val.fract() == 0.0 && val.abs() < 1e15 {
            self.url_builder.push_str(&(val as i64).to_string());
        } else {
            self.url_builder.push_str(&val.to_string());
        }
    }

    pub fn url_done(&mut self) {
        self.pending_url = self.url_builder.clone();
        self.url_builder.clear();
    }

    /// Returns 1 if the pending URL differs from the stored URL for `reqid`.
    /// Updates the stored URL when a change is detected.
    /// Returns 0 when reqid == 0 (first fetch handled by reqid==0 check in WASM).
    pub fn api_url_changed(&mut self, reqid: u32) -> i32 {
        if reqid == 0 {
            return 0;
        }
        let current = self.pending_url.clone();
        let stored = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.url.clone()));
        match stored {
            Some(prev) if prev == current => 0,
            _ => {
                if let Ok(mut store) = self.api_store.lock()
                    && let Some(entry) = store.get_mut(&reqid)
                {
                    entry.url = current;
                }
                1
            }
        }
    }
}
