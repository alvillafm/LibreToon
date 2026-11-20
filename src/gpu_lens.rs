//! Compute pass for the full-quality lens kernel, with CPU fallback.
use eframe::wgpu::{self, util::DeviceExt};
use std::sync::{Arc, OnceLock};

thread_local! {
    static WORKER_GPU: std::cell::RefCell<Option<Arc<GpuLens>>> = const { std::cell::RefCell::new(None) };
    static LIGHT_BATCH: std::cell::RefCell<Option<(Vec<crate::gpu_light::OwnedLightDispatch>, usize, bool)>> = const { std::cell::RefCell::new(None) };
}

// Preserve the CPU image until the complete chain succeeds, so any unsupported
// light, allocation failure or device loss can replay the original CPU path.
pub(crate) fn light_batch(pixels: &mut [u8], mut draw: impl FnMut(&mut [u8])) {
    let gpu = WORKER_GPU.with(|slot| slot.borrow().clone());
    let Some(gpu) = gpu else {
        draw(pixels);
        return;
    };
    if LIGHT_BATCH.with(|slot| slot.borrow().is_some()) {
        draw(pixels);
        return;
    }
    LIGHT_BATCH.with(|slot| *slot.borrow_mut() = Some((Vec::new(), 0, true)));
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            LIGHT_BATCH.with(|slot| *slot.borrow_mut() = None);
        }
    }
    let _reset = Reset;
    draw(pixels);
    let (lights, _, valid) = LIGHT_BATCH.with(|slot| slot.borrow_mut().take().unwrap());
    let inputs: Vec<_> = lights.iter().map(|light| light.borrowed()).collect();
    let completed = valid
        && gpu
            .lights
            .get_or_init(|| crate::gpu_light::GpuLight::new(gpu.device.clone(), gpu.queue.clone()))
            .as_ref()
            .is_some_and(|lights| lights.shade_many(pixels, &inputs));
    if completed {
        gpu.light_passes
            .fetch_add(inputs.len(), std::sync::atomic::Ordering::Relaxed);
    } else {
        // A thread-local guard restores the device even if the CPU path panics.
        struct Restore(Option<Arc<GpuLens>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                use_on_worker(self.0.take());
            }
        }
        let _restore = Restore(Some(gpu));
        use_on_worker(None);
        draw(pixels);
    }
}

pub(crate) fn collecting_lights() -> bool {
    LIGHT_BATCH.with(|slot| slot.borrow().is_some())
}

pub(crate) fn use_on_worker(gpu: Option<Arc<GpuLens>>) {
    WORKER_GPU.with(|slot| *slot.borrow_mut() = gpu);
}

pub(crate) fn blur_on_worker(
    image: &image::RgbaImage,
    offsets: &[(i32, i32)],
) -> Option<image::RgbaImage> {
    if (image.len() / 4).saturating_mul(offsets.len()) < 8_000_000 {
        return None;
    }
    WORKER_GPU.with(|slot| slot.borrow().as_ref()?.blur(image, offsets))
}

pub(crate) fn post_on_worker(pixels: &mut [u8], configs: &[Vec<u32>]) -> bool {
    WORKER_GPU.with(|slot| {
        let slot = slot.borrow();
        let Some(gpu) = slot.as_ref() else {
            return false;
        };
        gpu.post
            .get_or_init(|| crate::gpu_post::GpuPost::new(gpu.device.clone(), gpu.queue.clone()))
            .as_ref()
            .is_some_and(|post| post.apply(pixels, configs))
    })
}

pub(crate) fn available_on_worker() -> bool {
    WORKER_GPU.with(|slot| slot.borrow().is_some())
}
pub(crate) fn on_worker() -> Option<Arc<GpuLens>> {
    WORKER_GPU.with(|slot| slot.borrow().clone())
}

pub(crate) fn surface_on_worker(
    pixels: &[u8],
    width: usize,
    height: usize,
    bevel: f32,
    texture: f32,
    height_map: Option<&[u8]>,
    step: usize,
) -> Option<crate::light::Surface> {
    WORKER_GPU.with(|slot| {
        let slot = slot.borrow();
        let gpu = slot.as_ref()?;
        gpu.surfaces
            .get_or_init(|| {
                crate::gpu_surface::GpuSurface::new(gpu.device.clone(), gpu.queue.clone())
            })
            .as_ref()?
            .surface(pixels, width, height, bevel, texture, height_map, step)
    })
}

