use crate::host::VelHost;

impl VelHost {
    pub fn state_push(&mut self, kind: i32) {
        self.state_mode = kind as u8;
        self.state_buf.clear();
    }

    pub fn state_pop(&mut self) {
        let buf = std::mem::take(&mut self.state_buf);
        let mode = self.state_mode;
        self.state_mode = 0;
        if let Some(top) = self.stack.last_mut() {
            match mode {
                1 => top.hover_props = buf,
                2 => top.focus_props = buf,
                3 => top.active_props = buf,
                _ => {}
            }
        }
    }
}
