use crate::{
    Editor,
    design::{ACCENT, Icon, button},
    features::Stop,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
impl Editor {
    fn motion_path(&self, ids: &[(usize, usize)]) -> Option<(usize, Pos2, Pos2, [Pos2; 2])> {
        let &(l, m) = ids.first()?;
        let layer = self.doc.layers.get(l)?;
        let index = layer
            .options
            .tweens
            .iter()
            .position(|s| self.frame >= s.start && self.frame <= s.end)?;
        let t = &layer.options.tweens[index];
        let a = crate::vector::control_bounds(layer.frames[t.start].get(m)?).center();
        let b = crate::vector::control_bounds(layer.frames[t.end].get(m)?).center();
        let rail = if a.distance(b) < 0.1 {
            eframe::egui::vec2(160., 0.)
        } else {
            b - a
        };
        let origin = if a.distance(b) < 0.1 {
            a - rail / 2.
        } else {
            a
        };
        Some((
            index,
            a,
            b,
            [
                origin + rail * t.curve.pace[0] + crate::vector::pos(t.curve.path[0]).to_vec2(),
                origin
                    + rail * (1. - t.curve.pace[1])
                    + crate::vector::pos(t.curve.path[1]).to_vec2(),
            ],
        ))
    }

    pub fn visual_canvas_input(
        &mut self,
        ui: &egui::Ui,
        area: Rect,
        view: crate::canvas::View,
        ids: &[(usize, usize)],
    ) -> bool {
        if self.playing
            || !matches!(
                self.tool,
                crate::canvas::Tool::Select
                    | crate::canvas::Tool::Transform
                    | crate::canvas::Tool::Bucket
            )
        {
            return false;
        }
        let Some(screen) = ui.input(|i| i.pointer.interact_pos()) else {
            return false;
        };
        let path = self.motion_path(ids);
        let gradient = ids
            .first()
            .and_then(|&(l, m)| self.doc.layers[l].frames[self.frame].get(m))
            .and_then(|m| m.style.gradient.clone());
        if ui.input(|i| i.pointer.primary_pressed())
            && area.contains(screen)
            && ui.ctx().layer_id_at(screen) == Some(ui.layer_id())
        {
            if let Some((_, _, _, points)) = path {
                if let Some(i) = points
                    .iter()
                    .position(|p| view.screen(*p).distance(screen) < 8.)
                {
                    self.checkpoint();
                    self.animation.path_handle = Some(i);
                }
            }
            if self.animation.path_handle.is_none() {
                if let Some(g) = &gradient {
                    if let Some(i) = [g.start, g.end]
                        .iter()
                        .position(|p| view.screen(crate::vector::pos(*p)).distance(screen) < 8.)
                    {
                        self.checkpoint();
                        self.materialize_motion(ids);
                        self.animation.gradient_drag = Some(i);
                    }
                }
            }
        }
        let handled =
            self.animation.path_handle.is_some() || self.animation.gradient_drag.is_some();
        if ui.input(|i| i.pointer.primary_down()) {
            if let (Some(i), Some((index, a, b, _))) = (self.animation.path_handle, path) {
                let stationary = a.distance(b) < 0.1;
                let rail = if stationary {
                    Vec2::new(160., 0.)
                } else {
                    b - a
                };
                let origin = if stationary { a - rail / 2. } else { a };
                let world = view.world(screen);
                let curve = &mut self.doc.layers[ids[0].0].options.tweens[index].curve;
                curve.pace[i] = if i == 0 {
                    world.distance(origin) / rail.length()
                } else {
                    world.distance(origin + rail) / rail.length()
                }
                .clamp(0., 1.);
                // Keep a spatial bend only perpendicular to the timing rail.
                if !stationary {
                    let endpoint = if i == 0 {
                        origin + rail * curve.pace[i]
                    } else {
                        origin + rail * (1. - curve.pace[i])
                    };
                    let n = rail.rot90().normalized();
                    curve.path[i] = (n * (world - endpoint).dot(n)).into();
                }
            }
            if let Some(i) = self.animation.gradient_drag {
                for &(l, m) in ids {
                    if let Some(g) = self.doc.layers[l].frames[self.frame][m]
                        .style
                        .gradient
                        .as_mut()
                    {
                        if i == 0 {
                            g.start = crate::vector::xy(view.world(screen));
                        } else {
                            g.end = crate::vector::xy(view.world(screen));
                        }
                        self.feature.gradient = g.clone();
                    }
                }
            }
        } else if handled {
            self.animation.path_handle = None;
            self.animation.gradient_drag = None;
            self.recover();
        }
        handled
    }
    pub fn visual_canvas_overlays(
        &self,
        p: &egui::Painter,
        view: crate::canvas::View,
        ids: &[(usize, usize)],
    ) {
        if self.playing {
            return;
        }
        if let Some((index, a, b, handles)) = self.motion_path(ids) {
            let curve = &self.doc.layers[ids[0].0].options.tweens[index].curve;
            let span = &self.doc.layers[ids[0].0].options.tweens[index];
            let frames = span.end - span.start + 1;
            let stationary = a.distance(b) < 0.1;
            for i in 0..frames {
                let t = i as f32 / (frames - 1).max(1) as f32;
                let q = if stationary {
                    (a - Vec2::X * 80.).lerp(a + Vec2::X * 80., curve.time(t))
                } else {
                    curve.point(a, b, curve.time(t))
                };
                p.circle_filled(
                    view.screen(q),
                    if span.start + i == self.frame {
                        4.
                    } else {
                        2.5
                    },
                    if span.start + i == self.frame {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(180, 120, 250)
                    },
                );
            }
            for (i, h) in handles.iter().enumerate() {
                p.line_segment(
                    [view.screen(if i == 0 { a } else { b }), view.screen(*h)],
                    Stroke::new(1., ACCENT),
                );
                p.circle_filled(view.screen(*h), 5., ACCENT);
            }
        }
        if let Some(&(l, m)) = ids.first() {
            if let Some(g) = crate::animation::marks(&self.doc.layers[l], self.frame)
                .get(m)
                .and_then(|m| m.style.gradient.as_ref())
            {
                let a = view.screen(crate::vector::pos(g.start));
                let b = view.screen(crate::vector::pos(g.end));
                p.line_segment([a, b], Stroke::new(2., Color32::BLACK));
                p.line_segment([a, b], Stroke::new(1., Color32::WHITE));
                for point in [a, b] {
                    p.circle_filled(point, 5., Color32::WHITE);
                    p.circle_stroke(point, 5., Stroke::new(1., Color32::BLACK));
                }
            }
        }
    }
    pub fn motion_transform_ui(&mut self, ui: &mut egui::Ui) {
        let ids = self.canvas.selected.clone();
        let (l, m) = ids[0];
        let evaluated = crate::animation::marks(&self.doc.layers[l], self.frame);
        let Some(mark) = evaluated.get(m) else {
            return;
        };
        let center = crate::vector::control_bounds(mark).center();
        let reference = &self.doc.layers[l].frames[0];
        let matrix = reference
            .get(m)
            .map_or([1., 0., 0., 1.], |a| crate::animation::delta(a, mark));
        let sx = Vec2::new(matrix[0], matrix[1]).length().max(0.001);
        let sy = (matrix[0] * matrix[3] - matrix[1] * matrix[2]) / sx;
        let angle = matrix[1].atan2(matrix[0]).to_degrees();
        let mut values = [center.x, center.y, angle, sx * 100., sy * 100.];
        egui::Grid::new("motion-transform")
            .num_columns(2)
            .show(ui, |ui| {
                for (i, label) in ["X", "Y", "Rotation °", "Scale X %", "Scale Y %"]
                    .iter()
                    .enumerate()
                {
                    ui.label(*label);
                    ui.add(
                        egui::DragValue::new(&mut values[i])
                            .range(if i >= 3 { 1. ..=10000. } else { -1e6..=1e6 })
                            .speed(0.5),
                    );
                    ui.end_row();
                }
            });
        if values != [center.x, center.y, angle, sx * 100., sy * 100.] {
            self.checkpoint();
            self.autokey_motion(&ids);
            let translation = Vec2::new(values[0] - center.x, values[1] - center.y);
            let scale = Vec2::new(values[3] / (sx * 100.), values[4] / (sy * 100.).max(0.01));
            let rotation = (values[2] - angle).to_radians();
            for &(l, m) in &ids {
                if self.doc.layers[l].options.locked {
                    continue;
                }
                let original = self.doc.layers[l].frames[self.frame][m].clone();
                self.doc.layers[l].frames[self.frame][m] =
                    crate::vector::transform(&original, |p| {
                        center + crate::canvas::rotate((p - center) * scale, rotation) + translation
                    });
            }
            self.canvas.selected = ids;
            self.recover();
        }
    }
    pub fn sample_apply_at(&mut self, ids: &[(usize, usize)], p: Pos2, tolerance: f32) {
        let source = self
            .doc
            .layers
            .iter()
            .rev()
            .filter(|l| l.visible)
            .find_map(|l| {
                crate::animation::marks(l, self.frame)
                    .iter()
                    .rev()
                    .find(|m| {
                        crate::vector::contains_fill(m, p)
                            && !crate::canvas::stroke_hit(m, p, tolerance)
                    })
                    .cloned()
            });
        self.sample_at(p, tolerance);
        if let Some(source) = source {
            self.feature.gradient_on = source.style.gradient.is_some();
            if let Some(g) = &source.style.gradient {
                self.feature.gradient = g.clone();
            }
            if !ids.is_empty() {
                self.checkpoint();
                self.materialize_motion(ids);
                for &(l, m) in ids {
                    if self.doc.layers[l].options.locked {
                        continue;
                    }
                    let target = &mut self.doc.layers[l].frames[self.frame][m];
                    if target.fill.is_none() {
                        continue;
                    }
                    target.fill = source.fill;
                    target.style.gradient = source.style.gradient.clone();
                    let b = crate::vector::control_bounds(target);
                    if let Some(g) = target.style.gradient.as_mut() {
                        let a = crate::vector::control_bounds(&source);
                        let map = |p: [f32; 2]| {
                            crate::vector::xy(
                                b.min
                                    + (crate::vector::pos(p) - a.min)
                                        / a.size().max(Vec2::splat(0.001))
                                        * b.size(),
                            )
                        };
                        g.start = map(g.start);
                        g.end = map(g.end);
                    }
                }
                self.canvas.selected = ids.to_vec();
                self.recover();
            }
        }
    }
    pub fn symbol_thumbnail(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        frame: usize,
        size: Vec2,
    ) -> egui::Response {
        let (r, response) = ui.allocate_exact_size(size, Sense::click());
        if ui.is_rect_visible(r) {
            ui.painter()
                .rect_filled(r, 4., Color32::from_rgb(235, 235, 238));
            if let Some(s) = self.doc.extras.symbols.iter().find(|s| s.id == id) {
                let mut d = self.doc.clone();
                d.layers = s.layers.clone();
                d.extras.rig = s.rig.clone();
                d.extras.cameras = Default::default();
                d.extras.scenes = Default::default();
                let marks =
                    crate::features::resolve(&d, frame.min(d.layers[0].frames.len() - 1), false);
                let bounds = marks
                    .iter()
                    .map(|(_, _, m)| crate::vector::control_bounds(m))
                    .reduce(|a, b| a.union(b))
                    .unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.)));
                let view = crate::canvas::View {
                    center: r.center(),
                    scale: ((r.width() - 12.) / bounds.width().max(1.))
                        .min((r.height() - 12.) / bounds.height().max(1.)),
                    angle: 0.,
                    stage_center: bounds.center(),
                };
                let p = ui.painter().with_clip_rect(r);
                for (_, _, m) in marks {
                    crate::paint::draw(
                        &p,
                        &m,
                        view,
                        false,
                        false,
                        Some(&mut self.animation.thumbnail_cache),
                    );
                }
            }
        }
        response
    }
    pub fn frame_picker_ui(&mut self, ctx: &egui::Context) {
        let Some((l, m, f)) = self.animation.picker else {
            return;
        };
        if f != self.frame || l >= self.doc.layers.len() || m >= self.doc.layers[l].frames[f].len()
        {
            self.animation.picker = None;
            return;
        }
        let Some(instance) = crate::animation::marks(&self.doc.layers[l], f)[m]
            .style
            .instance
            .clone()
        else {
            self.animation.picker = None;
            return;
        };
        let count = self
            .doc
            .extras
            .symbols
            .iter()
            .find(|s| s.id == instance.symbol)
            .map_or(1, |s| s.layers[0].frames.len());
        let mut open = true;
        let mut selected = None;
        egui::Window::new(crate::i18n::tr("Selector de fotogramas"))
            .open(&mut open)
            .default_width(500.)
            .default_pos(ctx.content_rect().center() - egui::vec2(250., 230.))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(440.).show_rows(
                    ui,
                    94.,
                    count.div_ceil(5),
                    |ui, rows| {
                        for row in rows {
                            ui.horizontal(|ui| {
                                for i in row * 5..((row + 1) * 5).min(count) {
                                    ui.vertical(|ui| {
                                        if self
                                            .symbol_thumbnail(
                                                ui,
                                                instance.symbol,
                                                i,
                                                Vec2::new(80., 64.),
                                            )
                                            .clicked()
                                        {
                                            selected = Some(i);
                                        }
                                        ui.selectable_label(
                                            i == instance.first,
                                            format!("{}", i + 1),
                                        )
                                        .on_hover_text(crate::i18n::tr("Select frame preview"));
                                    });
                                }
                            });
                        }
                    },
                );
            });
        if let Some(i) = selected {
            self.checkpoint();
            self.autokey_motion(&[(l, m)]);
            self.doc.layers[l].frames[f][m]
                .style
                .instance
                .as_mut()
                .unwrap()
                .first = i;
            self.canvas.selected = vec![(l, m)];
            self.recover();
        }
        if !open {
            self.animation.picker = None;
        }
    }
    pub fn visual_gradient_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let old = self.feature.gradient.clone();
        ui.horizontal(|ui| {
            for (radial, icon, label) in [
                (false, Icon::Line, "Linear"),
                (true, Icon::Ellipse, "Radial"),
            ] {
                if button(
                    ui,
                    icon,
                    "",
                    label,
                    self.feature.gradient.radial == radial,
                    true,
                )
                .clicked()
                {
                    self.feature.gradient.radial = radial;
                }
            }
        });
        let (r, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().min(290.), 44.),
            Sense::click_and_drag(),
        );
        let bar = Rect::from_min_max(r.min, r.max - Vec2::new(0., 15.));
        let g = &self.feature.gradient;
        for i in 0..100 {
            let t = i as f32 / 99.;
            let c = g.color(crate::vector::pos(g.start).lerp(crate::vector::pos(g.end), t));
            ui.painter().rect_filled(
                Rect::from_min_max(
                    Pos2::new(bar.left() + bar.width() * i as f32 / 100., bar.top()),
                    Pos2::new(
                        bar.left() + bar.width() * (i + 1) as f32 / 100.,
                        bar.bottom(),
                    ),
                ),
                0.,
                Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
            );
        }
        for (i, s) in self.feature.gradient.stops.iter().enumerate() {
            let p = Pos2::new(bar.left() + bar.width() * s.t, bar.bottom() + 7.);
            ui.painter().circle_filled(
                p,
                5.,
                Color32::from_rgba_unmultiplied(s.color[0], s.color[1], s.color[2], 255),
            );
            ui.painter().circle_stroke(
                p,
                6.,
                Stroke::new(
                    1.,
                    if i == self.animation.stop {
                        ACCENT
                    } else {
                        Color32::GRAY
                    },
                ),
            );
        }
        if (response.clicked() || response.drag_started())
            && let Some(p) = response.interact_pointer_pos()
        {
            self.animation.stop = self
                .feature
                .gradient
                .stops
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (bar.left() + bar.width() * a.t - p.x)
                        .abs()
                        .total_cmp(&(bar.left() + bar.width() * b.t - p.x).abs())
                })
                .map_or(0, |p| p.0);
        }
        self.animation.stop = self
            .animation
            .stop
            .min(self.feature.gradient.stops.len() - 1);
        if response.dragged()
            && let Some(p) = response.interact_pointer_pos()
        {
            self.feature.gradient.stops[self.animation.stop].t =
                ((p.x - bar.left()) / bar.width()).clamp(0., 1.);
        }
        ui.horizontal(|ui| {
            let s = &mut self.feature.gradient.stops[self.animation.stop];
            let mut c =
                Color32::from_rgba_unmultiplied(s.color[0], s.color[1], s.color[2], s.color[3]);
            if ui.color_edit_button_srgba(&mut c).changed() {
                s.color = c.to_srgba_unmultiplied();
            }
            let mut alpha = s.color[3] as f32 / 255. * 100.;
            if ui
                .add(
                    egui::DragValue::new(&mut alpha)
                        .range(0. ..=100.)
                        .suffix(crate::i18n::tr("% opacity")),
                )
                .changed()
            {
                s.color[3] = (alpha / 100. * 255.).round() as u8;
            }
        });
        ui.add(
            egui::Slider::new(
                &mut self.feature.gradient.stops[self.animation.stop].t,
                0. ..=1.,
            )
            .text(crate::i18n::tr("Position")),
        );
        ui.horizontal(|ui| {
            if button(
                ui,
                Icon::Plus,
                "",
                "Add color",
                false,
                self.feature.gradient.stops.len() < 32,
            )
            .clicked()
            {
                self.feature.gradient.stops.push(Stop {
                    t: 0.5,
                    color: [255, 255, 255, 255],
                });
                self.animation.stop = self.feature.gradient.stops.len() - 1;
            }
            if button(
                ui,
                Icon::Trash,
                "",
                "Delete color",
                false,
                self.feature.gradient.stops.len() > 2,
            )
            .clicked()
            {
                self.feature.gradient.stops.remove(self.animation.stop);
                self.animation.stop = 0;
            }
        });
        let g = &mut self.feature.gradient;
        let delta = crate::vector::pos(g.end) - crate::vector::pos(g.start);
        let mut angle = delta.angle().to_degrees();
        ui.horizontal(|ui| {
            let (r, res) = ui.allocate_exact_size(Vec2::splat(64.), Sense::click_and_drag());
            let center = r.center();
            if (res.clicked() || res.dragged())
                && let Some(p) = res.interact_pointer_pos()
            {
                angle = (p - center).angle().to_degrees();
            }
            ui.painter()
                .circle_stroke(center, 25., Stroke::new(1., Color32::GRAY));
            let direction = Vec2::angled(angle.to_radians()) * 24.;
            ui.painter()
                .arrow(center, direction, Stroke::new(2., ACCENT));
            ui.add(
                egui::DragValue::new(&mut angle)
                    .suffix(crate::i18n::tr("°"))
                    .prefix(crate::i18n::tr("Rotation ")),
            );
        });
        g.end = crate::vector::xy(
            crate::vector::pos(g.start) + Vec2::angled(angle.to_radians()) * delta.length().max(1.),
        );
        let selected = g.stops[self.animation.stop].clone();
        g.stops.sort_by(|a, b| a.t.total_cmp(&b.t));
        self.animation.stop = g.stops.iter().position(|s| *s == selected).unwrap_or(0);
        *g != old
    }
    pub fn motion_properties(&mut self, ui: &mut egui::Ui) {
        if self.canvas.selected.is_empty() {
            return;
        }
        crate::design::category(ui, "TRANSFORMACIÓN Y ANIMACIÓN");
        self.motion_transform_ui(ui);
        let has_motion = crate::animation::span(&self.doc.layers[self.layer], self.frame).is_some();
        ui.horizontal(|ui| {
            if button(
                ui,
                Icon::Tween,
                "Motion tween",
                "Create motion tween",
                false,
                true,
            )
            .clicked()
            {
                self.create_motion();
            }
            if button(
                ui,
                Icon::Trash,
                "",
                "Remove motion tween",
                false,
                has_motion,
            )
            .clicked()
            {
                let ids = self.canvas.selected.clone();
                self.checkpoint();
                let f = self.frame;
                self.doc.layers[self.layer]
                    .options
                    .tweens
                    .retain(|t| f < t.start || f > t.end);
                self.canvas.selected = ids;
                self.recover();
            }
        });
        let l = self.canvas.selected[0].0;
        let Some(index) = self.doc.layers[l]
            .options
            .tweens
            .iter()
            .position(|s| self.frame >= s.start && self.frame <= s.end)
        else {
            return;
        };
        let mut curve = self.doc.layers[l].options.tweens[index].curve.clone();
        let span = &self.doc.layers[l].options.tweens[index];
        timing_controls(
            ui,
            &mut curve,
            &mut self.animation.ease_handle,
            span.end - span.start + 1,
        );
        if curve != self.doc.layers[l].options.tweens[index].curve {
            let ids = self.canvas.selected.clone();
            self.checkpoint();
            self.doc.layers[l].options.tweens[index].curve = curve;
            self.canvas.selected = ids;
            self.recover();
        }
    }
}