pub(crate) fn gaussian_on_worker(
    source: &image::RgbaImage,
    sigma: f32,
) -> Option<image::RgbaImage> {
    if source.len() < 256 * 256 * 4 {
        return None;
    }
    WORKER_GPU.with(|slot| {
        let slot = slot.borrow();
        let gpu = slot.as_ref()?;
        gpu.gaussian
            .get_or_init(|| {
                crate::gpu_gaussian::GpuGaussian::new(gpu.device.clone(), gpu.queue.clone())
            })
            .as_ref()?
            .blur(source, sigma)
    })
}

pub(crate) fn project_on_worker(
    source: &Arc<Vec<u8>>,
    source_size: [u32; 2],
    output: [u32; 2],
    start: eframe::egui::Pos2,
    dx: eframe::egui::Vec2,
    dy: eframe::egui::Vec2,
) -> Option<Vec<u8>> {
    WORKER_GPU.with(|slot| {
        let slot = slot.borrow();
        let gpu = slot.as_ref()?;
        gpu.planes
            .get_or_init(|| crate::gpu_plane::GpuPlane::new(gpu.device.clone(), gpu.queue.clone()))
            .as_ref()?
            .project(source, source_size, output, start, dx, dy)
    })
}

pub(crate) fn raster_on_worker<'a>(
    w: u32,
    h: u32,
    transparent: bool,
    meshes: &[eframe::egui::ClippedPrimitive],
    textures: impl Fn(eframe::egui::TextureId) -> Option<([usize; 2], &'a [u8])>,
) -> Option<Vec<u8>> {
    WORKER_GPU.with(|slot| {
        let slot = slot.borrow();
        let gpu = slot.as_ref()?;
        gpu.raster
            .get_or_init(|| {
                crate::gpu_raster::GpuRaster::new(gpu.device.clone(), gpu.queue.clone())
            })
            .as_ref()?
            .render(w, h, transparent, meshes, textures)
    })
}

pub(crate) fn light_on_worker(
    pixels: &mut [u8],
    dispatch: &crate::gpu_light::LightDispatch<'_>,
) -> bool {
    let collected = LIGHT_BATCH.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some((lights, bytes, valid)) = slot.as_mut() else {
            return false;
        };
        let additional =
            dispatch.mask.map_or(0, |mask| mask.len()) + dispatch.gobo.map_or(0, |gobo| gobo.len());
        if lights.len() >= 64 || bytes.saturating_add(additional) > 64 * 1024 * 1024 {
            *valid = false;
        } else if *valid {
            *bytes += additional;
            lights.push(crate::gpu_light::OwnedLightDispatch {
                config: dispatch.config,
                surface: dispatch.surface.clone(),
                mask: dispatch.mask.map(<[u8]>::to_vec),
                gobo: dispatch.gobo.map(<[u8]>::to_vec),
            });
        }
        true
    });
    if collected {
        return true;
    }
    WORKER_GPU.with(|slot| {
        let slot = slot.borrow();
        let Some(gpu) = slot.as_ref() else {
            return false;
        };
        let Some(lights) = gpu
            .lights
            .get_or_init(|| crate::gpu_light::GpuLight::new(gpu.device.clone(), gpu.queue.clone()))
        else {
            return false;
        };
        let completed = lights.shade(pixels, dispatch);
        if completed && dispatch.config[6] <= 6 {
            gpu.light_passes
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        completed
    })
}

pub(crate) struct GpuLens {
    pub(crate) frame_lock: std::sync::Mutex<()>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pipeline: OnceLock<Option<wgpu::ComputePipeline>>,
    pub(crate) lights: OnceLock<Option<crate::gpu_light::GpuLight>>,
    pub(crate) post: OnceLock<Option<crate::gpu_post::GpuPost>>,
    pub(crate) raster: OnceLock<Option<crate::gpu_raster::GpuRaster>>,
    pub(crate) planes: OnceLock<Option<crate::gpu_plane::GpuPlane>>,
    pub(crate) gaussian: OnceLock<Option<crate::gpu_gaussian::GpuGaussian>>,
    pub(crate) surfaces: OnceLock<Option<crate::gpu_surface::GpuSurface>>,
    resize: OnceLock<Option<crate::gpu_resize::GpuResize>>,
    pub(crate) light_passes: std::sync::atomic::AtomicUsize,
    pub(crate) composed_frames: std::sync::atomic::AtomicUsize,
}

