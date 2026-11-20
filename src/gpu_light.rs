//! Full-resolution lighting compute with reusable device buffers and CPU fallback.
use crate::{gpu_lens::block_on, light::Surface};
use eframe::wgpu::{self, util::DeviceExt};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub(crate) struct LightDispatch<'a> {
    pub config: [u32; 48],
    pub surface: Option<Arc<Surface>>,
    pub mask: Option<&'a [u8]>,
    pub gobo: Option<&'a [u8]>,
}

pub(crate) struct OwnedLightDispatch {
    pub config: [u32; 48],
    pub surface: Option<Arc<Surface>>,
    pub mask: Option<Vec<u8>>,
    pub gobo: Option<Vec<u8>>,
}
impl OwnedLightDispatch {
    pub fn borrowed(&self) -> LightDispatch<'_> {
        LightDispatch {
            config: self.config,
            surface: self.surface.clone(),
            mask: self.mask.as_deref(),
            gobo: self.gobo.as_deref(),
        }
    }
}

struct Buffers {
    bytes: u64,
    source: wgpu::Buffer,
    mask: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
}
struct CachedSurface {
    owner: Arc<Surface>,
    buffer: wgpu::Buffer,
}
#[derive(Default)]
struct State {
    buffers: Option<Buffers>,
    normals: VecDeque<CachedSurface>,
}

pub(crate) struct GpuLight {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    flat: wgpu::Buffer,
    state: Mutex<State>,
    failed: AtomicBool,
}

