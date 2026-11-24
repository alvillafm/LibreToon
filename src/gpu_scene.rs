//! Device-resident scene graph. Geometry is prepared only when its source
//! drawing changes; camera and light edits update parameters. One readback at
//! the completed-frame boundary serves the existing playback/export cache.
use crate::{Document, Mark, canvas::View, exporter::RenderSubset, gpu_image::Image};
use eframe::egui::{self, Pos2, Rect, Vec2};
use geo::BoundingRect;
use std::{collections::VecDeque, sync::Arc};

fn bounds(mark: &Mark) -> Rect {
    if mark.style.text.is_some()
        && let Some(b) = crate::vector::text_ink(mark).bounding_rect()
    {
        return Rect::from_min_max(
            Pos2::new(b.min().x as f32, b.min().y as f32),
            Pos2::new(b.max().x as f32, b.max().y as f32),
        );
    }
    crate::vector::control_bounds(mark)
}

#[derive(PartialEq)]
struct Key {
    layer: usize,
    piece: Option<u64>,
    subset: RenderSubset,
    size: [u32; 2],
    view: [u32; 6],
    assets: usize,
    marks: Vec<Mark>,
}
struct Source {
    key: Key,
    image: Arc<Image>,
    _assets: crate::shared::SharedVec<crate::media::Asset>,
}
struct Normal {
    source: Arc<Image>,
    bevel: u32,
    step: usize,
    height: Option<Arc<Image>>,
    texture: u32,
    image: Arc<Image>,
}
#[derive(Default)]
pub(crate) struct Cache {
    sources: VecDeque<Source>,
    normals: VecDeque<Normal>,
    pub frames: usize,
    pub reason: &'static str,
}
impl Cache {
    /// Release derived normals first, then old raster sources. Referenced images
    /// remain alive in the in-flight frame; eviction never changes its pixels.
    pub(crate) fn trim_to(&mut self, budget: usize) {
        while self.resident_bytes() > budget {
            if self.normals.pop_front().is_none() && self.sources.pop_front().is_none() {
                break;
            }
        }
    }
    pub(crate) fn resident_bytes(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        self.resident_bytes_with(&mut seen)
    }
    pub(crate) fn resident_bytes_with(&self, seen: &mut std::collections::HashSet<usize>) -> usize {
        let mut count = |image: &Arc<Image>| {
            if seen.insert(Arc::as_ptr(image) as usize) {
                image.bytes()
            } else {
                0
            }
        };
        let mut bytes = 0;
        for source in &self.sources {
            bytes += count(&source.image);
        }
        for normal in &self.normals {
            bytes += count(&normal.source) + count(&normal.image);
            if let Some(height) = &normal.height {
                bytes += count(height);
            }
        }
        bytes
    }
    fn source(&mut self, key: &Key) -> Option<Arc<Image>> {
        let i = self.sources.iter().position(|s| s.key == *key)?;
        let source = self.sources.remove(i)?;
        let image = source.image.clone();
        self.sources.push_back(source);
        Some(image)
    }
    fn insert(&mut self, source: Source) {
        let bytes = source.image.bytes();
        if bytes > 96 * 1024 * 1024 {
            return;
        }
        let mut total = bytes + self.sources.iter().map(|s| s.image.bytes()).sum::<usize>();
        while total > 96 * 1024 * 1024 || self.sources.len() >= 32 {
            total -= self.sources.pop_front().unwrap().image.bytes();
        }
        self.sources.push_back(source);
    }
    fn normals(
        &mut self,
        gpu: &crate::gpu_lens::GpuLens,
        source: &Arc<Image>,
        bevel: f32,
        texture: f32,
        height: Option<&Arc<Image>>,
        step: usize,
    ) -> Option<Arc<Image>> {
        if let Some(i) = self.normals.iter().position(|n| {
            Arc::ptr_eq(&n.source, source)
                && n.bevel == bevel.to_bits()
                && n.step == step
                && n.texture == texture.to_bits()
                && match (n.height.as_ref(), height) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    _ => false,
                }
        }) {
            let n = self.normals.remove(i)?;
            let image = n.image.clone();
            self.normals.push_back(n);
            return Some(image);
        }
        let image = gpu
            .surfaces
            .get_or_init(|| {
                crate::gpu_surface::GpuSurface::new(gpu.device.clone(), gpu.queue.clone())
            })
            .as_ref()?
            .native(source, bevel, texture, height.map(|h| h.as_ref()), step)?;
        {
            let bytes = image.bytes() + source.bytes() + height.map_or(0, |h| h.bytes());
            if bytes <= 64 * 1024 * 1024 {
                let mut total = bytes
                    + self
                        .normals
                        .iter()
                        .map(|n| {
                            n.image.bytes()
                                + n.source.bytes()
                                + n.height.as_ref().map_or(0, |h| h.bytes())
                        })
                        .sum::<usize>();
                while total > 64 * 1024 * 1024 || self.normals.len() >= 32 {
                    let n = self.normals.pop_front().unwrap();
                    total -= n.image.bytes()
                        + n.source.bytes()
                        + n.height.as_ref().map_or(0, |h| h.bytes());
                }
                self.normals.push_back(Normal {
                    source: source.clone(),
                    bevel: bevel.to_bits(),
                    step,
                    height: height.cloned(),
                    texture: texture.to_bits(),
                    image: image.clone(),
                });
            }
        }
        Some(image)
    }
}