impl GpuLens {
    pub(crate) fn native_outline(
        &self,
        source: &Arc<crate::gpu_image::Image>,
        distance: f32,
        feather: f32,
        opacity: f32,
        color: [u8; 3],
    ) -> Option<Arc<crate::gpu_image::Image>> {
        let rgba = self
            .surfaces
            .get_or_init(|| {
                crate::gpu_surface::GpuSurface::new(self.device.clone(), self.queue.clone())
            })
            .as_ref()?
            .outline_native(source, distance, feather, opacity, color)?;
        let mut config = vec![0u32; 16];
        config[..3].copy_from_slice(&[source.width, source.height, 16]);
        self.native_post(&rgba, &config, None)
    }
    pub(crate) fn native_resize(
        &self,
        source: &Arc<crate::gpu_image::Image>,
        size: [u32; 2],
    ) -> Option<Arc<crate::gpu_image::Image>> {
        if [source.width, source.height] == size {
            return Some(source.clone());
        }
        self.resize
            .get_or_init(|| {
                crate::gpu_resize::GpuResize::new(self.device.clone(), self.queue.clone())
            })
            .as_ref()?
            .native(source, size)
    }
    pub(crate) fn native_blur(
        &self,
        source: &Arc<crate::gpu_image::Image>,
        radius: f32,
        optics: &crate::depth::Optics,
    ) -> Option<Arc<crate::gpu_image::Image>> {
        if radius < 0.1 {
            return Some(source.clone());
        }
        let factor = match optics.quality {
            crate::depth::Quality::Low => 0.25,
            crate::depth::Quality::Medium => 0.5,
            crate::depth::Quality::High => 1.,
        };
        let size = [
            ((source.width as f32 * factor).round() as u32).max(1),
            ((source.height as f32 * factor).round() as u32).max(1),
        ];
        let small = self.native_resize(source, size)?;
        let blurred = if optics.method == crate::depth::BlurMethod::Gaussian {
            self.native_gaussian(&small, (radius * factor * 0.5).max(0.1))?
        } else {
            let samples = match optics.quality {
                crate::depth::Quality::Low => 8,
                crate::depth::Quality::Medium => 20,
                crate::depth::Quality::High => 48,
            };
            let offsets: Vec<_> = (0..samples)
                .map(|i| {
                    let a = i as f32 * 2.399963;
                    let r = ((i as f32 + 0.5) / samples as f32).sqrt() * radius * factor;
                    ((a.cos() * r).round() as i32, (a.sin() * r).round() as i32)
                })
                .collect();
            self.native_lens(&small, &offsets)?
        };
        self.native_resize(&blurred, [source.width, source.height])
    }
    pub(crate) fn native_post(
        &self,
        source: &Arc<crate::gpu_image::Image>,
        config: &[u32],
        aux: Option<&crate::gpu_image::Image>,
    ) -> Option<Arc<crate::gpu_image::Image>> {
        self.post
            .get_or_init(|| crate::gpu_post::GpuPost::new(self.device.clone(), self.queue.clone()))
            .as_ref()?
            .native(source, config, aux)
    }
    pub(crate) fn native_filter(
        &self,
        source: &Arc<crate::gpu_image::Image>,
        kind: u32,
        mask: Option<&crate::gpu_image::Image>,
        values: &[f32],
        color: [u8; 3],
    ) -> Option<Arc<crate::gpu_image::Image>> {
        let mut config = [0u32; 48];
        config[..7].copy_from_slice(&[
            source.width,
            source.height,
            0,
            0,
            source.width,
            source.height,
            kind,
        ]);
        for (i, v) in values.iter().enumerate() {
            config[16 + i] = v.to_bits();
        }
        for (i, v) in color.into_iter().enumerate() {
            config[28 + i] = (f32::from(v) / 255.).to_bits();
        }
        self.lights
            .get_or_init(|| {
                crate::gpu_light::GpuLight::new(self.device.clone(), self.queue.clone())
            })
            .as_ref()?
            .native(
                source,
                &crate::gpu_light::LightDispatch {
                    config,
                    surface: None,
                    mask: None,
                    gobo: None,
                },
                None,
                mask,
            )
    }
    pub(crate) fn native_gaussian(
        &self,
        source: &Arc<crate::gpu_image::Image>,
        sigma: f32,
    ) -> Option<Arc<crate::gpu_image::Image>> {
        if sigma < 0.05 {
            return Some(source.clone());
        }
        self.gaussian
            .get_or_init(|| {
                crate::gpu_gaussian::GpuGaussian::new(self.device.clone(), self.queue.clone())
            })
            .as_ref()?
            .native(source, sigma)
    }
    pub(crate) fn native_lens(
        &self,
        source: &Arc<crate::gpu_image::Image>,
        offsets: &[(i32, i32)],
    ) -> Option<Arc<crate::gpu_image::Image>> {
        let pipeline = self
            .pipeline
            .get_or_init(|| self.create_pipeline())
            .as_ref()?;
        let mut data = vec![
            source.width as i32,
            source.height as i32,
            offsets.len() as i32,
        ];
        data.extend(offsets.iter().flat_map(|&(x, y)| [x, y]));
        let data: Vec<_> = data.into_iter().flat_map(i32::to_le_bytes).collect();
        let params = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Resident lens samples"),
                contents: &data,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let output = crate::gpu_image::Image::new(&self.device, source.width, source.height, 4);
        let entries: Vec<_> = [&source.buffer, &params, &output.buffer]
            .into_iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Resident lens blur"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(source.width.div_ceil(16), source.height.div_ceil(16), 1);
        }
        self.queue.submit([encoder.finish()]);
        Some(output)
    }
    pub(crate) fn from_adapter(adapter: &wgpu::Adapter) -> Option<Self> {
        let caps = adapter.get_downlevel_capabilities();
        let limits = adapter.limits();
        (caps.flags.contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
            && adapter.get_info().device_type != wgpu::DeviceType::Cpu
            && limits.max_compute_invocations_per_workgroup >= 256
            && limits.max_compute_workgroup_size_x >= 16
            && limits.max_compute_workgroup_size_y >= 16
            && limits.max_storage_buffers_per_shader_stage >= 3)
            .then(|| {
                // Error scopes and queue synchronization belong to a device,
                // not to a Rust thread. Egui must never interleave its frame
                // with the compositor's compute scopes on that same device.
                let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    label: Some("LibreToon composition device"),
                    ..Default::default()
                }))
                .ok()?;
                Some(Self::new(device, queue))
            })
            .flatten()
    }

    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            frame_lock: std::sync::Mutex::new(()),
            device,
            queue,
            pipeline: OnceLock::new(),
            lights: OnceLock::new(),
            post: OnceLock::new(),
            raster: OnceLock::new(),
            planes: OnceLock::new(),
            gaussian: OnceLock::new(),
            surfaces: OnceLock::new(),
            resize: OnceLock::new(),
            light_passes: std::sync::atomic::AtomicUsize::new(0),
            composed_frames: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub(crate) fn light_pass_count(&self) -> usize {
        self.light_passes.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn create_pipeline(&self) -> Option<wgpu::ComputePipeline> {
        let device = &self.device;
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Lens blur parity probe"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
    @group(0) @binding(0) var<storage, read> src: array<u32>;
    @group(0) @binding(1) var<storage, read> kernel: array<i32>;
    @group(0) @binding(2) var<storage, read_write> dst: array<u32>;

    @compute @workgroup_size(16, 16)
    fn main(@builtin(global_invocation_id) pos: vec3<u32>) {
        let w = kernel[0];
        let h = kernel[1];
        let n = kernel[2];
        if (pos.x >= u32(w) || pos.y >= u32(h)) { return; }
        let shifts = vec4<u32>(0, 8, 16, 24);
        var sum = vec4<u32>(0);
        for (var i = 0; i < n; i++) {
            let x = i32(pos.x) + kernel[3 + 2 * i];
            let y = i32(pos.y) + kernel[4 + 2 * i];
            if (x >= 0 && x < w && y >= 0 && y < h) {
                sum += (vec4<u32>(src[y * w + x]) >> shifts) & vec4<u32>(255);
            }
        }
        let packed = (sum / vec4<u32>(u32(n))) << shifts;
        dst[pos.y * u32(w) + pos.x] = packed.x | packed.y | packed.z | packed.w;
    }
    "#
                .into(),
            ),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Lens blur parity probe"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if block_on(errors.pop()).is_some() {
            None
        } else {
            Some(pipeline)
        }
    }

    fn blur(&self, source: &image::RgbaImage, offsets: &[(i32, i32)]) -> Option<image::RgbaImage> {
        let (w, h) = source.dimensions();
        let device = &self.device;
        let limits = device.limits();
        if w == 0
            || h == 0
            || offsets.is_empty()
            || source.len() as u64 > limits.max_storage_buffer_binding_size
            || source.len() as u64 > limits.max_buffer_size
            || w.div_ceil(16).max(h.div_ceil(16)) > limits.max_compute_workgroups_per_dimension
        {
            return None;
        }
        let pipeline = self
            .pipeline
            .get_or_init(|| self.create_pipeline())
            .as_ref()?;
        let allocation = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = self.dispatch(source, offsets, pipeline);
        let invalid = block_on(validation.pop()).is_some();
        let exhausted = block_on(allocation.pop()).is_some();
        if invalid || exhausted { None } else { result }
    }

    fn dispatch(
        &self,
        source: &image::RgbaImage,
        offsets: &[(i32, i32)],
        pipeline: &wgpu::ComputePipeline,
    ) -> Option<image::RgbaImage> {
        let (w, h) = source.dimensions();
        let device = &self.device;
        let queue = &self.queue;
        let mut kernel = vec![w as i32, h as i32, offsets.len() as i32];
        kernel.extend(offsets.iter().flat_map(|&(x, y)| [x, y]));
        let kernel: Vec<_> = kernel.into_iter().flat_map(i32::to_le_bytes).collect();
        let buffer = |data| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: data,
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        let src = buffer(source.as_raw());
        let kernel = buffer(&kernel);
        let size = u64::from(w) * u64::from(h) * 4;
        let dst = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let entries = [&src, &kernel, &dst];
        let entries: Vec<_> = entries
            .into_iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(w.div_ceil(16), h.div_ceil(16), 1);
        }
        encoder.copy_buffer_to_buffer(&dst, 0, &readback, 0, size);
        let submitted = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(2)),
            })
            .ok()?;
        rx.recv_timeout(std::time::Duration::from_secs(2))
            .ok()?
            .ok()?;
        let mapped = slice.get_mapped_range().ok()?;
        image::RgbaImage::from_raw(w, h, mapped.to_vec())
    }
}