impl GpuLight {
    pub(crate) fn native(
        &self,
        source: &std::sync::Arc<crate::gpu_image::Image>,
        input: &LightDispatch<'_>,
        surface: Option<&crate::gpu_image::Image>,
        mask: Option<&crate::gpu_image::Image>,
    ) -> Option<std::sync::Arc<crate::gpu_image::Image>> {
        if source.width != input.config[0]
            || source.height != input.config[1]
            || source.bytes_per_pixel != 4
        {
            return None;
        }
        let output = crate::gpu_image::Image::new(&self.device, source.width, source.height, 4);
        let mut data: Vec<_> = input.config.iter().flat_map(|v| v.to_le_bytes()).collect();
        if let Some(gobo) = input.gobo {
            data.extend(gobo.iter().flat_map(|v| u32::from(*v).to_le_bytes()));
        }
        let params = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Frame graph light"),
                contents: &data,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let entries: Vec<_> = [
            &source.buffer,
            &params,
            surface.map_or(&self.flat, |image| &image.buffer),
            mask.map_or(&self.flat, |image| &image.buffer),
            &output.buffer,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: b.as_entire_binding(),
        })
        .collect();
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frame graph light"),
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
    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue) -> Option<Self> {
        if device.limits().max_storage_buffers_per_shader_stage < 5 {
            return None;
        }
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LibreToon lighting"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_light.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LibreToon lighting"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if block_on(errors.pop()).is_some() {
            return None;
        }
        let flat = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Flat lighting surface"),
            contents: &[0; 8],
            usage: wgpu::BufferUsages::STORAGE,
        });
        Some(Self {
            device,
            queue,
            pipeline,
            flat,
            state: Mutex::new(State::default()),
            failed: AtomicBool::new(false),
        })
    }

    pub(crate) fn shade(&self, pixels: &mut [u8], input: &LightDispatch<'_>) -> bool {
        self.shade_many(pixels, std::slice::from_ref(input))
    }

    pub(crate) fn shade_many(&self, pixels: &mut [u8], inputs: &[LightDispatch<'_>]) -> bool {
        let _span = crate::render_profile::span("GPU lights");
        let Some(input) = inputs.first() else {
            return true;
        };
        if self.failed.load(Ordering::Relaxed) {
            return false;
        }
        let [w, h] = [input.config[0], input.config[1]];
        let bytes = u64::from(w) * u64::from(h) * 4;
        let limits = self.device.limits();
        let surface_bytes = input
            .surface
            .as_ref()
            .map_or(8, |s| s.slopes.len() as u64 * 8);
        if inputs.iter().any(|other| {
            other.config[..2] != input.config[..2]
                || other
                    .surface
                    .as_ref()
                    .is_some_and(|s| s.slopes.len() * 8 > 32 * 1024 * 1024)
                || other.mask.is_some_and(|mask| mask.len() != pixels.len())
        }) || w == 0
            || h == 0
            || bytes as usize != pixels.len()
            || bytes > 64 * 1024 * 1024
            || bytes.max(surface_bytes) > limits.max_storage_buffer_binding_size
            || bytes.max(surface_bytes) > limits.max_buffer_size
            || surface_bytes > 32 * 1024 * 1024
            || w.div_ceil(16).max(h.div_ceil(16)) > limits.max_compute_workgroups_per_dimension
            || input.mask.is_some_and(|mask| mask.len() != pixels.len())
        {
            return false;
        }
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let exhausted = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let invalid = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let rendered = self.dispatch(pixels, inputs, &mut state);
        let invalid = block_on(invalid.pop()).is_some();
        let exhausted = block_on(exhausted.pop()).is_some();
        match rendered.filter(|_| !invalid && !exhausted) {
            Some(rendered) => {
                pixels.copy_from_slice(&rendered);
                true
            }
            None => {
                self.failed.store(true, Ordering::Relaxed);
                state.buffers = None;
                state.normals.clear();
                false
            }
        }
    }

    fn dispatch(
        &self,
        pixels: &[u8],
        inputs: &[LightDispatch<'_>],
        state: &mut State,
    ) -> Option<Vec<u8>> {
        let device = &self.device;
        let bytes = pixels.len() as u64;
        if state
            .buffers
            .as_ref()
            .is_none_or(|buffers| buffers.bytes != bytes)
        {
            let create = |label, usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: bytes,
                    usage,
                    mapped_at_creation: false,
                })
            };
            state.buffers = Some(Buffers {
                bytes,
                source: create(
                    "Light artwork",
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                ),
                mask: create(
                    "Light mask",
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
                output: create(
                    "Lit artwork",
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                ),
                readback: create(
                    "Lighting readback",
                    wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                ),
            });
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        self.queue
            .write_buffer(&state.buffers.as_ref()?.source, 0, pixels);
        for (index, input) in inputs.iter().enumerate() {
            if let Some(surface) = &input.surface {
                if let Some(index) = state
                    .normals
                    .iter()
                    .position(|entry| Arc::ptr_eq(&entry.owner, surface))
                {
                    let entry = state.normals.remove(index)?;
                    state.normals.push_back(entry);
                } else {
                    const NORMAL_BUDGET: usize = 32 * 1024 * 1024;
                    let size = surface.slopes.len() * 8;
                    if size > NORMAL_BUDGET {
                        return None;
                    }
                    let mut total = state
                        .normals
                        .iter()
                        .map(|entry| entry.owner.slopes.len() * 8)
                        .sum::<usize>();
                    while total + size > NORMAL_BUDGET || state.normals.len() >= 32 {
                        total -= state.normals.pop_front()?.owner.slopes.len() * 8;
                    }
                    let normals: Vec<u8> = surface
                        .slopes
                        .iter()
                        .flat_map(|n| n.x.to_le_bytes().into_iter().chain(n.y.to_le_bytes()))
                        .collect();
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Cached lighting normals"),
                        contents: &normals,
                        usage: wgpu::BufferUsages::STORAGE,
                    });
                    state.normals.push_back(CachedSurface {
                        owner: surface.clone(),
                        buffer,
                    });
                }
            }
            let buffers = state.buffers.as_ref()?;
            let mut params: Vec<u8> = input.config.iter().flat_map(|v| v.to_le_bytes()).collect();
            if let Some(gobo) = input.gobo {
                params.extend(gobo.iter().flat_map(|v| u32::from(*v).to_le_bytes()));
            }
            // Each command owns immutable parameters and mask storage. Queue writes
            // to a shared parameter buffer would make all dispatches see the last light.
            let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Resident light parameters"),
                contents: &params,
                usage: wgpu::BufferUsages::STORAGE,
            });
            let mask = input.mask.map(|mask| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Resident light mask"),
                    contents: mask,
                    usage: wgpu::BufferUsages::STORAGE,
                })
            });
            let normals = input
                .surface
                .as_ref()
                .and_then(|_| state.normals.back())
                .map_or(&self.flat, |entry| &entry.buffer);
            let entries: Vec<_> = [
                if index % 2 == 0 {
                    &buffers.source
                } else {
                    &buffers.output
                },
                &params,
                normals,
                mask.as_ref().unwrap_or(&buffers.mask),
                if index % 2 == 0 {
                    &buffers.output
                } else {
                    &buffers.source
                },
            ]
            .into_iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();
            let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Lighting bindings"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &entries,
            });
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bindings, &[]);
                pass.dispatch_workgroups(
                    input.config[0].div_ceil(16),
                    input.config[1].div_ceil(16),
                    1,
                );
            }
        }
        let buffers = state.buffers.as_ref()?;
        let output = if inputs.len() % 2 == 1 {
            &buffers.output
        } else {
            &buffers.source
        };
        encoder.copy_buffer_to_buffer(output, 0, &buffers.readback, 0, bytes);
        let submitted = self.queue.submit([encoder.finish()]);
        let (send, result) = std::sync::mpsc::channel();
        let slice = buffers.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = send.send(result);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(Duration::from_secs(2)),
            })
            .ok()?;
        result.recv_timeout(Duration::from_secs(2)).ok()?.ok()?;
        let mapped = slice.get_mapped_range().ok()?;
        let output = mapped.to_vec();
        drop(mapped);
        buffers.readback.unmap();
        Some(output)
    }
}