fn composite(
    gpu: &crate::gpu_lens::GpuLens,
    bottom: &Arc<Image>,
    top: &Arc<Image>,
) -> Option<Arc<Image>> {
    let mut config = vec![0u32; 16];
    config[..6].copy_from_slice(&[bottom.width, bottom.height, 12, 1, top.width, 0]);
    gpu.native_post(bottom, &config, Some(top))
}

fn auxiliary(
    doc: &Document,
    ctx: &egui::Context,
    render_cache: &mut crate::paint::RenderCache,
    cache: &mut Cache,
    world: &[(usize, usize, Mark)],
    gpu: &crate::gpu_lens::GpuLens,
    subset: RenderSubset,
    size: [u32; 2],
    view: View,
    layer: usize,
) -> Option<Arc<Image>> {
    let marks = world
        .iter()
        .filter(|(l, root, m)| match subset {
            RenderSubset::Height(index) => *l == index && m.style.surface_height.is_some(),
            RenderSubset::LightMask(index, target) => {
                *l == index && *root == target && m.style.light_mask
            }
            RenderSubset::SceneLightMask(id) => m.style.light_mask_target == Some(id),
            RenderSubset::Single(false, index) => {
                *l == index && m.style.surface_height.is_none() && !m.style.light_mask
            }
            _ => false,
        })
        .map(|(_, _, m)| m.clone())
        .collect();
    let key = Key {
        layer,
        piece: None,
        subset,
        size,
        view: [
            view.center.x.to_bits(),
            view.center.y.to_bits(),
            view.stage_center.x.to_bits(),
            view.stage_center.y.to_bits(),
            view.scale.to_bits(),
            view.angle.to_bits(),
        ],
        assets: doc.extras.media.assets.identity(),
        marks,
    };
    if let Some(image) = cache.source(&key) {
        return Some(image);
    }
    let (_, meshes) = crate::exporter::art_meshes(
        doc,
        size[0],
        size[1],
        ctx,
        render_cache,
        &[],
        Some(subset),
        world,
        0,
        true,
        Some((size, view)),
    );
    let raster = gpu
        .raster
        .get_or_init(|| crate::gpu_raster::GpuRaster::new(gpu.device.clone(), gpu.queue.clone()))
        .as_ref()?;
    let mut image = raster.native(size[0], size[1], &meshes, |id| {
        if let egui::TextureId::User(id) = id {
            doc.extras.media.pixels(id)
        } else {
            None
        }
    })?;
    if image.width != size[0] {
        image = gpu
            .planes
            .get_or_init(|| crate::gpu_plane::GpuPlane::new(gpu.device.clone(), gpu.queue.clone()))
            .as_ref()?
            .native(&image, size, Pos2::ZERO, Vec2::X, Vec2::Y)?;
    }
    cache.insert(Source {
        key,
        image: image.clone(),
        _assets: doc.extras.media.assets.clone(),
    });
    Some(image)
}
fn adjustment(
    gpu: &crate::gpu_lens::GpuLens,
    image: Arc<Image>,
    layer: &crate::document::Layer,
    frame: usize,
    scale: f32,
    outline: Option<bool>,
) -> Option<Arc<Image>> {
    let opacity = layer.options.opacity.clamp(0., 1.);
    if opacity == 0. {
        return Some(image);
    }
    let Some(mut effects) = crate::post::effects_at(layer, frame) else {
        return Some(image);
    };
    if let Some(only) = outline {
        effects
            .to_mut()
            .retain(|effect| effect.is_outline() == only);
    }
    crate::post::animate_grain(effects.to_mut(), frame);
    let changed = crate::post::native_apply(gpu, image.clone(), &effects, scale)?;
    if opacity == 1. {
        Some(changed)
    } else {
        gpu.native_filter(&changed, 11, Some(&image), &[opacity], [0; 3])
    }
}