pub(crate) fn block_on<F: std::future::Future>(future: F) -> F::Output {
    struct Wake(std::thread::Thread);
    impl std::task::Wake for Wake {
        fn wake(self: std::sync::Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = std::task::Waker::from(std::sync::Arc::new(Wake(std::thread::current())));
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(value) => return value,
            std::task::Poll::Pending => std::thread::park(),
        }
    }
}

#[test]
#[ignore = "requires GPU; compositor must not share the UI device"]
fn gpu_compositor_uses_an_independent_device() {
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
    let (ui_device, _) = block_on(adapter.request_device(&Default::default())).unwrap();
    let gpu = GpuLens::from_adapter(&adapter).expect("independent compute device");
    assert_ne!(ui_device, gpu.device);
    let ui_scope = ui_device.push_error_scope(wgpu::ErrorFilter::Validation);
    let source =
        crate::gpu_image::Image::upload(&gpu.device, 64, 32, &[40, 60, 80, 255].repeat(64 * 32));
    let mut config = vec![0; 16];
    config[..3].copy_from_slice(&[64, 32, 13]);
    let result =
        crate::gpu_image::checked_scope(&gpu.device, || gpu.native_post(&source, &config, None))
            .unwrap();
    assert_eq!(
        result.read(&gpu.device, &gpu.queue).unwrap(),
        [40, 60, 80, 255].repeat(64 * 32)
    );
    assert!(block_on(ui_scope.pop()).is_none());
}

