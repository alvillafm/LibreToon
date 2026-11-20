//! Hardware rasterization of the same clipped, antialiased meshes as the stage.
use crate::gpu_lens::block_on;
use eframe::{
    egui,
    wgpu::{self, util::DeviceExt},
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
type TexturePixels<'a> = Option<([usize; 2], &'a [u8])>;
struct Target {
    size: [u32; 2],
    texture: wgpu::Texture,
    stride: u32,
}
pub(crate) struct GpuRaster {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    target: Mutex<Option<Target>>,
    failed: AtomicBool,
}
impl GpuRaster {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Option<Self> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vector artwork GPU rasterizer"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_raster.wgsl").into()),
        });
        let attributes = wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Unorm8x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vector artwork GPU rasterizer"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        if let Some(error) = block_on(scope.pop()) {
            eprintln!("Raster GPU validation: {error}");
            return None;
        }
        Some(Self {
            device,
            queue,
            pipeline,
            target: Mutex::new(None),
            failed: AtomicBool::new(false),
        })
    }
    pub fn render<'a>(
        &self,
        w: u32,
        h: u32,
        transparent: bool,
        meshes: &[egui::ClippedPrimitive],
        textures: impl Fn(egui::TextureId) -> TexturePixels<'a>,
    ) -> Option<Vec<u8>> {
        let _span = crate::render_profile::span("GPU raster");
        let limits = self.device.limits();
        if self.failed.load(Ordering::Relaxed)
            || w == 0
            || h == 0
            || w.max(h) > limits.max_texture_dimension_2d
            || u64::from(w) * u64::from(h) * 4 > 64 * 1024 * 1024
        {
            return None;
        }
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let allocation = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let result = self.dispatch(w, h, transparent, meshes, textures);
        let bad = block_on(allocation.pop()).is_some() | block_on(validation.pop()).is_some();
        if bad || result.is_none() {
            self.failed.store(true, Ordering::Relaxed);
            return None;
        }
        let image = result?;
        let data = image.read(&self.device, &self.queue)?;
        let mut pixels =
            Vec::with_capacity(w as usize * h as usize * if transparent { 4 } else { 3 });
        for row in data.chunks_exact(image.width as usize * 4) {
            if transparent {
                pixels.extend_from_slice(&row[..w as usize * 4]);
            } else {
                for p in row[..w as usize * 4].chunks_exact(4) {
                    pixels.extend_from_slice(&p[..3]);
                }
            }
        }
        Some(pixels)
    }
    pub(crate) fn native<'a>(
        &self,
        w: u32,
        h: u32,
        meshes: &[egui::ClippedPrimitive],
        textures: impl Fn(egui::TextureId) -> TexturePixels<'a>,
    ) -> Option<std::sync::Arc<crate::gpu_image::Image>> {
        if w == 0
            || h == 0
            || w.max(h) > self.device.limits().max_texture_dimension_2d
            || u64::from(w) * u64::from(h) * 4 > 64 * 1024 * 1024
        {
            return None;
        }
        self.dispatch(w, h, true, meshes, textures)
    }
    fn dispatch<'a>(
        &self,
        w: u32,
        h: u32,
        transparent: bool,
        meshes: &[egui::ClippedPrimitive],
        textures: impl Fn(egui::TextureId) -> TexturePixels<'a>,
    ) -> Option<std::sync::Arc<crate::gpu_image::Image>> {
        let device = &self.device;
        let mut target = self.target.lock().unwrap_or_else(|e| e.into_inner());
        if target.as_ref().is_none_or(|t| t.size != [w, h]) {
            let stride = (w * 4).div_ceil(256) * 256;
            *target = Some(Target {
                size: [w, h],
                stride,
                texture: device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Artwork plane"),
                    size: wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                }),
            });
        }
        let target = target.as_ref()?;
        let view = target.texture.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        struct Draw {
            vertex: wgpu::Buffer,
            index: wgpu::Buffer,
            bindings: wgpu::BindGroup,
            count: u32,
            clip: [u32; 4],
        }
        let mut draws = Vec::new();
        for primitive in meshes {
            let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
                return None;
            };
            if mesh.indices.is_empty() {
                continue;
            }
            let clip = primitive.clip_rect.intersect(egui::Rect::from_min_max(
                egui::Pos2::ZERO,
                egui::pos2(w as f32, h as f32),
            ));
            if !clip.is_positive() {
                continue;
            }
            let x = clip.min.x.floor().max(0.) as u32;
            let y = clip.min.y.floor().max(0.) as u32;
            let right = (clip.max.x.ceil() as u32).min(w);
            let bottom = (clip.max.y.ceil() as u32).min(h);
            if right <= x || bottom <= y {
                continue;
            }
            let mut vertices = Vec::with_capacity(mesh.vertices.len() * 20);
            for vertex in &mesh.vertices {
                for value in [
                    vertex.pos.x * 2. / w as f32 - 1.,
                    1. - vertex.pos.y * 2. / h as f32,
                    vertex.uv.x,
                    vertex.uv.y,
                ] {
                    vertices.extend(value.to_le_bytes());
                }
                vertices.extend(vertex.color.to_array());
            }
            let indices: Vec<_> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
            let buffer = |label, data: &[u8], usage| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: data,
                    usage,
                })
            };
            let pixels = textures(mesh.texture_id);
            let flag = u32::from(pixels.is_some());
            let (size, pixels) = pixels.unwrap_or(([1, 1], &[255; 4]));
            if size.contains(&0)
                || size[0].max(size[1]) > device.limits().max_texture_dimension_2d as usize
                || pixels.len() != size[0] * size[1] * 4
            {
                return None;
            }
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Artwork image"),
                size: wgpu::Extent3d {
                    width: size[0] as u32,
                    height: size[1] as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size[0] as u32 * 4),
                    rows_per_image: Some(size[1] as u32),
                },
                texture.size(),
            );
            let image = texture.create_view(&Default::default());
            let flags = buffer(
                "Artwork texture flag",
                &[flag, 0, 0, 0]
                    .into_iter()
                    .flat_map(u32::to_le_bytes)
                    .collect::<Vec<_>>(),
                wgpu::BufferUsages::UNIFORM,
            );
            let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Artwork sampling"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&image),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: flags.as_entire_binding(),
                    },
                ],
            });
            draws.push(Draw {
                vertex: buffer("Artwork vertices", &vertices, wgpu::BufferUsages::VERTEX),
                index: buffer("Artwork indices", &indices, wgpu::BufferUsages::INDEX),
                bindings,
                count: mesh.indices.len() as u32,
                clip: [x, y, right - x, bottom - y],
            });
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Artwork raster"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(if transparent {
                            wgpu::Color::TRANSPARENT
                        } else {
                            wgpu::Color::WHITE
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            for draw in &draws {
                pass.set_scissor_rect(draw.clip[0], draw.clip[1], draw.clip[2], draw.clip[3]);
                pass.set_bind_group(0, &draw.bindings, &[]);
                pass.set_vertex_buffer(0, draw.vertex.slice(..));
                pass.set_index_buffer(draw.index.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..draw.count, 0, 0..1);
            }
        }
        let image = crate::gpu_image::Image::new(device, target.stride / 4, h, 4);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &image.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(target.stride),
                    rows_per_image: Some(h),
                },
            },
            target.texture.size(),
        );
        self.queue.submit([encoder.finish()]);
        Some(image)
    }
}
