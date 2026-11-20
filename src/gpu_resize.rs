//! Triangle reconstruction matching the reference resizer, including alpha.
use crate::{gpu_image::Image, gpu_lens::block_on};
use eframe::wgpu::{self, util::DeviceExt};
use std::sync::Arc;
pub(crate) struct GpuResize {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipelines: [wgpu::ComputePipeline; 2],
}
impl GpuResize {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Option<Self> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle reconstruction"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_resize.wgsl").into()),
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
            label: Some("Triangle reconstruction"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Triangle reconstruction"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipelines = ["vertical", "horizontal"].map(|entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Triangle reconstruction"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        });
        if let Some(error) = block_on(scope.pop()) {
            eprintln!("Resize GPU validation: {error}");
            return None;
        }
        Some(Self {
            device,
            queue,
            layout,
            pipelines,
        })
    }
    pub fn native(&self, source: &Arc<Image>, size: [u32; 2]) -> Option<Arc<Image>> {
        if size.contains(&0)
            || source.width as u64 * size[1] as u64 * 16
                > self.device.limits().max_storage_buffer_binding_size
        {
            return None;
        }
        let output = Image::new(&self.device, size[0], size[1], 4);
        let temporary = Image::new(&self.device, source.width, size[1], 16);
        let config: Vec<_> = [source.width, source.height, size[0], size[1]]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        let params = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Resize dimensions"),
                contents: &config,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let entries: Vec<_> = [&source.buffer, &params, &temporary.buffer, &output.buffer]
            .into_iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Triangle reconstruction"),
            layout: &self.layout,
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for (index, width) in [(0, source.width), (1, size[0])] {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipelines[index]);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(width.div_ceil(16), size[1].div_ceil(16), 1);
        }
        self.queue.submit([encoder.finish()]);
        Some(output)
    }
}
