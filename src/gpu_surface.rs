//! Exact separable Euclidean distance transform and the existing bevel kernel
//! on compute. Row/column envelopes use independent storage, not jump flooding.
use crate::{gpu_lens::block_on, light::Surface};
use eframe::{
    egui::Vec2,
    wgpu::{self, util::DeviceExt},
};
use std::{sync::Mutex, time::Duration};
struct Workspace {
    dimensions: [u32; 4],
    source: wgpu::Buffer,
    field: wgpu::Buffer,
    sites: wgpu::Buffer,
    boundaries: wgpu::Buffer,
    normals: wgpu::Buffer,
    read: wgpu::Buffer,
}
pub(crate) struct GpuSurface {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipelines: Vec<wgpu::ComputePipeline>,
    workspace: Mutex<Option<Workspace>>,
}
impl GpuSurface {
    pub(crate) fn outline_native(
        &self,
        source: &std::sync::Arc<crate::gpu_image::Image>,
        distance: f32,
        feather: f32,
        opacity: f32,
        color: [u8; 3],
    ) -> Option<std::sync::Arc<crate::gpu_image::Image>> {
        let (w, h) = (source.width, source.height);
        let field_bytes = (w as u64 + 2) * (h as u64 + 2) * 16;
        let scratch = ((w as u64 + 3) * (h as u64 + 2)).max((h as u64 + 3) * (w as u64 + 2)) * 4;
        if field_bytes > self.device.limits().max_storage_buffer_binding_size
            || source.bytes() > 32 * 1024 * 1024
        {
            return None;
        }
        let make = |bytes| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Outline distance field"),
                size: bytes,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let field = make(field_bytes);
        let sites = make(scratch);
        let boundaries = make(scratch);
        let output = crate::gpu_image::Image::new(&self.device, w, h, 8);
        let data = [
            w,
            h,
            w,
            h,
            1,
            1f32.to_bits(),
            2,
            0,
            0,
            0,
            1,
            distance.to_bits(),
            feather.to_bits(),
            opacity.to_bits(),
            (f32::from(color[0]) / 255.).to_bits(),
            (f32::from(color[1]) / 255.).to_bits(),
            (f32::from(color[2]) / 255.).to_bits(),
        ];
        let data: Vec<_> = data.into_iter().flat_map(u32::to_le_bytes).collect();
        let params = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Outline parameters"),
                contents: &data,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let entries: Vec<_> = [
            &source.buffer,
            &params,
            &field,
            &sites,
            &boundaries,
            &output.buffer,
            &source.buffer,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: b.as_entire_binding(),
        })
        .collect();
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Outline distance field"),
            layout: &self.layout,
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for (index, x, y) in [
            (0, (w + 2).div_ceil(16), (h + 2).div_ceil(16)),
            (1, (h + 2).div_ceil(64), 1),
            (2, (w + 2).div_ceil(64), 1),
            (7, w.div_ceil(16), h.div_ceil(16)),
        ] {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipelines[index]);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(x, y, 1);
        }
        self.queue.submit([encoder.finish()]);
        Some(output)
    }
    pub(crate) fn native(
        &self,
        source: &std::sync::Arc<crate::gpu_image::Image>,
        bevel: f32,
        texture: f32,
        height_map: Option<&crate::gpu_image::Image>,
        step: usize,
    ) -> Option<std::sync::Arc<crate::gpu_image::Image>> {
        let (w, h) = (
            source.width.div_ceil(step as u32),
            source.height.div_ceil(step as u32),
        );
        let field = (w as usize + 2) * (h as usize + 2) * 16;
        let scratch =
            ((w as usize + 3) * (h as usize + 2)).max((h as usize + 3) * (w as usize + 2)) * 4;
        let normals = w as usize * h as usize * 8;
        let limits = self.device.limits();
        if source.bytes_per_pixel != 4
            || normals > 32 * 1024 * 1024
            || field.max(scratch) as u64 > limits.max_storage_buffer_binding_size
        {
            return None;
        }
        let dimensions = [source.width, source.height, w, h];
        let device = &self.device;
        let mut workspace = self.workspace.lock().unwrap_or_else(|e| e.into_inner());
        if workspace
            .as_ref()
            .is_none_or(|ws| ws.dimensions != dimensions)
        {
            let make = |size, usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Resident relief workspace"),
                    size: size as u64,
                    usage,
                    mapped_at_creation: false,
                })
            };
            *workspace = Some(Workspace {
                dimensions,
                source: make(
                    4,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
                field: make(field, wgpu::BufferUsages::STORAGE),
                sites: make(scratch, wgpu::BufferUsages::STORAGE),
                boundaries: make(scratch, wgpu::BufferUsages::STORAGE),
                normals: make(
                    8,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                ),
                read: make(
                    8,
                    wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                ),
            });
        }
        let workspace = workspace.as_ref()?;
        let output = crate::gpu_image::Image::new(device, w, h, 8);
        let radius = ((bevel / step as f32).max(1.) * 0.22)
            .round()
            .clamp(2., 12.) as u32;
        let data: Vec<_> = dimensions
            .into_iter()
            .chain([
                step as u32,
                bevel.to_bits(),
                radius,
                0,
                texture.to_bits(),
                u32::from(height_map.is_some()),
            ])
            .flat_map(u32::to_le_bytes)
            .collect();
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Resident relief"),
            contents: &data,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let entries: Vec<_> = [
            &source.buffer,
            &params,
            &workspace.field,
            &workspace.sites,
            &workspace.boundaries,
            &output.buffer,
            height_map.map_or(&source.buffer, |image| &image.buffer),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: b.as_entire_binding(),
        })
        .collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Resident relief"),
            layout: &self.layout,
            entries: &entries,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        for (index, x, y) in [
            (0, (w + 2).div_ceil(16), (h + 2).div_ceil(16)),
            (1, (h + 2).div_ceil(64), 1),
            (2, (w + 2).div_ceil(64), 1),
            (3, w.div_ceil(16), h.div_ceil(16)),
            (4, h.div_ceil(64), 1),
            (5, w.div_ceil(64), 1),
            (4, h.div_ceil(64), 1),
            (5, w.div_ceil(64), 1),
            (6, w.div_ceil(16), h.div_ceil(16)),
        ] {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipelines[index]);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(x, y, 1);
        }
        self.queue.submit([encoder.finish()]);
        Some(output)
    }
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Option<Self> {
        if device.limits().max_storage_buffers_per_shader_stage < 7 {
            return None;
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader_source = include_str!("gpu_surface.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Stable world-space relief"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let entries: Vec<_> = (0..7)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: binding < 2 || binding == 6,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Relief workspace"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Relief workspace"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipelines = [
            "coverage",
            "rows",
            "columns",
            "height_values",
            "smooth_x",
            "smooth_y",
            "slopes",
            "outline",
        ]
        .into_iter()
        .map(|entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        })
        .collect();
        if let Some(error) = block_on(scope.pop()) {
            eprintln!("Surface GPU validation: {error}");
            return None;
        }
        Some(Self {
            device,
            queue,
            layout,
            pipelines,
            workspace: Mutex::new(None),
        })
    }
    pub fn surface(
        &self,
        pixels: &[u8],
        width: usize,
        height: usize,
        bevel: f32,
        texture: f32,
        height_map: Option<&[u8]>,
        step: usize,
    ) -> Option<Surface> {
        let (w, h) = (width.div_ceil(step), height.div_ceil(step));
        let limits = self.device.limits();
        let field = (w + 2) * (h + 2) * 16;
        let scratch = ((w + 3) * (h + 2)).max((h + 3) * (w + 2)) * 4;
        let normals = w * h * 8;
        if pixels.len() != width * height * 4
            || height_map.is_some_and(|map| map.len() != pixels.len())
            || w == 0
            || h == 0
            || normals > 32 * 1024 * 1024
            || field.max(scratch).max(normals).max(pixels.len()) as u64
                > limits.max_storage_buffer_binding_size
            || field
                + scratch * 2
                + normals * 2
                + pixels.len()
                + height_map.map_or(0, |map| map.len())
                > 192 * 1024 * 1024
        {
            return None;
        }
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let allocation = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let result = self.dispatch(
            pixels,
            [width as u32, height as u32, w as u32, h as u32],
            bevel,
            texture,
            height_map,
            step,
            field,
            scratch,
            normals,
        );
        let bad = block_on(allocation.pop()).is_some() | block_on(validation.pop()).is_some();
        if bad { None } else { result }
    }
    fn dispatch(
        &self,
        pixels: &[u8],
        dimensions: [u32; 4],
        bevel: f32,
        texture: f32,
        height_map: Option<&[u8]>,
        step: usize,
        field: usize,
        scratch: usize,
        normals: usize,
    ) -> Option<Surface> {
        let device = &self.device;
        let mut workspace = self.workspace.lock().unwrap_or_else(|e| e.into_inner());
        if workspace.as_ref().is_none_or(|w| {
            w.dimensions != dimensions
                || w.source.size() < pixels.len() as u64
                || w.normals.size() < normals as u64
                || w.read.size() < normals as u64
        }) {
            let make = |size, usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Relief workspace"),
                    size: size as u64,
                    usage,
                    mapped_at_creation: false,
                })
            };
            *workspace = Some(Workspace {
                dimensions,
                source: make(
                    pixels.len(),
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
                field: make(field, wgpu::BufferUsages::STORAGE),
                sites: make(scratch, wgpu::BufferUsages::STORAGE),
                boundaries: make(scratch, wgpu::BufferUsages::STORAGE),
                normals: make(
                    normals,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                ),
                read: make(
                    normals,
                    wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                ),
            });
        }
        let workspace = workspace.as_ref()?;
        let [_, _, w, h] = dimensions;
        let limit = (bevel / step as f32).max(1.);
        let radius = (limit * 0.22).round().clamp(2., 12.) as u32;
        let data: Vec<_> = dimensions
            .into_iter()
            .chain([
                step as u32,
                bevel.to_bits(),
                radius,
                0,
                texture.to_bits(),
                u32::from(height_map.is_some()),
            ])
            .flat_map(u32::to_le_bytes)
            .collect();
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Relief parameters"),
            contents: &data,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let height_map = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Relief height guide"),
            contents: height_map.unwrap_or(&[0; 4]),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let entries: Vec<_> = [
            &workspace.source,
            &params,
            &workspace.field,
            &workspace.sites,
            &workspace.boundaries,
            &workspace.normals,
            &height_map,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: b.as_entire_binding(),
        })
        .collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Relief workspace"),
            layout: &self.layout,
            entries: &entries,
        });
        self.queue.write_buffer(&workspace.source, 0, pixels);
        let mut encoder = device.create_command_encoder(&Default::default());
        for (index, x, y) in [
            (0, (w + 2).div_ceil(16), (h + 2).div_ceil(16)),
            (1, (h + 2).div_ceil(64), 1),
            (2, (w + 2).div_ceil(64), 1),
            (3, w.div_ceil(16), h.div_ceil(16)),
            (4, h.div_ceil(64), 1),
            (5, w.div_ceil(64), 1),
            (4, h.div_ceil(64), 1),
            (5, w.div_ceil(64), 1),
            (6, w.div_ceil(16), h.div_ceil(16)),
        ] {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipelines[index]);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(x, y, 1);
        }
        encoder.copy_buffer_to_buffer(&workspace.normals, 0, &workspace.read, 0, normals as u64);
        let submission = self.queue.submit([encoder.finish()]);
        let slice = workspace.read.slice(..);
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
        let slopes = mapped
            .chunks_exact(8)
            .map(|p| {
                Vec2::new(
                    f32::from_le_bytes(p[..4].try_into().unwrap()),
                    f32::from_le_bytes(p[4..].try_into().unwrap()),
                )
            })
            .collect::<Vec<_>>();
        drop(mapped);
        workspace.read.unmap();
        if slopes.iter().any(|p| !p.is_finite()) {
            return None;
        }
        Some(Surface {
            width: w as usize,
            height: h as usize,
            step,
            slopes,
        })
    }
}
