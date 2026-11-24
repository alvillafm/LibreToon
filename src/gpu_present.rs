//! Resident viewport presentation. Shared-device work runs only in egui prepare callbacks.
//! Geometry is prepared off-thread; the presenter never maps a completed image to CPU.
use crate::{Document, Mark, canvas::View, gpu_image::Image};
use eframe::{
    egui::{self, Pos2, Rect},
    egui_wgpu,
    wgpu::{self, util::DeviceExt},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

#[derive(Clone)]
pub(crate) struct Request {
    pub key: Vec<usize>,
    pub doc: Document,
    pub frame: usize,
    pub clock: usize,
    pub size: [u32; 2],
    pub video: Vec<crate::media::ExportVideoFrame>,
    pub camera: bool,
    pub post: bool,
    pub blur: bool,
    pub full: bool,
    pub budget: usize,
}
struct Prepared {
    request: Request,
    world: Vec<(usize, usize, Mark)>,
    geometry: Mutex<crate::paint::RenderCache>,
}
struct Frame {
    key: Vec<usize>,
    image: Arc<Image>,
}
struct State {
    gpu: Arc<crate::gpu_lens::GpuLens>,
    format: wgpu::TextureFormat,
    // art_meshes runs egui passes. Never give it the live editor context:
    // doing so replaces input, viewport geometry and widget state per layer.
    render_ctx: egui::Context,
    graph: crate::gpu_scene::Cache,
    pipeline: Option<wgpu::RenderPipeline>,
    bind: Option<wgpu::BindGroup>,
    uniform: Option<wgpu::Buffer>,
    next: Option<Arc<Prepared>>,
    current: Option<Frame>,
    cached: VecDeque<Frame>,
    failed: bool,
    failed_notified: bool,
}
impl State {
    fn resident_bytes(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        let mut bytes = self.graph.resident_bytes_with(&mut seen);
        for frame in self.cached.iter().chain(self.current.iter()) {
            if seen.insert(Arc::as_ptr(&frame.image) as usize) {
                bytes = bytes.saturating_add(frame.image.bytes());
            }
        }
        bytes
    }
    fn trim_to(&mut self, budget: usize) {
        // Keep the displayed frame even when one image exceeds the allowance.
        // Drop old completed frames before geometry needed by the next frame.
        while self.resident_bytes() > budget {
            let removable = self.cached.iter().position(|frame| {
                self.current
                    .as_ref()
                    .is_none_or(|current| !Arc::ptr_eq(&current.image, &frame.image))
            });
            if let Some(index) = removable {
                self.cached.remove(index);
            } else {
                break;
            }
        }
        let frames = self
            .resident_bytes()
            .saturating_sub(self.graph.resident_bytes());
        self.graph.trim_to(budget.saturating_sub(frames));
    }
}
pub(crate) struct Presenter {
    state: Arc<Mutex<State>>,
    pending: Option<(Vec<usize>, mpsc::Receiver<Prepared>)>,
    target: Vec<usize>,
    geometry: Arc<Mutex<crate::paint::RenderCache>>,
    generation: Arc<AtomicUsize>,
    progress: Arc<AtomicUsize>,
    show_progress: bool,
}
impl Presenter {
    #[cfg(test)]
    pub(crate) fn shown_key(&self) -> Option<Vec<usize>> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .current
            .as_ref()
            .map(|f| f.key.clone())
    }
    #[cfg(test)]
    pub(crate) fn fail_for_test(&self) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).failed = true;
    }
    pub(crate) fn new(render: &egui_wgpu::RenderState) -> Option<Self> {
        let limits = render.device.limits();
        if render.adapter.get_info().device_type == wgpu::DeviceType::Cpu
            || !render
                .adapter
                .get_downlevel_capabilities()
                .flags
                .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
            || limits.max_storage_buffers_per_shader_stage < 3
            || limits.max_compute_invocations_per_workgroup < 256
            || limits.max_compute_workgroup_size_x < 16
            || limits.max_compute_workgroup_size_y < 16
        {
            return None;
        }
        Some(Self {
            state: Arc::new(Mutex::new(State {
                gpu: Arc::new(crate::gpu_lens::GpuLens::new(
                    render.device.clone(),
                    render.queue.clone(),
                )),
                format: render.target_format,
                render_ctx: egui::Context::default(),
                graph: Default::default(),
                pipeline: None,
                bind: None,
                uniform: None,
                next: None,
                current: None,
                cached: VecDeque::new(),
                failed: false,
                failed_notified: false,
            })),
            pending: None,
            target: Vec::new(),
            geometry: Default::default(),
            generation: Default::default(),
            progress: Default::default(),
            show_progress: false,
        })
    }
    pub(crate) fn take_failure(&self) -> Option<&'static str> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.failed && !state.failed_notified {
            state.failed_notified = true;
            Some("Direct GPU preview failed. Using the compatible preview renderer.")
        } else {
            None
        }
    }
    pub(crate) fn available(&self) -> bool {
        !self.state.lock().unwrap_or_else(|e| e.into_inner()).failed
    }
    pub(crate) fn reset(&mut self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        self.pending = None;
        self.target.clear();
        self.show_progress = false;
        self.geometry = Default::default();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.graph = Default::default();
        state.next = None;
        state.current = None;
        state.cached.clear();
        state.bind = None;
        state.failed = false;
        state.failed_notified = false;
    }
    pub(crate) fn retain_frames(&mut self, epoch: usize, mut valid: impl FnMut(usize) -> bool) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        self.target.clear();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.next = None;
        state.cached.retain_mut(|frame| {
            if frame.key.get(1).is_some_and(|&index| valid(index)) {
                frame.key[0] = epoch;
                true
            } else {
                false
            }
        });
        // A stale displayed image stays visible while its replacement renders,
        // but only an unaffected image is marked as belonging to the new epoch.
        if let Some(frame) = &mut state.current
            && frame.key.get(1).is_some_and(|&index| valid(index))
        {
            frame.key[0] = epoch;
        }
    }
    pub(crate) fn paint(
        &mut self,
        request: Request,
        painter: &egui::Painter,
        stage: View,
        ram_ready: bool,
    ) -> bool {
        if !self.available() {
            return false;
        }
        let ctx = painter.ctx();
        if self.target != request.key {
            self.target = request.key.clone();
            self.generation.fetch_add(1, Ordering::Relaxed);
        }
        if let Some((submitted, job)) = &self.pending {
            match job.try_recv() {
                Ok(prepared) => {
                    self.pending = None;
                    if prepared.request.key == request.key {
                        self.state.lock().unwrap_or_else(|e| e.into_inner()).next =
                            Some(Arc::new(prepared));
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    let failed = *submitted == request.key;
                    self.pending = None;
                    if failed {
                        self.state.lock().unwrap_or_else(|e| e.into_inner()).failed = true;
                    }
                    ctx.request_repaint();
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(15));
                }
            }
        }
        if !self.available() {
            return false;
        }
        let needed = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(index) = state.cached.iter().position(|f| f.key == request.key) {
                let frame = state.cached.remove(index).unwrap();
                state.current = Some(Frame {
                    key: frame.key.clone(),
                    image: frame.image.clone(),
                });
                state.cached.push_back(frame);
            }
            state.current.as_ref().is_none_or(|f| f.key != request.key)
                && state
                    .next
                    .as_ref()
                    .is_none_or(|p| p.request.key != request.key)
        };
        self.show_progress = needed && !ram_ready;
        if needed && self.pending.is_none() {
            let (tx, rx) = mpsc::sync_channel(1);
            let snapshot = request.clone();
            let geometry = self.geometry.clone();
            let generation = self.generation.clone();
            let expected = generation.load(Ordering::Relaxed);
            let progress = self.progress.clone();
            progress.store(0, Ordering::Relaxed);
            let repaint = ctx.clone();
            let accepted = crate::geometry_jobs::spawn(move || {
                if generation.load(Ordering::Relaxed) != expected {
                    return;
                }
                let (scene, local) = crate::animation::locate(&snapshot.doc, snapshot.frame);
                let scene = crate::animation::scene_doc(&snapshot.doc, scene);
                let mut world =
                    crate::features::resolve_preview(&scene, local, false, snapshot.clock);
                if snapshot.camera {
                    if let Some(camera) = crate::depth::camera(&scene, local) {
                        let optics = crate::animation::camera_optics(camera, local);
                        world.retain(|(layer, _, _)| {
                            crate::depth::visible(&scene.layers[*layer].options.depth, &optics)
                        });
                    }
                }
                progress.store(25, Ordering::Relaxed);
                if generation.load(Ordering::Relaxed) != expected {
                    return;
                }
                let mut cache = geometry.lock().unwrap_or_else(|e| e.into_inner());
                cache.prepare_composited(&world);
                let prepared = cache.geometry_snapshot();
                drop(cache);
                progress.store(90, Ordering::Relaxed);
                if generation.load(Ordering::Relaxed) != expected {
                    return;
                }
                let request = Request {
                    doc: scene,
                    frame: local,
                    ..snapshot
                };
                let _ = tx.send(Prepared {
                    request,
                    world,
                    geometry: Mutex::new(prepared),
                });
                repaint.request_repaint();
            });
            if accepted {
                self.pending = Some((request.key.clone(), rx));
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
        let corners = [
            Pos2::ZERO,
            Pos2::new(request.doc.width as f32, 0.),
            Pos2::new(request.doc.width as f32, request.doc.height as f32),
            Pos2::new(0., request.doc.height as f32),
        ]
        .map(|p| stage.screen(p));
        let bounds = corners
            .iter()
            .fold(Rect::NOTHING, |r, &p| r.union(Rect::from_min_max(p, p)))
            .intersect(painter.clip_rect())
            .intersect(ctx.content_rect());
        if !bounds.is_positive() {
            return false;
        }
        let visible = self
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .current
            .as_ref()
            .is_some_and(|frame| frame.key == request.key || !ram_ready);
        painter.add(egui_wgpu::Callback::new_paint_callback(
            bounds,
            Draw {
                state: self.state.clone(),
                corners,
                rect: bounds,
                ctx: ctx.clone(),
                budget: request.budget,
            },
        ));
        visible
    }
    pub(crate) fn stats(&self) -> (usize, usize, usize, usize) {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        (
            state.resident_bytes(),
            state.cached.len(),
            state.gpu.composed_frames.load(Ordering::Relaxed),
            state.gpu.light_pass_count(),
        )
    }
    pub(crate) fn progress(&self) -> Option<usize> {
        if !self.show_progress || !self.available() {
            return None;
        }
        self.pending.as_ref().map(|(key, _)| {
            if *key == self.target {
                self.progress.load(Ordering::Relaxed)
            } else {
                0
            }
        })
    }
}

fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> Option<wgpu::RenderPipeline> {
    crate::gpu_image::checked_scope(device, || {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Resident viewport presentation"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_present.wgsl").into()),
        });
        Some(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Resident viewport presentation"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            }),
        )
    })
}
fn parameters(image: &Image, corners: [Pos2; 4], rect: Rect, srgb: bool) -> Vec<u8> {
    let mut data = Vec::with_capacity(80);
    for v in [image.width, image.height, u32::from(srgb), 0] {
        data.extend(v.to_le_bytes());
    }
    for p in corners {
        for value in [
            (p.x - rect.left()) / rect.width() * 2. - 1.,
            1. - (p.y - rect.top()) / rect.height() * 2.,
            0.,
            0.,
        ] {
            data.extend(value.to_le_bytes());
        }
    }
    data
}
struct Draw {
    state: Arc<Mutex<State>>,
    corners: [Pos2; 4],
    rect: Rect,
    ctx: egui::Context,
    budget: usize,
}
impl Draw {
    fn prepare_native(&self, device: &wgpu::Device) -> Vec<wgpu::CommandBuffer> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.failed {
            return Vec::new();
        }
        if state.pipeline.is_none() {
            state.pipeline = pipeline(device, state.format);
            if state.pipeline.is_none() {
                state.failed = true;
                self.ctx.request_repaint();
                return Vec::new();
            }
        }
        if let Some(prepared) = state.next.take() {
            let request = &prepared.request;
            let gpu = state.gpu.clone();
            let camera =
                crate::depth::camera(&request.doc, request.frame).filter(|_| request.camera);
            let mut geometry = prepared.geometry.lock().unwrap_or_else(|e| e.into_inner());
            let render_ctx = state.render_ctx.clone();
            let image = crate::gpu_scene::render_resident(
                &gpu,
                &request.doc,
                request.frame,
                request.size[0],
                request.size[1],
                &render_ctx,
                &mut geometry,
                &mut state.graph,
                &prepared.world,
                &request.video,
                camera,
                request.post,
                request.blur,
                request.full,
                false,
                None,
            );
            if let Some(image) = image {
                state.current = Some(Frame {
                    key: request.key.clone(),
                    image: image.clone(),
                });
                state.cached.retain(|f| f.key != request.key);
                state.cached.push_back(Frame {
                    key: request.key.clone(),
                    image,
                });
            } else {
                state.failed = true;
            }
            self.ctx.request_repaint();
        }
        state.trim_to(self.budget);
        if let Some(frame) = &state.current {
            let data = parameters(
                &frame.image,
                self.corners,
                self.rect,
                state.format.is_srgb(),
            );
            let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Viewport image placement"),
                contents: &data,
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Resident viewport image"),
                layout: &state.pipeline.as_ref().unwrap().get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: frame.image.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uniform.as_entire_binding(),
                    },
                ],
            });
            state.uniform = Some(uniform);
            state.bind = Some(bind);
        }
        Vec::new()
    }
}
impl egui_wgpu::CallbackTrait for Draw {
    fn prepare(
        &self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _: &egui_wgpu::ScreenDescriptor,
        _: &mut wgpu::CommandEncoder,
        _: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.prepare_native(device)))
        {
            Ok(commands) => commands,
            Err(_) => {
                self.state.lock().unwrap_or_else(|e| e.into_inner()).failed = true;
                self.ctx.request_repaint();
                Vec::new()
            }
        }
    }
    fn paint(
        &self,
        _: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        _: &egui_wgpu::CallbackResources,
    ) {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let (Some(pipeline), Some(bind)) = (&state.pipeline, &state.bind) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui_wgpu::CallbackTrait;
    #[test]
    #[ignore = "requires GPU; tests direct presentation on gamma and sRGB targets without scene readback"]
    fn resident_presentation_matches_reference_without_readback() {
        use crate::gpu_lens::{GpuLens, block_on, use_on_worker};
        let adapter =
            block_on(wgpu::Instance::default().request_adapter(&Default::default())).unwrap();
        let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
        let gpu = Arc::new(GpuLens::new(device.clone(), queue.clone()));
        use_on_worker(Some(gpu.clone()));
        let ctx = egui::Context::default();
        for example in [
            ":builtin-orbit-lights",
            ":builtin-woodland-depth",
            ":builtin-pip-rig",
        ] {
            let doc = crate::examples::project(example).unwrap().unwrap();
            let (w, h) = (doc.width, doc.height);
            let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(w as f32, h as f32));
            let corners = [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ];
            for format in [
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ] {
                let state = Arc::new(Mutex::new(State {
                    gpu: gpu.clone(),
                    format,
                    render_ctx: egui::Context::default(),
                    graph: Default::default(),
                    pipeline: None,
                    bind: None,
                    uniform: None,
                    next: None,
                    current: None,
                    cached: VecDeque::new(),
                    failed: false,
                    failed_notified: false,
                }));
                for frame in [0, 18, 40] {
                    let world = crate::features::resolve_preview(&doc, frame, false, frame);
                    let camera = crate::depth::camera(&doc, frame);
                    let expected = crate::gpu_scene::render(
                        &doc,
                        frame,
                        w,
                        h,
                        &ctx,
                        &mut Default::default(),
                        &mut Default::default(),
                        &world,
                        &[],
                        camera,
                        true,
                        true,
                        true,
                        false,
                        None,
                    )
                    .unwrap()
                    .chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect::<Vec<_>>();
                    state.lock().unwrap().next = Some(Arc::new(Prepared {
                        request: Request {
                            key: vec![frame],
                            doc: doc.clone(),
                            frame,
                            clock: frame,
                            size: [w, h],
                            video: Vec::new(),
                            camera: true,
                            post: true,
                            blur: true,
                            full: true,
                            budget: 128 * 1024 * 1024,
                        },
                        world,
                        geometry: Default::default(),
                    }));
                    let draw = Draw {
                        state: state.clone(),
                        corners,
                        rect,
                        ctx: ctx.clone(),
                        budget: 128 * 1024 * 1024,
                    };
                    let mut resources = egui_wgpu::CallbackResources::default();
                    let mut encoder = device.create_command_encoder(&Default::default());
                    let readbacks = crate::gpu_image::READBACKS.load(Ordering::Relaxed);
                    draw.prepare(
                        &device,
                        &queue,
                        &egui_wgpu::ScreenDescriptor {
                            size_in_pixels: [w, h],
                            pixels_per_point: 1.,
                        },
                        &mut encoder,
                        &mut resources,
                    );
                    assert_eq!(
                        crate::gpu_image::READBACKS.load(Ordering::Relaxed),
                        readbacks,
                        "resident composition must not map its output"
                    );
                    assert!(
                        !state.lock().unwrap().failed,
                        "presentation pipeline or scene graph failed"
                    );
                    let target = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Headless viewport target"),
                        size: wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    });
                    let view = target.create_view(&Default::default());
                    {
                        let mut pass = encoder
                            .begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Headless native viewport"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    depth_slice: None,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                ..Default::default()
                            })
                            .forget_lifetime();
                        draw.paint(
                            egui::PaintCallbackInfo {
                                viewport: rect,
                                clip_rect: rect,
                                pixels_per_point: 1.,
                                screen_size_px: [w, h],
                            },
                            &mut pass,
                            &resources,
                        );
                    }
                    let stride = (w * 4).div_ceil(256) * 256;
                    let read = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Test-only viewport capture"),
                        size: u64::from(stride) * u64::from(h),
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    });
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &target,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &read,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(stride),
                                rows_per_image: Some(h),
                            },
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                    let submission = queue.submit([encoder.finish()]);
                    let (tx, rx) = mpsc::channel();
                    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                        let _ = tx.send(r);
                    });
                    device
                        .poll(wgpu::PollType::Wait {
                            submission_index: Some(submission),
                            timeout: Some(std::time::Duration::from_secs(10)),
                        })
                        .unwrap();
                    rx.recv_timeout(std::time::Duration::from_secs(10))
                        .unwrap()
                        .unwrap();
                    let mapped = read.slice(..).get_mapped_range().unwrap();
                    let actual: Vec<_> = mapped
                        .chunks_exact(stride as usize)
                        .flat_map(|row| row[..w as usize * 4].iter().copied())
                        .collect();
                    let max = actual
                        .iter()
                        .zip(&expected)
                        .map(|(a, b)| a.abs_diff(*b))
                        .max()
                        .unwrap();
                    eprintln!(
                        "direct {example} frame {frame} {format:?}: max channel difference {max}"
                    );
                    assert!(max <= 2, "direct presentation changed the rendered pixels");
                    {
                        let mut state = state.lock().unwrap();
                        let image = state.current.as_ref().unwrap().image.clone();
                        state.trim_to(0);
                        assert_eq!(state.graph.resident_bytes(), 0);
                        assert_eq!(state.cached.len(), 1);
                        assert!(Arc::ptr_eq(&image, &state.current.as_ref().unwrap().image));
                        assert_eq!(state.resident_bytes(), image.bytes());
                    }
                    drop(mapped);
                    read.unmap();
                }
            }
        }
        use_on_worker(None);
    }
}