#[test]
#[ignore = "manual compute benchmark requiring a compatible GPU adapter"]
fn benchmark_gpu_lens_parity() {
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        .expect("GPU adapter unavailable");
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        .expect("GPU device unavailable");
    let lens = GpuLens::new(device, queue);
    assert!(
        lens.pipeline
            .get_or_init(|| lens.create_pipeline())
            .is_some()
    );
    let (w, h) = (1920u32, 1080u32);
    let source = image::RgbaImage::from_fn(w, h, |x, y| {
        let a = ((x + 3 * y) % 256) as u8;
        image::Rgba([a / 2, a / 3, a / 4, a])
    });
    let offsets: Vec<_> = (0..48)
        .map(|i| {
            let a = (i as f32) * 2.399963;
            let r = ((i as f32 + 0.5) / 48.).sqrt() * 42.;
            ((a.cos() * r).round() as i32, (a.sin() * r).round() as i32)
        })
        .collect();
    let start = std::time::Instant::now();
    let reference = crate::depth::lens_blur(&source, &offsets);
    let cpu = start.elapsed();
    let start = std::time::Instant::now();
    let pixels = lens.blur(&source, &offsets).expect("GPU lens pass");
    let gpu = start.elapsed();
    assert!(pixels == reference);
    eprintln!(
        "{}: CPU {cpu:?}; GPU with transfers {gpu:?}; exact RGBA match",
        adapter.get_info().name
    );
}
