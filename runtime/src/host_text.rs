use crate::host::VelHost;

impl VelHost {
    /// Append text_state[key] to str_builder (used in interpolation).
    pub fn text_state_get(&mut self, key: Option<String>) {
        if let Some(k) = key
            && let Some(v) = self.text_state.get(&k)
        {
            let v = v.clone();
            self.str_builder.push_str(&v);
        }
    }

    /// Set text_state[key] = static string from WASM data section.
    pub fn text_state_set(&mut self, key: Option<String>, val: Option<String>) {
        if let (Some(k), Some(v)) = (key, val) {
            self.text_state.insert(k, v);
        }
    }

    /// Return 1 if text_state[key] is non-empty, 0 otherwise.
    pub fn text_state_bool(&self, key: Option<String>) -> i32 {
        key.and_then(|k| self.text_state.get(&k))
            .map(|v| if v.is_empty() { 0 } else { 1 })
            .unwrap_or(0)
    }

    /// Set text_state[key] = current str_builder value, then clear str_builder.
    pub fn text_state_set_built(&mut self, key: Option<String>) {
        if let Some(k) = key {
            let v = self.str_builder.clone();
            self.text_state.insert(k, v);
            self.str_builder.clear();
        }
    }
}