#[cfg(test)]
#[test]
#[ignore = "manual end-to-end GPU lighting benchmark with optional production project"]
fn benchmark_gpu_project_lighting() {
    use crate::gpu_lens::{GpuLens, use_on_worker};
    let instance = wgpu::Instance::default();
    let adapter =
        block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let gpu = Arc::new(GpuLens::new(device, queue));
    let doc = std::env::var_os("LIBRETOON_BENCHMARK_PROJECT")
        .map(|path| crate::documents::read_project(std::path::Path::new(&path)).unwrap())
        .unwrap_or_else(|| {
            crate::examples::project(":builtin-orbit-lights")
                .unwrap()
                .unwrap()
        });
    let count = crate::animation::movie_count(&doc);
    let ctx = eframe::egui::Context::default();
    let mut cpu_cache = crate::paint::RenderCache::default();
    let mut gpu_cache = crate::paint::RenderCache::default();
    let mut cpu = Duration::ZERO;
    let mut accelerated = Duration::ZERO;
    let mut difference = 0;
    for frame in [0, count / 2, count.saturating_sub(1)] {
        use_on_worker(None);
        let start = std::time::Instant::now();
        let expected =
            crate::exporter::raster(&doc, frame, doc.width, doc.height, &ctx, &mut cpu_cache);
        cpu += start.elapsed();
        use_on_worker(Some(gpu.clone()));
        let start = std::time::Instant::now();
        let rendered =
            crate::exporter::raster(&doc, frame, doc.width, doc.height, &ctx, &mut gpu_cache);
        accelerated += start.elapsed();
        let largest = expected
            .iter()
            .zip(&rendered)
            .map(|(&a, &b)| a.abs_diff(b))
            .max()
            .unwrap();
        difference = difference.max(largest);
        let squared = expected
            .iter()
            .zip(&rendered)
            .map(|(&a, &b)| f64::from(a.abs_diff(b)).powi(2))
            .sum::<f64>();
        let rms = (squared / expected.len() as f64).sqrt();
        let tail = expected
            .iter()
            .zip(&rendered)
            .filter(|(a, b)| a.abs_diff(**b) > 3)
            .count();
        eprintln!(
            "frame {frame}: max channel difference {largest}/255, RMS {rms:.5}/255, {tail} channels differ by more than 3"
        );
        if let Some(directory) = std::env::var_os("LIBRETOON_RENDER_COMPARE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            image::RgbImage::from_raw(doc.width, doc.height, expected.clone())
                .unwrap()
                .save(directory.join(format!("cpu-{frame}.png")))
                .unwrap();
            image::RgbImage::from_raw(doc.width, doc.height, rendered.clone())
                .unwrap()
                .save(directory.join(format!("gpu-{frame}.png")))
                .unwrap();
        }
        // Hardware triangle setup quantizes subpixel positions. Bright lights
        // can amplify that coverage difference on an otherwise identical edge.
        // Check those rare outliers bidirectionally against a one-pixel
        // neighbourhood, while still bounding RMS and the outlier population.
        let width = doc.width as usize;
        let height = doc.height as usize;
        let nearby_error = |a: &[u8], b: &[u8]| {
            a.chunks_exact(3)
                .enumerate()
                .filter_map(|(i, pixel)| {
                    if pixel
                        .iter()
                        .zip(&b[i * 3..i * 3 + 3])
                        .all(|(x, y)| x.abs_diff(*y) <= 8)
                    {
                        return None;
                    }
                    let x = i % width;
                    let y = i / width;
                    // A coverage value may lie between the two neighbouring
                    // colours rather than equal either sample exactly.
                    let mut low = [255u8; 3];
                    let mut high = [0u8; 3];
                    for py in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                        for px in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                            for c in 0..3 {
                                let v = b[(py * width + px) * 3 + c];
                                low[c] = low[c].min(v);
                                high[c] = high[c].max(v);
                            }
                        }
                    }
                    Some(
                        (0..3)
                            .map(|c| pixel[c].abs_diff(pixel[c].clamp(low[c], high[c])))
                            .max()
                            .unwrap(),
                    )
                })
                .max()
                .unwrap_or(0)
        };
        let edge_error = nearby_error(&expected, &rendered).max(nearby_error(&rendered, &expected));
        eprintln!("frame {frame}: bidirectional edge error {edge_error}/255");
        assert!(
            edge_error <= 8 && rms <= 0.25 && tail * 10_000 <= expected.len(),
            "frame {frame}: maximum channel difference {largest}, edge error {edge_error}, RMS {rms}"
        );
    }
    use_on_worker(None);
    assert!(gpu.light_pass_count() > 0);
    eprintln!(
        "{}: {}x{}, three complete frames, {} GPU light passes. CPU {cpu:?}, GPU {accelerated:?}, max RGB difference {difference}/255",
        adapter.get_info().name,
        doc.width,
        doc.height,
        gpu.light_pass_count()
    );
}

