//! Real pressure from egui touch events and Windows Wintab drivers.
//! Mouse input remains at full width; no pressure is inferred from speed.
#[derive(Default)]
pub struct PenInput {
    pressure: Option<f32>,
    touch_until: f64,
    #[cfg(windows)]
    native: native::Wintab,
}
impl PenInput {
    pub fn update(&mut self, ctx: &eframe::egui::Context) {
        ctx.input(|i| {
            for event in &i.events {
                if let eframe::egui::Event::Touch {
                    force: Some(force),
                    phase,
                    ..
                } = event
                {
                    if !matches!(
                        phase,
                        eframe::egui::TouchPhase::End | eframe::egui::TouchPhase::Cancel
                    ) {
                        self.pressure = Some(force.clamp(0., 1.));
                        self.touch_until = i.time + 0.2;
                    }
                }
            }
            if i.time > self.touch_until {
                self.pressure = None;
            }
        });
        #[cfg(windows)]
        if let Some(force) = self.native.poll() {
            self.pressure = Some(force);
        }
    }
    pub fn force(&self) -> f32 {
        self.pressure.unwrap_or(1.)
    }
    pub fn detected(&self) -> bool {
        self.pressure.is_some()
    }
    #[cfg(test)]
    pub fn test_pressure(&mut self, pressure: Option<f32>) {
        self.pressure = pressure;
    }
}
#[cfg(windows)]
mod native {
    use std::ffi::{c_char, c_void};
    type Handle = *mut c_void;
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct LogContext {
        name: [c_char; 40],
        options: u32,
        status: u32,
        locks: u32,
        msg_base: u32,
        device: u32,
        pkt_rate: u32,
        pkt_data: u32,
        pkt_mode: u32,
        move_mask: u32,
        btn_down: u32,
        btn_up: u32,
        in_org_x: i32,
        in_org_y: i32,
        in_org_z: i32,
        in_ext_x: i32,
        in_ext_y: i32,
        in_ext_z: i32,
        out_org_x: i32,
        out_org_y: i32,
        out_org_z: i32,
        out_ext_x: i32,
        out_ext_y: i32,
        out_ext_z: i32,
        sens_x: u32,
        sens_y: u32,
        sens_z: u32,
        sys_mode: i32,
        sys_org_x: i32,
        sys_org_y: i32,
        sys_ext_x: i32,
        sys_ext_y: i32,
        sys_sens_x: u32,
        sys_sens_y: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Axis {
        min: i32,
        max: i32,
        units: u32,
        resolution: u32,
    }
    type Info = unsafe extern "system" fn(u32, u32, *mut c_void) -> u32;
    type Open = unsafe extern "system" fn(Handle, *mut LogContext, i32) -> Handle;
    type Packets = unsafe extern "system" fn(Handle, i32, *mut c_void) -> i32;
    type Close = unsafe extern "system" fn(Handle) -> i32;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryA(name: *const u8) -> Handle;
        fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
        fn FreeLibrary(module: Handle) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetForegroundWindow() -> Handle;
        fn GetWindowThreadProcessId(window: Handle, pid: *mut u32) -> u32;
    }
    #[derive(Default)]
    pub struct Wintab {
        attempted: bool,
        module: Handle,
        context: Handle,
        packets: Option<Packets>,
        close: Option<Close>,
        min: i32,
        max: i32,
        last: Option<f32>,
    }
    impl Wintab {
        fn initialize(&mut self) {
            // SAFETY: symbols are checked before conversion; handles are owned
            // by this object and used on the creating GUI thread only.
            unsafe {
                let hwnd = GetForegroundWindow();
                let mut pid = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                if pid != std::process::id() {
                    return;
                }
                self.attempted = true;
                self.module = LoadLibraryA(c"Wintab32.dll".as_ptr().cast());
                if self.module.is_null() {
                    return;
                }
                let info = GetProcAddress(self.module, c"WTInfoA".as_ptr().cast());
                let open = GetProcAddress(self.module, c"WTOpenA".as_ptr().cast());
                let packets = GetProcAddress(self.module, c"WTPacketsGet".as_ptr().cast());
                let close = GetProcAddress(self.module, c"WTClose".as_ptr().cast());
                if [info, open, packets, close].iter().any(|p| p.is_null()) {
                    return;
                }
                let info: Info = std::mem::transmute(info);
                let open: Open = std::mem::transmute(open);
                let mut lc: LogContext = std::mem::zeroed();
                if info(4, 0, (&mut lc as *mut LogContext).cast()) == 0 {
                    return;
                }
                let mut axis = Axis::default();
                if info(100 + lc.device, 15, (&mut axis as *mut Axis).cast()) == 0
                    || axis.max <= axis.min
                {
                    return;
                }
                lc.options |= 1;
                lc.options &= !4;
                lc.pkt_data = 0x400;
                lc.pkt_mode = 0;
                lc.move_mask = 0x400;
                lc.btn_down = 0;
                lc.btn_up = 0;
                self.context = open(hwnd, &mut lc, 1);
                self.min = axis.min;
                self.max = axis.max;
                self.packets = Some(std::mem::transmute(packets));
                self.close = Some(std::mem::transmute(close));
            }
        }
        pub fn poll(&mut self) -> Option<f32> {
            if !self.attempted {
                self.initialize();
            }
            if self.context.is_null() {
                return None;
            }
            if let Some(packets) = self.packets {
                let mut values = [0u32; 128];
                // Packet layout requests only PK_NORMAL_PRESSURE (one UINT).
                let count = unsafe { packets(self.context, 128, values.as_mut_ptr().cast()) }
                    .clamp(0, 128) as usize;
                if count > 0 {
                    let value =
                        (values[count - 1] as f32 - self.min as f32) / (self.max - self.min) as f32;
                    self.last = if value > 0. {
                        Some(value.clamp(0., 1.))
                    } else {
                        None
                    };
                }
            }
            self.last
        }
    }
    impl Drop for Wintab {
        fn drop(&mut self) {
            unsafe {
                if !self.context.is_null() {
                    if let Some(close) = self.close {
                        close(self.context);
                    }
                }
                if !self.module.is_null() {
                    FreeLibrary(self.module);
                }
            }
        }
    }
}
