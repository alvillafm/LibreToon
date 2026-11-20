//! Bounded device-resident world planes. Camera movement updates six floats;
//! it does not upload or resample the source artwork on the CPU.
use crate::gpu_lens::block_on;
use eframe::{
    egui::{Pos2, Vec2},
    wgpu::{self, util::DeviceExt},
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
struct Source {
    owner: Arc<Vec<u8>>,
    size: [u32; 2],
    buffer: wgpu::Buffer,
}
struct Output {
    bytes: u64,
    buffer: wgpu::Buffer,
    read: wgpu::Buffer,
}
#[derive(Default)]
struct State {
    sources: VecDeque<Source>,
    output: Option<Output>,
}
pub(crate) struct GpuPlane {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    state: Mutex<State>,
}
impl GpuPlane {
    pub(crate) fn native(
        &self,
        source: &std::sync::Arc<crate::gpu_image::Image>,
        output: [u32; 2],
        start: Pos2,
        dx: Vec2,
        dy: Vec2,
    ) -> Option<std::sync::Arc<crate::gpu_image::Image>> {
        if source.bytes_per_pixel != 4 {
            return None;
        }
        let image = crate::gpu_image::Image::new(&self.device, output[0], output[1], 4);
        let mut data = vec![output[0], output[1], source.width, source.height];
        data.extend(
            [start.x, start.y, dx.x, dx.y, dy.x, dy.y]
                .into_iter()
                .map(f32::to_bits),
        );
        let data: Vec<_> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let params = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Frame graph camera"),
                contents: &data,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let entries: Vec<_> = [&source.buffer, &params, &image.buffer]
            .into_iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frame graph camera"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(output[0].div_ceil(16), output[1].div_ceil(16), 1);
        }
        self.queue.submit([encoder.finish()]);
        Some(image)
    }
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Option<Self> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Resident camera planes"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_plane.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Resident camera planes"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(error) = block_on(scope.pop()) {
            eprintln!("Plane GPU validation: {error}");
            return None;
        }
        Some(Self {
            device,
            queue,
            pipeline,
            state: Mutex::new(State::default()),
        })
    }
    pub fn project(
        &self,
        source: &Arc<Vec<u8>>,
        source_size: [u32; 2],
        output: [u32; 2],
        start: Pos2,
        dx: Vec2,
        dy: Vec2,
    ) -> Option<Vec<u8>> {
        let bytes = u64::from(output[0]) * u64::from(output[1]) * 4;
        let limits = self.device.limits();
        if source.len() as u64 != u64::from(source_size[0]) * u64::from(source_size[1]) * 4
            || output.contains(&0)
            || bytes.max(source.len() as u64) > 64 * 1024 * 1024
            || bytes.max(source.len() as u64) > limits.max_storage_buffer_binding_size
            || output[0].div_ceil(16).max(output[1].div_ceil(16))
                > limits.max_compute_workgroups_per_dimension
        {
            return None;
        }
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let allocation = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let result = self.dispatch(source, source_size, output, start, dx, dy, bytes);
        let bad = block_on(allocation.pop()).is_some() | block_on(validation.pop()).is_some();
        if bad { None } else { result }
    }
    fn dispatch(
        &self,
        source: &Arc<Vec<u8>>,
        source_size: [u32; 2],
        output: [u32; 2],
        start: Pos2,
        dx: Vec2,
        dy: Vec2,
        bytes: u64,
    ) -> Option<Vec<u8>> {
        let device = &self.device;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(index) = state
            .sources
            .iter()
            .position(|s| Arc::ptr_eq(&s.owner, source) && s.size == source_size)
        {
            let s = state.sources.remove(index)?;
            state.sources.push_back(s);
        } else {
            let mut total =
                source.len() + state.sources.iter().map(|s| s.owner.len()).sum::<usize>();
            while total > 96 * 1024 * 1024 || state.sources.len() >= 32 {
                total -= state.sources.pop_front()?.owner.len();
            }
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Resident lit plane"),
                contents: source,
                usage: wgpu::BufferUsages::STORAGE,
            });
            state.sources.push_back(Source {
                owner: source.clone(),
                size: source_size,
                buffer,
            });
        }
        if state.output.as_ref().is_none_or(|o| o.bytes != bytes) {
            let make = |usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Projected plane"),
                    size: bytes,
                    usage,
                    mapped_at_creation: false,
                })
            };
            state.output = Some(Output {
                bytes,
                buffer: make(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC),
                read: make(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST),
            });
        }
        let mut data = vec![output[0], output[1], source_size[0], source_size[1]];
        data.extend(
            [start.x, start.y, dx.x, dx.y, dy.x, dy.y]
                .into_iter()
                .map(f32::to_bits),
        );
        let data: Vec<_> = data.iter().flat_map(|i| i.to_le_bytes()).collect();
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera transform"),
            contents: &data,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let target = state.output.as_ref()?;
        let source = &state.sources.back()?.buffer;
        let entries: Vec<_> = [source, &params, &target.buffer]
            .into_iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera plane"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(output[0].div_ceil(16), output[1].div_ceil(16), 1);
        }
        encoder.copy_buffer_to_buffer(&target.buffer, 0, &target.read, 0, bytes);
        let submission = self.queue.submit([encoder.finish()]);
        let slice = target.read.slice(..);
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
        let result = mapped.to_vec();
        drop(mapped);
        target.read.unmap();
        Some(result)
    }
}