#[cfg(test)]
#[test]
#[ignore = "manual sequential camera preview benchmark with production project"]
fn benchmark_camera_lighting_reuse() {
    use crate::gpu_lens::{GpuLens, use_on_worker};
    let instance = wgpu::Instance::default();
    let adapter =
        block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let gpu = Arc::new(GpuLens::new(device, queue));
    let doc = std::env::var_os("LIBRETOON_BENCHMARK_PROJECT")
        .map(|path| crate::documents::read_project(std::path::Path::new(&path)).unwrap())
        .unwrap_or_else(|| {
            crate::examples::project(":builtin-orbit-lights")
                .unwrap()
                .unwrap()
        });
    let ctx = eframe::egui::Context::default();
    let mut cache = crate::paint::RenderCache::default();
    use_on_worker(Some(gpu.clone()));
    for frame in 0..6.min(crate::animation::movie_count(&doc)) {
        let before = gpu.light_pass_count();
        let start = std::time::Instant::now();
        let pixels = crate::exporter::raster(&doc, frame, doc.width, doc.height, &ctx, &mut cache);
        assert_eq!(pixels.len(), doc.width as usize * doc.height as usize * 3);
        eprintln!(
            "camera frame {frame}: {:?}, {} new GPU light passes",
            start.elapsed(),
            gpu.light_pass_count() - before
        );
    }
    use_on_worker(None);
}

#[cfg(test)]
#[test]
#[ignore = "manual release throughput benchmark: uncached sequential compositing, not frame-cache playback"]
fn benchmark_gpu_scene_throughput() {
    use crate::gpu_lens::{GpuLens, use_on_worker};
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
    let gpu = Arc::new(GpuLens::new(device, queue));
    let doc = std::env::var_os("LIBRETOON_BENCHMARK_PROJECT")
        .map(|p| crate::documents::read_project(std::path::Path::new(&p)).unwrap())
        .unwrap_or_else(|| {
            crate::examples::project(":builtin-orbit-lights")
                .unwrap()
                .unwrap()
        });
    let count = crate::animation::movie_count(&doc);
    let ctx = eframe::egui::Context::default();
    use_on_worker(Some(gpu.clone()));
    for factor in [0.25f32, 0.5, 1.] {
        let (w, h) = (
            (doc.width as f32 * factor).round() as u32,
            (doc.height as f32 * factor).round() as u32,
        );
        let mut cache = crate::paint::RenderCache::default();
        // Compile and allocate before measuring. This is a render benchmark;
        // no final frame cache is used, every measured frame is composed.
        for frame in 0..2.min(count) {
            crate::exporter::raster(&doc, frame, w, h, &ctx, &mut cache);
        }
        let mut times = Vec::new();
        for first in [0, count / 2, count.saturating_sub(6)] {
            for frame in first..(first + 6).min(count) {
                let start = std::time::Instant::now();
                let pixels = crate::exporter::raster(&doc, frame, w, h, &ctx, &mut cache);
                assert_eq!(pixels.len(), w as usize * h as usize * 3);
                times.push(start.elapsed().as_secs_f64());
            }
        }
        let total = times.iter().sum::<f64>();
        let mean = total / times.len() as f64;
        times.sort_by(f64::total_cmp);
        let p95 = times[(times.len() * 95 / 100).min(times.len() - 1)];
        eprintln!(
            "{}: {w}x{h}, {} uncached frames, mean {:.2} ms ({:.1} composed FPS), p95 {:.2} ms; {} resident GPU frames; fallback {}",
            adapter.get_info().name,
            times.len(),
            mean * 1000.,
            1. / mean,
            p95 * 1000.,
            cache.native_scene.frames,
            cache.native_scene.reason
        );
    }
    use_on_worker(None);
}