fn video_image(
    gpu: &crate::gpu_lens::GpuLens,
    doc: &Document,
    w: u32,
    h: u32,
    ctx: &egui::Context,
    render_cache: &mut crate::paint::RenderCache,
    world: &[(usize, usize, Mark)],
    video: &[crate::media::ExportVideoFrame],
    clip_index: usize,
    optics: Option<&crate::depth::Optics>,
) -> Option<Arc<Image>> {
    let clip = &video[clip_index];
    let scale = (w as f32 / doc.width as f32).min(h as f32 / doc.height as f32);
    let radius = optics
        .filter(|o| o.strength > 0.)
        .map_or(0., |o| o.radius(clip.depth.distance - o.position) * scale)
        .min(128.);
    let pad = if radius >= 0.1 {
        (radius * 2. + 3.).ceil() as u32
    } else {
        0
    };
    let size = [w + pad * 2, h + pad * 2];
    let (_, meshes) = crate::exporter::art_meshes(
        doc,
        w,
        h,
        ctx,
        render_cache,
        video,
        Some(RenderSubset::Single(true, clip_index)),
        world,
        pad,
        true,
        None,
    );
    let raster = gpu
        .raster
        .get_or_init(|| crate::gpu_raster::GpuRaster::new(gpu.device.clone(), gpu.queue.clone()))
        .as_ref()?;
    let projector = gpu
        .planes
        .get_or_init(|| crate::gpu_plane::GpuPlane::new(gpu.device.clone(), gpu.queue.clone()))
        .as_ref()?;
    let mut image = raster.native(size[0], size[1], &meshes, |id| {
        if let egui::TextureId::User(id) = id {
            video
                .get((u64::MAX - id) as usize)
                .map(|clip| {
                    (
                        [clip.pixels.width() as usize, clip.pixels.height() as usize],
                        clip.pixels.as_raw().as_slice(),
                    )
                })
                .or_else(|| doc.extras.media.pixels(id))
        } else {
            None
        }
    })?;
    if image.width != size[0] {
        image = projector.native(&image, size, Pos2::ZERO, Vec2::X, Vec2::Y)?;
    }
    if radius >= 0.1 {
        image = gpu.native_blur(&image, radius, optics?)?;
    }
    if pad > 0 {
        image = projector.native(
            &image,
            [w, h],
            Pos2::new(pad as f32, pad as f32),
            Vec2::X,
            Vec2::Y,
        )?;
    }
    Some(image)
}

pub(crate) fn render(
    doc: &Document,
    frame: usize,
    w: u32,
    h: u32,
    ctx: &egui::Context,
    render_cache: &mut crate::paint::RenderCache,
    cache: &mut Cache,
    world: &[(usize, usize, Mark)],
    video: &[crate::media::ExportVideoFrame],
    camera: Option<&crate::features::Camera>,
    post: bool,
    blur: bool,
    full_quality: bool,
    transparent: bool,
    work: Option<&crate::depth::PreviewWork>,
) -> Option<Vec<u8>> {
    let gpu = crate::gpu_lens::on_worker()?;
    let image = render_resident(
        &gpu,
        doc,
        frame,
        w,
        h,
        ctx,
        render_cache,
        cache,
        world,
        video,
        camera,
        post,
        blur,
        full_quality,
        transparent,
        work,
    )?;
    let rgba = image.read(&gpu.device, &gpu.queue)?;
    if transparent {
        Some(rgba)
    } else {
        Some(
            rgba.chunks_exact(4)
                .flat_map(|p| p[..3].iter().copied())
                .collect(),
        )
    }
}

