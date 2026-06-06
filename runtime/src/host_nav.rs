use crate::host::VelHost;

impl VelHost {
    pub fn set_navigation(&mut self, path: Option<String>) {
        self.navigate_to = path;
    }

    pub fn take_navigation(&mut self) -> Option<String> {
        self.navigate_to.take()
    }

    pub fn set_nav_param(&mut self, idx: i32, val: f64) {
        let idx = idx as usize;
        if idx >= self.nav_params.len() {
            self.nav_params.resize(idx + 1, 0.0);
        }
        self.nav_params[idx] = val;
    }

    pub fn take_nav_params(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.nav_params)
    }

    pub fn print_str(&self, s: Option<String>) {
        eprintln!("[vel] {}", s.unwrap_or_default());
    }

    pub fn print_str_built(&mut self) {
        eprintln!("[vel] {}", self.str_builder);
        self.str_builder.clear();
    }
}
