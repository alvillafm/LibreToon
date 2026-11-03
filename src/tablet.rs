//! Real pressure from pointer events and the desktop tablet backends.
//! Mouse input remains at full width; no pressure is inferred from speed.
#[derive(Default)]
pub struct PenInput {
    pressure: Option<f32>,
    touch_until: f64,
    #[cfg(windows)]
    native: native::Wintab,
    #[cfg(target_os = "linux")]
    x11: x11::Tablet,
}
impl PenInput {
    pub fn update(&mut self, ctx: &eframe::egui::Context) {
        let (contact, touch_sample) = ctx.input(|i| {
            let mut touch_sample = None;
            for event in &i.events {
                if let eframe::egui::Event::Touch { force, phase, .. } = event {
                    match phase {
                        eframe::egui::TouchPhase::Start | eframe::egui::TouchPhase::Move => {
                            if let Some(force) = force.filter(|force| force.is_finite()) {
                                self.pressure = Some(force.clamp(0., 1.));
                                self.touch_until = i.time + 0.2;
                                touch_sample = self.pressure;
                            }
                        }
                        eframe::egui::TouchPhase::End | eframe::egui::TouchPhase::Cancel => {
                            self.touch_until = f64::NEG_INFINITY;
                        }
                    }
                }
            }
            let contact = i.focused
                && (i.pointer.primary_down()
                    || i.pointer.primary_pressed()
                    || i.pointer.primary_released());
            if !contact || i.time > self.touch_until {
                self.pressure = touch_sample.filter(|_| contact);
            }
            (contact, touch_sample)
        });
        #[cfg(windows)]
        {
            let wintab = self.native.poll();
            if contact && touch_sample.is_none() && self.pressure.is_none() {
                self.pressure = wintab;
            }
        }
        #[cfg(target_os = "macos")]
        if contact && touch_sample.is_none() {
            self.pressure = mac::pressure().or(self.pressure);
        }
        #[cfg(target_os = "linux")]
        {
            if !contact {
                self.x11.release();
            } else if touch_sample.is_none() && self.pressure.is_none() {
                self.pressure = self.x11.pressure();
            }
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
    // Virtual devices (UINT_MAX) do not identify a physical device category.
    fn device_category(device: u32) -> Option<u32> {
        100u32.checked_add(device)
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
                if info(5, 0, (&mut lc as *mut LogContext).cast()) == 0
                    && info(4, 0, (&mut lc as *mut LogContext).cast()) == 0
                {
                    return;
                }
                let mut axis = Axis::default();
                let mut device_count = 0u32;
                info(1, 4, (&mut device_count as *mut u32).cast());
                // Some drivers expose a virtual default context. In that case,
                // open the first physical device that reports a pressure axis.
                let device = std::iter::once(lc.device)
                    .chain(0..device_count.min(64))
                    .find(|&device| {
                        device_category(device).is_some_and(|category| {
                            info(category, 15, (&mut axis as *mut Axis).cast()) != 0
                                && axis.max > axis.min
                        })
                    });
                let Some(device) = device else {
                    return;
                };
                lc.device = device;
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
                    let value = (values[count - 1] as f32 - self.min as f32)
                        / (self.max as f64 - self.min as f64) as f32;
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
    #[cfg(test)]
    mod tests {
        use super::device_category;

        #[test]
        fn virtual_and_overflowing_devices_are_rejected() {
            assert_eq!(device_category(0), Some(100));
            assert_eq!(device_category(1), Some(101));
            assert_eq!(device_category(u32::MAX), None);
            assert_eq!(device_category(u32::MAX - 99), None);
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

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::{c_char, c_void};
    type Object = *mut c_void;

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Object;
        fn sel_registerName(name: *const c_char) -> Object;
        fn objc_msgSend();
    }

    pub(super) fn pressure() -> Option<f32> {
        // Called on the AppKit thread. Each signature matches the selector's
        // return type, and pressure is only valid on the event types below.
        unsafe {
            let object: unsafe extern "C" fn(Object, Object) -> Object =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let integer: unsafe extern "C" fn(Object, Object) -> usize =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let subtype: unsafe extern "C" fn(Object, Object) -> i16 =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let float: unsafe extern "C" fn(Object, Object) -> f32 =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let app = object(
                objc_getClass(c"NSApplication".as_ptr()),
                sel_registerName(c"sharedApplication".as_ptr()),
            );
            let event = object(app, sel_registerName(c"currentEvent".as_ptr()));
            if event.is_null() {
                return None;
            }
            let kind = integer(event, sel_registerName(c"type".as_ptr()));
            let tablet_point = kind == 23
                || (matches!(kind, 1 | 6)
                    && subtype(event, sel_registerName(c"subtype".as_ptr())) == 1);
            if tablet_point {
                let force = float(event, sel_registerName(c"pressure".as_ptr()));
                return force.is_finite().then(|| force.clamp(0., 1.));
            }
            None
        }
    }
}

#[cfg(target_os = "linux")]
mod x11 {
    use std::{
        ffi::{CStr, c_char, c_int, c_ulong, c_void},
        ptr,
        time::{Duration, Instant},
    };

    type Handle = *mut c_void;

    #[repr(C)]
    struct Device {
        id: c_int,
        name: *mut c_char,
        usage: c_int,
        attachment: c_int,
        enabled: c_int,
        class_count: c_int,
        classes: *mut *mut c_int,
    }

    #[repr(C)]
    struct Valuator {
        kind: c_int,
        source: c_int,
        number: c_int,
        label: c_ulong,
        min: f64,
        max: f64,
        value: f64,
        resolution: c_int,
        mode: c_int,
    }

    type Query = unsafe extern "C" fn(Handle, c_int, *mut c_int) -> *mut Device;
    type Free = unsafe extern "C" fn(*mut Device);
    type Close = unsafe extern "C" fn(Handle) -> c_int;

    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(name: *const c_char, flags: c_int) -> Handle;
        fn dlsym(handle: Handle, name: *const c_char) -> Handle;
        fn dlclose(handle: Handle) -> c_int;
    }

    struct Library(Handle);
    impl Library {
        fn open(name: &CStr) -> Option<Self> {
            let handle = unsafe { dlopen(name.as_ptr(), 2) };
            if handle.is_null() {
                None
            } else {
                Some(Self(handle))
            }
        }

        fn symbol(&self, name: &CStr) -> Option<Handle> {
            let symbol = unsafe { dlsym(self.0, name.as_ptr()) };
            (!symbol.is_null()).then_some(symbol)
        }
    }

    impl Drop for Library {
        fn drop(&mut self) {
            unsafe { dlclose(self.0) };
        }
    }

    struct Connection {
        display: Handle,
        pressure_atom: c_ulong,
        query: Query,
        free: Free,
        close: Close,
        _xi: Library,
        _xlib: Library,
    }

    impl Connection {
        fn open() -> Option<Self> {
            // Load the desktop libraries only when X11 is available, so
            // Wayland-only installations can still use normal pointer input.
            std::env::var_os("DISPLAY")?;
            let xlib = Library::open(c"libX11.so.6")?;
            let xi = Library::open(c"libXi.so.6")?;
            unsafe {
                let open: unsafe extern "C" fn(*const c_char) -> Handle =
                    std::mem::transmute(xlib.symbol(c"XOpenDisplay")?);
                let close: Close = std::mem::transmute(xlib.symbol(c"XCloseDisplay")?);
                let intern: unsafe extern "C" fn(Handle, *const c_char, c_int) -> c_ulong =
                    std::mem::transmute(xlib.symbol(c"XInternAtom")?);
                let version: unsafe extern "C" fn(Handle, *mut c_int, *mut c_int) -> c_int =
                    std::mem::transmute(xi.symbol(c"XIQueryVersion")?);
                let query: Query = std::mem::transmute(xi.symbol(c"XIQueryDevice")?);
                let free: Free = std::mem::transmute(xi.symbol(c"XIFreeDeviceInfo")?);
                let display = open(ptr::null());
                if display.is_null() {
                    return None;
                }
                let (mut major, mut minor) = (2, 0);
                if version(display, &mut major, &mut minor) != 0 || major < 2 {
                    close(display);
                    return None;
                }
                let pressure_atom = intern(display, c"Abs Pressure".as_ptr(), 0);
                Some(Self {
                    display,
                    pressure_atom,
                    query,
                    free,
                    close,
                    _xi: xi,
                    _xlib: xlib,
                })
            }
        }

        fn sample(&self) -> (bool, Option<f32>) {
            // Query all devices because a tablet may be plugged in mid-session.
            // Inspect only enabled slave pointers with the standard pressure axis.
            unsafe {
                let mut count = 0;
                let devices = (self.query)(self.display, 0, &mut count);
                if devices.is_null() {
                    return (false, None);
                }
                let mut found = false;
                let mut pressure = None;
                for device in std::slice::from_raw_parts(devices, count.max(0) as usize) {
                    if device.enabled == 0 || device.usage != 3 || device.classes.is_null() {
                        continue;
                    }
                    for &class in std::slice::from_raw_parts(
                        device.classes,
                        device.class_count.max(0) as usize,
                    ) {
                        if class.is_null() || *class != 2 {
                            continue;
                        }
                        let axis = &*class.cast::<Valuator>();
                        if axis.label == self.pressure_atom && axis.mode == 1 && axis.max > axis.min
                        {
                            found = true;
                            let sample = ((axis.value - axis.min) / (axis.max - axis.min)) as f32;
                            if sample.is_finite() && sample > 0. {
                                pressure = Some(sample.clamp(0., 1.));
                                break;
                            }
                        }
                    }
                    if pressure.is_some() {
                        break;
                    }
                }
                (self.free)(devices);
                (found, pressure)
            }
        }
    }

    impl Drop for Connection {
        fn drop(&mut self) {
            unsafe { (self.close)(self.display) };
        }
    }

    #[derive(Default)]
    pub(super) struct Tablet {
        attempted: bool,
        connection: Option<Connection>,
        next_sample: Option<Instant>,
        last: Option<f32>,
    }

    impl Tablet {
        pub(super) fn pressure(&mut self) -> Option<f32> {
            let now = Instant::now();
            if self.next_sample.is_some_and(|next| now < next) {
                return self.last;
            }
            if !self.attempted {
                self.attempted = true;
                self.connection = Connection::open();
            }
            let (device, pressure) = self.connection.as_ref()?.sample();
            self.next_sample = Some(
                now + if device {
                    Duration::from_millis(8)
                } else {
                    Duration::from_secs(2)
                },
            );
            self.last = pressure;
            pressure
        }

        pub(super) fn release(&mut self) {
            if self.last.take().is_some() {
                self.next_sample = None;
            }
        }
    }
}