pub(crate) fn render_resident(
    gpu: &Arc<crate::gpu_lens::GpuLens>,
    doc: &Document,
    frame: usize,
    w: u32,
    h: u32,
    ctx: &egui::Context,
    render_cache: &mut crate::paint::RenderCache,
    cache: &mut Cache,
    world: &[(usize, usize, Mark)],
    video: &[crate::media::ExportVideoFrame],
    camera: Option<&crate::features::Camera>,
    post: bool,
    blur: bool,
    full_quality: bool,
    transparent: bool,
    work: Option<&crate::depth::PreviewWork>,
) -> Option<Arc<Image>> {
    cache.reason = "device or image limits";
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) * 4 > 32 * 1024 * 1024 {
        return None;
    }
    let optics = camera.map(|c| crate::animation::camera_optics(c, frame));
    cache.reason = "frame graph allocation or capability";
    let mut groups = Vec::<(usize, usize, usize, u64)>::new();
    for (fx, layer) in doc
        .layers
        .iter()
        .enumerate()
        .filter(|(_, l)| post && l.visible && crate::folder_groups::layer_active(doc, l, frame))
    {
        let Some(crate::light::Receiver::Folder(id)) = layer.options.fx_target else {
            continue;
        };
        let indices: Vec<_> = doc
            .layers
            .iter()
            .enumerate()
            .take(fx)
            .filter(|(_, l)| crate::light::Receiver::Folder(id).matches(doc, l))
            .map(|(i, _)| i)
            .collect();
        let (Some(&start), Some(&end)) = (indices.first(), indices.last()) else {
            continue;
        };
        if indices.len() < 2
            || indices.len() != end - start + 1
            || doc.layers[start..=end]
                .iter()
                .any(|l| l.options.adjustment.is_some() || l.options.light.is_some())
            || groups.iter().any(|(a, b, _, _)| start <= *b && *a <= end)
            || video
                .iter()
                .any(|clip| (start + 1..=end).contains(&clip.slot))
        {
            continue;
        }
        let last = world
            .iter()
            .filter(|(i, _, m)| {
                (start..=end).contains(i)
                    && !m.style.light_mask
                    && m.style.surface_height.is_none()
                    && (m.style.text.is_some() || bounds(m).expand(m.width * 0.5).is_positive())
            })
            .map(|(i, _, _)| *i)
            .max();
        if let Some(last) = last {
            groups.push((start, end, last, id));
        }
    }
    let auto_outlines: Vec<_> = doc
        .layers
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            if !post
                || !l.visible
                || l.options.fx_target.is_some()
                || !crate::folder_groups::layer_active(doc, l, frame)
                || !crate::post::effects_at(l, frame)
                    .is_some_and(|e| e.iter().any(crate::post::Effect::enabled_outline))
            {
                return None;
            }
            let target = world
                .iter()
                .filter(|(layer, _, m)| {
                    *layer < i
                        && !crate::post::is_adjustment(&doc.layers[*layer])
                        && m.style.surface_height.is_none()
                        && !m.style.light_mask
                })
                .map(|(layer, _, _)| *layer)
                .max()?;
            Some((i, target))
        })
        .collect();
    crate::gpu_image::checked_scope(&gpu.device, || {
        let _profile = crate::render_profile::span("resident scene graph");
        let raster = gpu
            .raster
            .get_or_init(|| {
                crate::gpu_raster::GpuRaster::new(gpu.device.clone(), gpu.queue.clone())
            })
            .as_ref()?;
        let projector = gpu
            .planes
            .get_or_init(|| crate::gpu_plane::GpuPlane::new(gpu.device.clone(), gpu.queue.clone()))
            .as_ref()?;
        let lights = gpu
            .lights
            .get_or_init(|| crate::gpu_light::GpuLight::new(gpu.device.clone(), gpu.queue.clone()))
            .as_ref()?;
        let scale = (w as f32 / doc.width as f32).min(h as f32 / doc.height as f32);
        let mut result: Option<Arc<Image>> = None;
        let mut group_image: Option<Arc<Image>> = None;
        let mut clips = video.iter().enumerate().peekable();
        for index in 0..=doc.layers.len() {
            if work.is_some_and(crate::depth::PreviewWork::canceled) {
                return None;
            }
            while clips.peek().is_some_and(|(_, clip)| clip.slot <= index) {
                let (clip_index, _) = clips.next()?;
                let image = video_image(
                    &gpu,
                    doc,
                    w,
                    h,
                    ctx,
                    render_cache,
                    world,
                    video,
                    clip_index,
                    optics.as_ref().filter(|_| blur),
                )?;
                result = Some(if let Some(previous) = result.take() {
                    composite(&gpu, &previous, &image)?
                } else {
                    image
                });
            }
            let Some(layer) = doc.layers.get(index) else {
                break;
            };
            if !layer.visible || !crate::folder_groups::layer_active(doc, layer, frame) {
                continue;
            }
            if post
                && layer.options.fx_target.is_none()
                && crate::post::visible_effects_at(layer, frame)
                && let Some(image) = result.take()
            {
                result = Some(adjustment(
                    &gpu,
                    image,
                    layer,
                    frame,
                    scale,
                    auto_outlines
                        .iter()
                        .any(|(fx, _)| *fx == index)
                        .then_some(false),
                )?);
            }
            if crate::post::is_adjustment(layer) {
                continue;
            }
            let marks: Vec<_> = world
                .iter()
                .filter(|(l, _, m)| {
                    *l == index && m.style.surface_height.is_none() && !m.style.light_mask
                })
                .collect();
            if marks.is_empty() {
                continue;
            }
            let group = groups
                .iter()
                .find(|(start, end, _, _)| (*start..=*end).contains(&index));
            let receiving: Vec<_> = if post {
                doc.layers
                    .iter()
                    .enumerate()
                    .skip(index + 1)
                    .filter_map(|(i, l)| {
                        l.options
                            .light
                            .as_ref()
                            .filter(|light| {
                                l.visible
                                    && crate::folder_groups::layer_active(doc, l, frame)
                                    && light.receives(doc, layer)
                            })
                            .map(|light| (i, light, l.options.opacity))
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let radius = if blur {
                optics
                    .as_ref()
                    .map_or(0., |o| {
                        o.radius(layer.options.depth.distance - o.position) * scale
                    })
                    .min(128.)
            } else {
                0.
            };
            let pad = if radius >= 0.1 {
                (radius * 2. + 3.).ceil() as u32
            } else {
                0
            };
            let projected_size = [w + pad * 2, h + pad * 2];
            let output = crate::exporter::lit_output_view(
                doc,
                frame,
                camera,
                &layer.options.depth,
                w,
                h,
                pad,
            );
            let density = if full_quality {
                (scale * (output.scale / scale).ceil()).max(output.scale)
            } else {
                scale
            };
            let layer_bounds = world.iter().filter(|(layer, _, _)| *layer == index).fold(
                Rect::NOTHING,
                |bounds, (_, _, m)| {
                    bounds.union(crate::vector::control_bounds(m).expand(m.width * 0.5))
                },
            );
            let extent = (-layer_bounds.min.x)
                .max(-layer_bounds.min.y)
                .max(layer_bounds.max.x - doc.width as f32)
                .max(layer_bounds.max.y - doc.height as f32)
                .max(0.);
            let source_pad = (((extent * scale + 32.) / 64.).ceil().clamp(1., 8.) as u32) * 64;
            let step = if full_quality && output.scale > scale {
                projected_size[0].max(projected_size[1]).div_ceil(1024)
            } else {
                (w + source_pad * 2).max(h + source_pad * 2).div_ceil(1024)
            }
            .max(1) as usize;
            let mut pieces = Vec::new();
            let mask_roots: std::collections::HashSet<_> = world
                .iter()
                .filter(|(l, _, m)| *l == index && m.style.light_mask)
                .map(|(_, root, _)| *root)
                .collect();
            if !receiving.is_empty() {
                for (_, root, mark) in &marks {
                    let piece = (
                        (!mask_roots.is_empty()).then_some(*root),
                        Some(mark.style.surface_piece),
                    );
                    if pieces.last() != Some(&piece) {
                        if pieces.len() == 16 || pieces.contains(&piece) {
                            pieces.clear();
                            break;
                        }
                        pieces.push(piece);
                    }
                }
            }
            let pieces: Vec<_> = if pieces.len() > 1 {
                pieces
            } else if !mask_roots.is_empty() {
                let mut roots = Vec::new();
                for (_, root, _) in &marks {
                    let piece = (Some(*root), None);
                    if !roots.contains(&piece) {
                        roots.push(piece);
                    }
                }
                roots
            } else {
                vec![(None, None)]
            };
            let mut layer_image: Option<Arc<Image>> = None;
            for (root, piece) in pieces {
                let source_marks: Vec<_> = marks
                    .iter()
                    .filter(|(_, source, m)| {
                        root.is_none_or(|r| *source == r)
                            && piece.is_none_or(|p| m.style.surface_piece == p)
                    })
                    .map(|(_, _, m)| m.clone())
                    .collect();
                let bounds = source_marks.iter().fold(Rect::NOTHING, |bounds, m| {
                    bounds.union(crate::gpu_scene::bounds(m).expand(m.width * 0.5))
                });
                if !bounds.is_finite() || !bounds.is_positive() {
                    continue;
                }
                let padding = receiving
                    .iter()
                    .map(|(_, light, _)| light.region_padding(frame, density, step))
                    .fold(2., f32::max)
                    .min(512.)
                    + 2.;
                let tile = (step * 128) as f32;
                let drawing = Rect::from_min_max(
                    (bounds.min.to_vec2() * density).to_pos2(),
                    (bounds.max.to_vec2() * density).to_pos2(),
                )
                .expand(padding);
                let visible = Rect::from_points(
                    &[
                        Pos2::ZERO,
                        Pos2::new(projected_size[0] as f32, 0.),
                        Pos2::new(0., projected_size[1] as f32),
                        Pos2::new(projected_size[0] as f32, projected_size[1] as f32),
                    ]
                    .map(|p| output.world(p) * density),
                )
                .expand(padding);
                let footprint = drawing.intersect(visible);
                if !receiving.is_empty() && !footprint.is_positive() {
                    continue;
                }
                let min = footprint.min.to_vec2() / tile;
                let max = footprint.max.to_vec2() / tile;
                let guard = Vec2::splat((step * 64).min(128) as f32);
                let origin = min.floor() * tile - guard;
                let extent = max.ceil() * tile + guard - origin;
                let size = if receiving.is_empty() {
                    projected_size
                } else {
                    [extent.x as u32, extent.y as u32]
                };
                if size.contains(&0)
                    || u64::from(size[0]) * u64::from(size[1]) * 4 > 64 * 1024 * 1024
                {
                    return None;
                }
                let view = if receiving.is_empty() {
                    output
                } else {
                    View {
                        center: Pos2::ZERO - origin,
                        stage_center: Pos2::ZERO,
                        scale: density,
                        angle: 0.,
                    }
                };
                let subset = if piece.is_some() || root.is_some() {
                    RenderSubset::Piece(index, root, piece)
                } else {
                    RenderSubset::Single(false, index)
                };
                let key = Key {
                    layer: index,
                    piece,
                    subset,
                    size,
                    view: [
                        view.center.x.to_bits(),
                        view.center.y.to_bits(),
                        view.stage_center.x.to_bits(),
                        view.stage_center.y.to_bits(),
                        view.scale.to_bits(),
                        view.angle.to_bits(),
                    ],
                    assets: doc.extras.media.assets.identity(),
                    marks: source_marks,
                };
                let base = if let Some(image) = cache.source(&key) {
                    image
                } else {
                    let (_, meshes) = crate::exporter::art_meshes(
                        doc,
                        w,
                        h,
                        ctx,
                        render_cache,
                        &[],
                        Some(subset),
                        world,
                        0,
                        true,
                        Some((size, view)),
                    );
                    let mut image = raster.native(size[0], size[1], &meshes, |id| {
                        if let egui::TextureId::User(id) = id {
                            doc.extras.media.pixels(id)
                        } else {
                            None
                        }
                    })?;
                    if image.width != size[0] {
                        image = projector.native(&image, size, Pos2::ZERO, Vec2::X, Vec2::Y)?;
                    }
                    cache.insert(Source {
                        key,
                        image: image.clone(),
                        _assets: doc.extras.media.assets.clone(),
                    });
                    image
                };
                let mut lit = base.clone();
                let height = if world
                    .iter()
                    .any(|(l, _, m)| *l == index && m.style.surface_height.is_some())
                {
                    Some(auxiliary(
                        doc,
                        ctx,
                        render_cache,
                        cache,
                        world,
                        &gpu,
                        RenderSubset::Height(index),
                        size,
                        view,
                        index,
                    )?)
                } else {
                    None
                };
                let local_mask = if let Some(root) = root.filter(|r| mask_roots.contains(r)) {
                    Some(auxiliary(
                        doc,
                        ctx,
                        render_cache,
                        cache,
                        world,
                        &gpu,
                        RenderSubset::LightMask(index, root),
                        size,
                        view,
                        index,
                    )?)
                } else {
                    None
                };
                let scene_mask = if let Some(id) = layer.options.light_receiver.filter(|id| {
                    world
                        .iter()
                        .any(|(_, _, m)| m.style.light_mask_target == Some(*id))
                }) {
                    Some(auxiliary(
                        doc,
                        ctx,
                        render_cache,
                        cache,
                        world,
                        &gpu,
                        RenderSubset::SceneLightMask(id),
                        size,
                        view,
                        index,
                    )?)
                } else {
                    None
                };
                let mask = match (local_mask, scene_mask) {
                    (Some(a), Some(b)) => Some(composite(&gpu, &a, &b)?),
                    (Some(a), None) | (None, Some(a)) => Some(a),
                    (None, None) => None,
                };
                for (_, light, opacity) in &receiving {
                    let settings =
                        light.native_settings(frame, size, view, step, *opacity, mask.is_some());
                    if *opacity <= 0. || f32::from_bits(settings.config[23]) <= 0. {
                        continue;
                    }
                    let normal = if settings.relief {
                        Some(cache.normals(
                            &gpu,
                            if settings.texture == 0. { &base } else { &lit },
                            settings.bevel,
                            settings.texture,
                            height.as_ref(),
                            step,
                        )?)
                    } else {
                        None
                    };
                    let mask = if settings.feather > 0. && light.uses_mask() {
                        if let Some(mask) = &mask {
                            Some(gpu.native_blur(
                                mask,
                                settings.feather.min(128.),
                                &crate::depth::Optics::default(),
                            )?)
                        } else {
                            None
                        }
                    } else {
                        mask.clone()
                    };
                    lit = lights.native(
                        &lit,
                        &crate::gpu_light::LightDispatch {
                            config: settings.config,
                            surface: None,
                            mask: None,
                            gobo: settings.gobo.as_deref().map(|g| g.as_slice()),
                        },
                        normal.as_deref(),
                        mask.as_deref(),
                    )?;
                    gpu.light_passes
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let start = view.screen(output.world(Pos2::new(0.5, 0.5))) - Vec2::splat(0.5);
                let dx =
                    crate::canvas::rotate(Vec2::X * (view.scale / output.scale), -output.angle);
                let dy =
                    crate::canvas::rotate(Vec2::Y * (view.scale / output.scale), -output.angle);
                let image = if receiving.is_empty() {
                    lit
                } else {
                    projector.native(&lit, projected_size, start, dx, dy)?
                };
                layer_image = Some(if let Some(previous) = layer_image.take() {
                    composite(&gpu, &previous, &image)?
                } else {
                    image
                });
            }
            let Some(mut layer_image) = layer_image else {
                continue;
            };
            if post && let Some(floor) = layer.options.light_receiver {
                for (light_index, light_row) in doc
                    .layers
                    .iter()
                    .enumerate()
                    .skip(index + 1)
                    .filter(|(_, l)| l.visible && crate::folder_groups::layer_active(doc, l, frame))
                {
                    let Some(light) = &light_row.options.light else {
                        continue;
                    };
                    let Some(shadow) = light
                        .native_shadow(
                            projected_size[0],
                            projected_size[1],
                            pad,
                            doc,
                            frame,
                            camera.is_some(),
                            light_row.options.opacity,
                        )
                        .filter(|s| s.floor == floor)
                    else {
                        continue;
                    };
                    let mut mask: Option<Arc<Image>> = None;
                    for (caster_index, caster) in doc
                        .layers
                        .iter()
                        .enumerate()
                        .take(light_index)
                        .skip(index + 1)
                    {
                        if !caster.visible
                            || crate::post::is_adjustment(caster)
                            || !shadow.caster.map_or_else(
                                || light.receives(doc, caster),
                                |id| caster.options.light_receiver == Some(id),
                            )
                        {
                            continue;
                        }
                        let caster_view = crate::exporter::lit_output_view(
                            doc,
                            frame,
                            camera,
                            &caster.options.depth,
                            w,
                            h,
                            pad,
                        );
                        let caster_mask = auxiliary(
                            doc,
                            ctx,
                            render_cache,
                            cache,
                            world,
                            &gpu,
                            RenderSubset::Single(false, caster_index),
                            projected_size,
                            caster_view,
                            caster_index,
                        )?;
                        mask = Some(if let Some(previous) = mask.take() {
                            composite(&gpu, &previous, &caster_mask)?
                        } else {
                            caster_mask
                        });
                    }
                    if let Some(mut mask) = mask {
                        if shadow.softness > 0. {
                            mask = gpu.native_blur(
                                &mask,
                                shadow.softness,
                                &crate::depth::Optics::default(),
                            )?;
                        }
                        layer_image = gpu.native_filter(
                            &layer_image,
                            6,
                            Some(&mask),
                            &shadow.values,
                            [0; 3],
                        )?;
                    }
                }
            }
            for (_, fx) in doc
                .layers
                .iter()
                .enumerate()
                .skip(index + 1)
                .filter(|(fx, l)| {
                    post && l.visible
                        && crate::folder_groups::layer_active(doc, l, frame)
                        && l.options
                            .fx_target
                            .is_some_and(|target| target.matches(doc, layer))
                        && !group.is_some_and(|(_, end, _, id)| {
                            *fx > *end
                                && l.options.fx_target == Some(crate::light::Receiver::Folder(*id))
                        })
                })
            {
                layer_image = adjustment(&gpu, layer_image, fx, frame, scale, None)?;
            }
            for (fx, _) in auto_outlines.iter().filter(|(_, target)| *target == index) {
                layer_image = adjustment(
                    &gpu,
                    layer_image,
                    &doc.layers[*fx],
                    frame,
                    scale,
                    Some(true),
                )?;
            }
            if blur && let Some(optics) = optics.as_ref().filter(|o| o.strength > 0.) {
                if radius >= 0.1 {
                    layer_image = gpu.native_blur(&layer_image, radius, optics)?;
                }
            }
            if pad > 0 {
                layer_image = projector.native(
                    &layer_image,
                    [w, h],
                    Pos2::new(pad as f32, pad as f32),
                    Vec2::X,
                    Vec2::Y,
                )?;
            }
            if let Some((_, end, last, id)) = group {
                group_image = Some(if let Some(previous) = group_image.take() {
                    composite(&gpu, &previous, &layer_image)?
                } else {
                    layer_image
                });
                if index != *last {
                    continue;
                }
                layer_image = group_image.take()?;
                for fx in doc.layers.iter().skip(end + 1).filter(|l| {
                    post && l.visible
                        && crate::folder_groups::layer_active(doc, l, frame)
                        && l.options.fx_target == Some(crate::light::Receiver::Folder(*id))
                }) {
                    layer_image = adjustment(&gpu, layer_image, fx, frame, scale, None)?;
                }
            }
            result = Some(if let Some(previous) = result.take() {
                composite(&gpu, &previous, &layer_image)?
            } else {
                layer_image
            });
            if let Some(work) = work {
                work.report(20 + (67 * index / doc.layers.len().max(1)) as u8);
            }
        }
        let result = result.unwrap_or_else(|| Image::new(&gpu.device, w, h, 4));
        let result = if transparent {
            result
        } else {
            let mut white = vec![0u32; 16];
            white[..3].copy_from_slice(&[w, h, 13]);
            gpu.native_post(&result, &white, None)?
        };
        cache.frames = cache.frames.saturating_add(1);
        gpu.composed_frames
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        cache.reason = "";
        Some(result)
    })
}

#[test]
#[ignore = "requires a GPU; complete device-resident scene parity"]
fn gpu_resident_scene_graph_matches_the_reference() {
    use crate::gpu_lens::{GpuLens, block_on, use_on_worker};
    let instance = eframe::wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
    let gpu = Arc::new(GpuLens::new(device, queue));
    for example in [
        ":builtin-orbit-lights",
        ":builtin-woodland-depth",
        ":builtin-pip-rig",
    ] {
        let doc = crate::examples::project(example).unwrap().unwrap();
        let ctx = egui::Context::default();
        let mut cache = crate::paint::RenderCache::default();
        for frame in [0, 18, 40] {
            use_on_worker(None);
            let expected = crate::exporter::raster(
                &doc,
                frame,
                doc.width,
                doc.height,
                &ctx,
                &mut crate::paint::RenderCache::default(),
            );
            use_on_worker(Some(gpu.clone()));
            let actual =
                crate::exporter::raster(&doc, frame, doc.width, doc.height, &ctx, &mut cache);
            use_on_worker(None);
            assert!(
                cache.native_scene.frames > 0,
                "native graph must execute: {}",
                cache.native_scene.reason
            );
            let rms = (expected
                .iter()
                .zip(&actual)
                .map(|(a, b)| f64::from(a.abs_diff(*b)).powi(2))
                .sum::<f64>()
                / expected.len() as f64)
                .sqrt();
            let max = expected
                .iter()
                .zip(&actual)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            eprintln!("native {example} frame {frame}: max {max}, RMS {rms}");
            assert!(max <= 4 && rms < 0.3);
        }
    }
    // Buffer input remains usable without a CPU shadow copy.
    let pixels = [50, 70, 100, 255].repeat(64 * 32);
    let image = Image::upload(&gpu.device, 64, 32, &pixels);
    assert_eq!(image.read(&gpu.device, &gpu.queue).unwrap(), pixels);
}