/// One dot per frame; the two horizontal handles control endpoint speed for
/// translation, rotation, scale, bone angles and puppet pins alike.
pub fn timing_controls(
    ui: &mut egui::Ui,
    curve: &mut crate::animation::Curve,
    handle: &mut Option<usize>,
    frames: usize,
) {
    ui.horizontal(|ui| {
        for (icon, label, pace) in [
            (Icon::Line, "Lineal", [1. / 3., 1. / 3.]),
            (Icon::Next, "Acelerar", [0., 0.6]),
            (Icon::Previous, "Desacelerar", [0.6, 0.]),
            (Icon::Onion, "Suavizar ambos extremos", [0., 0.]),
        ] {
            if button(ui, icon, "", label, false, true).clicked() {
                curve.ease = crate::animation::Curve::default().ease;
                curve.pace = pace;
            }
        }
    });
    let (r, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(260.), 90.),
        Sense::click_and_drag(),
    );
    let r = r.shrink2(Vec2::new(10., 12.));
    let a = Pos2::new(r.left(), r.top() + 12.);
    let b = Pos2::new(r.right(), a.y);
    let endpoints = [a, b];
    let handles = [
        a + Vec2::new(r.width() * curve.pace[0], 32.),
        b - Vec2::new(r.width() * curve.pace[1], -32.),
    ];
    for i in 0..frames.clamp(2, 10000) {
        let t = i as f32 / (frames.max(2) - 1) as f32;
        ui.painter()
            .circle_filled(a.lerp(b, curve.time(t)), 2., ACCENT);
    }
    for i in 0..2 {
        ui.painter()
            .line_segment([endpoints[i], handles[i]], Stroke::new(1., Color32::GRAY));
        ui.painter().circle_filled(handles[i], 5., ACCENT);
    }
    if response.drag_started()
        && let Some(p) = response.interact_pointer_pos()
    {
        *handle = Some(if p.distance(handles[0]) < p.distance(handles[1]) {
            0
        } else {
            1
        });
    }
    if response.dragged()
        && let (Some(i), Some(p)) = (*handle, response.interact_pointer_pos())
    {
        curve.pace[i] = if i == 0 {
            (p.x - a.x) / r.width()
        } else {
            (b.x - p.x) / r.width()
        }
        .clamp(0., 1.);
    }
    ui.small(crate::i18n::tr(
        "Un punto por fotograma · Acerca el tirador para comprimir los fotogramas.",
    ));
}
