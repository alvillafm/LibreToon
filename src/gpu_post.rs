//! Ping-pong post-processing: upload once, retain intermediate images on device,
//! read back only after the entire compatible adjustment chain completes.
use crate::gpu_lens::block_on;
use eframe::wgpu::{self, util::DeviceExt};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

struct Buffers {
    bytes: u64,
    a: wgpu::Buffer,
    b: wgpu::Buffer,
    read: wgpu::Buffer,
}
pub(crate) struct GpuPost {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    buffers: Mutex<Option<Buffers>>,
    failed: AtomicBool,
    flat: wgpu::Buffer,
}
impl GpuPost {
    pub(crate) fn native(
        &self,
        source: &std::sync::Arc<crate::gpu_image::Image>,
        config: &[u32],
        auxiliary: Option<&crate::gpu_image::Image>,
    ) -> Option<std::sync::Arc<crate::gpu_image::Image>> {
        if config.len() < 16
            || source.width != config[0]
            || source.height != config[1]
            || (source.bytes_per_pixel != 4 && !(config[2] == 16 && source.bytes_per_pixel == 8))
        {
            return None;
        }
        let output = crate::gpu_image::Image::new(&self.device, source.width, source.height, 4);
        let data: Vec<_> = config.iter().flat_map(|v| v.to_le_bytes()).collect();
        let params = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Frame graph effect"),
                contents: &data,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let aux = auxiliary.map_or(&self.flat, |image| &image.buffer);
        let entries: Vec<_> = [&source.buffer, &params, &output.buffer, aux]
            .into_iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frame graph effect"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(source.width.div_ceil(16), source.height.div_ceil(16), 1);
        }
        self.queue.submit([encoder.finish()]);
        Some(output)
    }
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Option<Self> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let shader_source = include_str!("gpu_post.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Resident post-processing"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Resident post-processing"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let internal_error = block_on(internal.pop());
        let validation_error = block_on(scope.pop());
        if let Some(error) = internal_error.or(validation_error) {
            eprintln!("Post GPU validation: {error}");
            return None;
        }
        Some(Self {
            flat: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Empty post auxiliary"),
                contents: &[0; 4],
                usage: wgpu::BufferUsages::STORAGE,
            }),
            device,
            queue,
            pipeline,
            buffers: Mutex::new(None),
            failed: AtomicBool::new(false),
        })
    }
    pub fn apply(&self, pixels: &mut [u8], configs: &[Vec<u32>]) -> bool {
        let Some(config) = configs.first() else {
            return true;
        };
        let (w, h) = (config[0], config[1]);
        let bytes = pixels.len() as u64;
        let limits = self.device.limits();
        if self.failed.load(Ordering::Relaxed)
            || w == 0
            || h == 0
            || configs.len() > 64
            || bytes != u64::from(w) * u64::from(h) * 4
            || bytes > 64 * 1024 * 1024
            || bytes > limits.max_storage_buffer_binding_size
            || bytes > limits.max_buffer_size
            || w.div_ceil(16).max(h.div_ceil(16)) > limits.max_compute_workgroups_per_dimension
            || configs
                .iter()
                .any(|c| c.len() < 16 || c[..2] != config[..2])
        {
            return false;
        }
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let allocation = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let result = self.dispatch(pixels, configs);
        let bad = block_on(allocation.pop()).is_some() | block_on(validation.pop()).is_some();
        if let Some(result) = result.filter(|_| !bad) {
            pixels.copy_from_slice(&result);
            true
        } else {
            self.failed.store(true, Ordering::Relaxed);
            *self.buffers.lock().unwrap_or_else(|e| e.into_inner()) = None;
            false
        }
    }
    fn dispatch(&self, pixels: &[u8], configs: &[Vec<u32>]) -> Option<Vec<u8>> {
        let device = &self.device;
        let bytes = pixels.len() as u64;
        let mut buffers = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
        if buffers.as_ref().is_none_or(|b| b.bytes != bytes) {
            let make = |label, usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: bytes,
                    usage,
                    mapped_at_creation: false,
                })
            };
            let usage = wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST;
            *buffers = Some(Buffers {
                bytes,
                a: make("Post input", usage),
                b: make("Post output", usage),
                read: make(
                    "Post readback",
                    wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                ),
            });
        }
        let buffers = buffers.as_ref()?;
        self.queue.write_buffer(&buffers.a, 0, pixels);
        let mut encoder = device.create_command_encoder(&Default::default());
        for (index, config) in configs.iter().enumerate() {
            let data: Vec<_> = config.iter().flat_map(|v| v.to_le_bytes()).collect();
            let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Post parameters"),
                contents: &data,
                usage: wgpu::BufferUsages::STORAGE,
            });
            let pair = if index % 2 == 0 {
                (&buffers.a, &buffers.b)
            } else {
                (&buffers.b, &buffers.a)
            };
            let entries: Vec<_> = [pair.0, &params, pair.1, &self.flat]
                .into_iter()
                .enumerate()
                .map(|(binding, b)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: b.as_entire_binding(),
                })
                .collect();
            let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Post chain"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &entries,
            });
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(config[0].div_ceil(16), config[1].div_ceil(16), 1);
        }
        let output = if configs.len() % 2 == 1 {
            &buffers.b
        } else {
            &buffers.a
        };
        encoder.copy_buffer_to_buffer(output, 0, &buffers.read, 0, bytes);
        let submission = self.queue.submit([encoder.finish()]);
        let slice = buffers.read.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(2)),
            })
            .ok()?;
        rx.recv_timeout(std::time::Duration::from_secs(2))
            .ok()?
            .ok()?;
        let mapped = slice.get_mapped_range().ok()?;
        let output = mapped.to_vec();
        drop(mapped);
        buffers.read.unmap();
        Some(output)
    }
}
