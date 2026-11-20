//! Separable Gaussian convolution with floating-point intermediate storage.
use crate::gpu_lens::block_on;
use eframe::wgpu::{self, util::DeviceExt};
use std::{sync::Mutex, time::Duration};
struct Buffers {
    bytes: u64,
    source: wgpu::Buffer,
    intermediate: wgpu::Buffer,
    output: wgpu::Buffer,
    read: wgpu::Buffer,
}

#[test]
#[ignore = "requires a GPU; full precision Gaussian parity including image borders"]
fn gpu_gaussian_matches_cpu() {
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
    let gpu = GpuGaussian::new(device, queue).expect("Gaussian shader compiles");
    let source = image::RgbaImage::from_fn(320, 240, |x, y| {
        image::Rgba([
            ((x * 7 + y * 11) % 256) as u8,
            ((x * 3 + y * 17) % 256) as u8,
            ((x + y * 3) % 256) as u8,
            ((x * 3 + y * 7) % 256) as u8,
        ])
    });
    for sigma in [0.1, 1., 3., 9., 24.] {
        let expected = image::imageops::blur(&source, sigma);
        let actual = gpu.blur(&source, sigma).expect("Gaussian dispatch");
        let max = expected
            .as_raw()
            .iter()
            .zip(actual.as_raw())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        let rms = (expected
            .as_raw()
            .iter()
            .zip(actual.as_raw())
            .map(|(a, b)| f64::from(a.abs_diff(*b)).powi(2))
            .sum::<f64>()
            / expected.len() as f64)
            .sqrt();
        eprintln!("sigma {sigma}: max {max}, RMS {rms}");
        assert!(max <= 1 && rms < 0.4);
    }
}
pub(crate) struct GpuGaussian {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    vertical: wgpu::ComputePipeline,
    horizontal: wgpu::ComputePipeline,
    buffers: Mutex<Option<Buffers>>,
}
impl GpuGaussian {
    pub(crate) fn native(
        &self,
        source: &std::sync::Arc<crate::gpu_image::Image>,
        sigma: f32,
    ) -> Option<std::sync::Arc<crate::gpu_image::Image>> {
        if !sigma.is_finite()
            || sigma <= 0.
            || sigma > 64.
            || source.bytes() * 4 > self.device.limits().max_storage_buffer_binding_size as usize
        {
            return None;
        }
        let (w, h) = (source.width, source.height);
        let bytes = source.bytes() as u64;
        let device = &self.device;
        let intermediate = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Resident Gaussian intermediate"),
            size: bytes * 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let output = crate::gpu_image::Image::new(device, w, h, 4);
        let mut size = (((((sigma - 0.8) / 0.3) + 1.) * 2. + 1.).max(3.)) as usize;
        if size % 2 == 0 {
            size += 1;
        }
        let scale = 1. / ((2. * std::f32::consts::PI).sqrt() * sigma);
        let mean = (size / 2) as f32;
        let mut kernel: Vec<_> = (0..size)
            .map(|x| (-0.5 * ((x as f32 - mean) / sigma).powf(2.)).exp() * scale)
            .collect();
        let norm = 1. / kernel.iter().sum::<f32>();
        for v in &mut kernel {
            *v *= norm;
        }
        let mut config = vec![w, h, size as u32, 0];
        config.extend(kernel.into_iter().map(f32::to_bits));
        let config: Vec<_> = config.iter().flat_map(|v| v.to_le_bytes()).collect();
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Resident Gaussian weights"),
            contents: &config,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let entries: Vec<_> = [&source.buffer, &params, &intermediate, &output.buffer]
            .into_iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Resident Gaussian"),
            layout: &self.layout,
            entries: &entries,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        for pipeline in [&self.vertical, &self.horizontal] {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(w.div_ceil(16), h.div_ceil(16), 1);
        }
        self.queue.submit([encoder.finish()]);
        Some(output)
    }
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Option<Self> {
        if device.limits().max_storage_buffers_per_shader_stage < 4 {
            return None;
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Full precision Gaussian"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_gaussian.wgsl").into()),
        });
        let entries: Vec<_> = (0..4)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: binding < 2,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Gaussian planes"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gaussian planes"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let make = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Separable Gaussian"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let vertical = make("vertical");
        let horizontal = make("horizontal");
        if let Some(error) = block_on(scope.pop()) {
            eprintln!("Gaussian GPU validation: {error}");
            return None;
        }
        Some(Self {
            device,
            queue,
            layout,
            vertical,
            horizontal,
            buffers: Mutex::new(None),
        })
    }
    pub fn blur(&self, source: &image::RgbaImage, sigma: f32) -> Option<image::RgbaImage> {
        let (w, h) = source.dimensions();
        let bytes = u64::from(w) * u64::from(h) * 4;
        let limits = self.device.limits();
        if w == 0
            || h == 0
            || !sigma.is_finite()
            || sigma <= 0.
            || sigma > 64.
            || bytes * 4 > limits.max_storage_buffer_binding_size
            || bytes * 4 > limits.max_buffer_size
            || bytes > 32 * 1024 * 1024
        {
            return None;
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let alloc = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let result = self.dispatch(source, sigma, bytes);
        let bad = block_on(alloc.pop()).is_some() | block_on(scope.pop()).is_some();
        if bad { None } else { result }
    }
    fn dispatch(
        &self,
        source: &image::RgbaImage,
        sigma: f32,
        bytes: u64,
    ) -> Option<image::RgbaImage> {
        let (w, h) = source.dimensions();
        let device = &self.device;
        let mut buffers = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
        if buffers.as_ref().is_none_or(|b| b.bytes != bytes) {
            let make = |size, usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Gaussian workspace"),
                    size,
                    usage,
                    mapped_at_creation: false,
                })
            };
            *buffers = Some(Buffers {
                bytes,
                source: make(
                    bytes,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
                intermediate: make(bytes * 4, wgpu::BufferUsages::STORAGE),
                output: make(
                    bytes,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                ),
                read: make(
                    bytes,
                    wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                ),
            });
        }
        let buffers = buffers.as_ref()?;
        let mut size = (((((sigma - 0.8) / 0.3) + 1.) * 2. + 1.).max(3.)) as usize;
        if size % 2 == 0 {
            size += 1;
        }
        let scale = 1. / ((2. * std::f32::consts::PI).sqrt() * sigma);
        let mean = (size / 2) as f32;
        let mut kernel: Vec<_> = (0..size)
            .map(|x| (-0.5 * ((x as f32 - mean) / sigma).powf(2.)).exp() * scale)
            .collect();
        let norm = 1. / kernel.iter().sum::<f32>();
        for v in &mut kernel {
            *v *= norm;
        }
        let mut config = vec![w, h, size as u32, 0];
        config.extend(kernel.into_iter().map(f32::to_bits));
        let config: Vec<_> = config.iter().flat_map(|v| v.to_le_bytes()).collect();
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gaussian weights"),
            contents: &config,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let entries: Vec<_> = [
            &buffers.source,
            &params,
            &buffers.intermediate,
            &buffers.output,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: b.as_entire_binding(),
        })
        .collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gaussian planes"),
            layout: &self.layout,
            entries: &entries,
        });
        self.queue.write_buffer(&buffers.source, 0, source.as_raw());
        let mut encoder = device.create_command_encoder(&Default::default());
        for pipeline in [&self.vertical, &self.horizontal] {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(w.div_ceil(16), h.div_ceil(16), 1);
        }
        encoder.copy_buffer_to_buffer(&buffers.output, 0, &buffers.read, 0, bytes);
        let submission = self.queue.submit([encoder.finish()]);
        let slice = buffers.read.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(2)),
            })
            .ok()?;
        rx.recv_timeout(Duration::from_secs(2)).ok()?.ok()?;
        let mapped = slice.get_mapped_range().ok()?;
        let output = mapped.to_vec();
        drop(mapped);
        buffers.read.unmap();
        image::RgbaImage::from_raw(w, h, output)
    }
}
