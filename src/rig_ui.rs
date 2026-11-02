use crate::{
    Document, Editor,
    canvas::{Tool, View},
    design::{self, Icon},
    i18n::tr,
    rig::{self, Binding, Bone, BoneKey, Deformer, Pin, PinKey},
    vector,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
#[derive(Clone)]
enum Gesture {
    Create {
        start: Pos2,
        end: Pos2,
        parent: Option<u64>,
    },
    Bone {
        id: u64,
    },
    Pin {
        target: u64,
        id: u64,
    },
    Paint {
        last: Pos2,
    },
}
pub struct RigEditor {
    pub bone: Option<u64>,
    pub target: Option<u64>,
    pub pin: Option<u64>,
    pub create: bool,
    pub paint: bool,
    pub radius: f32,
    pub strength: f32,
    pub timing: Option<usize>,
    drag: Option<Gesture>,
}
impl RigEditor {
    pub fn cancel_gesture(&mut self) -> bool {
        self.drag
            .take()
            .is_some_and(|g| !matches!(g, Gesture::Create { .. }))
    }
}
impl Default for RigEditor {
    fn default() -> Self {
        Self {
            bone: None,
            target: None,
            pin: None,
            create: true,
            paint: false,
            radius: 40.,
            strength: 0.35,
            timing: None,
            drag: None,
        }
    }
}
#[derive(Clone)]
pub struct Target {
    pub layer: usize,
    pub mark: usize,
    pub track: u64,
    pub symbol: u64,
    pub name: String,
    pub matrix: rig::Matrix,
    pub bounds: Rect,
}
fn crosses(a: Pos2, b: Pos2, r: Rect) -> bool {
    let d = b - a;
    let mut low = 0f32;
    let mut high = 1f32;
    for (p, q) in [
        (-d.x, a.x - r.left()),
        (d.x, r.right() - a.x),
        (-d.y, a.y - r.top()),
        (d.y, r.bottom() - a.y),
    ] {
        if p.abs() < 1e-8 {
            if q < 0. {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0. {
                low = low.max(t);
            } else {
                high = high.min(t);
            }
            if low > high {
                return false;
            }
        }
    }
    true
}
impl Editor {
    pub fn finish_rig_gesture(&mut self) {
        if self.rigging.cancel_gesture() {
            self.recover();
        }
    }
    fn binding_delta(&self, b: &Binding) -> rig::Matrix {
        self.rig_targets()
            .iter()
            .find(|t| t.track == b.target)
            .map_or(rig::IDENTITY, |t| {
                rig::multiply(t.matrix, rig::inverse(b.matrix).unwrap_or(rig::IDENTITY))
            })
    }

    pub fn rig_targets(&self) -> Vec<Target> {
        let mut out = vec![];
        for (l, layer) in self
            .doc
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.visible && !l.options.locked)
        {
            for (m, mark) in crate::animation::marks(layer, self.frame)
                .iter()
                .enumerate()
            {
                if let Some(i) = &mark.style.instance {
                    let name = self
                        .doc
                        .extras
                        .symbols
                        .iter()
                        .find(|s| s.id == i.symbol)
                        .map_or_else(|| "Símbolo".into(), |s| s.name.clone());
                    out.push(Target {
                        layer: l,
                        mark: m,
                        track: mark.style.track,
                        symbol: i.symbol,
                        name,
                        matrix: i.matrix,
                        bounds: vector::control_bounds(mark),
                    });
                }
            }
        }
        out
    }
    fn rig_hit(&self, p: Pos2, tolerance: f32) -> Option<(usize, usize)> {
        let marks = crate::features::resolve(&self.doc, self.frame, false);
        marks.iter().rev().find_map(|(l, m, mark)| {
            if self.doc.layers[*l].options.locked {
                return None;
            }
            let source = crate::animation::marks(&self.doc.layers[*l], self.frame);
            if source.get(*m).is_some_and(|m| m.style.instance.is_some())
                && (vector::contains_fill(mark, p) || crate::canvas::stroke_hit(mark, p, tolerance))
            {
                Some((*l, *m))
            } else {
                None
            }
        })
    }
    pub fn rig_bind(&mut self, l: usize, m: usize) -> Option<u64> {
        if self.doc.layers[l].options.locked {
            return None;
        }
        self.materialize_motion(&[(l, m)]);
        let mark = &mut self.doc.layers[l].frames[self.frame][m];
        let instance = mark.style.instance.clone()?;
        if rig::inverse(instance.matrix).is_none() {
            self.status = tr("La transformación del símbolo no permite vincularlo").into();
            return None;
        }
        if mark.style.track == 0 {
            mark.style.track = crate::features::id();
        }
        let target = mark.style.track;
        let bounds = vector::control_bounds(mark);
        if !self
            .doc
            .extras
            .rig
            .bindings
            .iter()
            .any(|b| b.target == target)
        {
            if self.doc.extras.rig.bindings.len() >= 512 {
                return None;
            }
            self.doc.extras.rig.bindings.push(Binding::new(
                target,
                instance.symbol,
                instance.matrix,
                bounds,
            ));
        }
        self.rigging.target = Some(target);
        Some(target)
    }
    pub fn add_bone(&mut self, start: Pos2, end: Pos2, parent: Option<u64>) {
        if self.doc.extras.rig.bones.len() >= 128 || start.distance(end) < 0.1 {
            return;
        }
        let targets: Vec<_> = self
            .rig_targets()
            .into_iter()
            .filter(|t| crosses(start, end, t.bounds.expand(self.rigging.radius)))
            .collect();
        self.checkpoint();
        let id = crate::features::id();
        let number = self.doc.extras.rig.bones.len() + 1;
        self.doc.extras.rig.bones.push(Bone {
            id,
            name: format!("{} {number}", tr("Hueso")),
            parent,
            start: vector::xy(start),
            end: vector::xy(end),
            radius: self.rigging.radius,
            ik: false,
            targets: Default::default(),
            keys: vec![BoneKey::default()].into(),
        });
        for t in targets {
            if let Some(track) = self.rig_bind(t.layer, t.mark) {
                self.doc
                    .extras
                    .rig
                    .bones
                    .last_mut()
                    .unwrap()
                    .targets
                    .push(track);
                self.doc.extras.rig.auto_weights(track);
            }
        }
        self.rigging.bone = Some(id);
        self.recover();
    }
    pub fn add_puppet_pin(&mut self, l: usize, m: usize, world: Pos2) {
        self.checkpoint();
        let Some(target) = self.rig_bind(l, m) else {
            return;
        };
        let Some(index) = self
            .doc
            .extras
            .rig
            .bindings
            .iter()
            .position(|b| b.target == target)
        else {
            return;
        };
        let binding = &self.doc.extras.rig.bindings[index];
        if binding.pins.len() >= 32 {
            return;
        }
        let poses = self.doc.extras.rig.poses(self.frame);
        let deformer = Deformer::new(binding, &poses, self.frame);
        let current = crate::animation::marks(&self.doc.layers[l], self.frame)[m]
            .style
            .instance
            .as_ref()
            .unwrap()
            .matrix;
        let delta = rig::multiply(current, rig::inverse(binding.matrix).unwrap());
        let local = rig::point(rig::inverse(delta).unwrap_or(rig::IDENTITY), world);
        let rest = deformer.inverse_point(local);
        if binding
            .pins
            .iter()
            .any(|p| vector::pos(p.rest).distance(rest) < 0.1)
        {
            return;
        }
        let target_position = deformer.inverse_skin(local);
        let id = crate::features::id();
        let number = binding.pins.len() + 1;
        let mut keys = vec![PinKey {
            frame: 0,
            position: vector::xy(rest),
            curve: Default::default(),
        }];
        if self.frame == 0 {
            keys[0].position = vector::xy(target_position);
        } else {
            keys.push(PinKey {
                frame: self.frame,
                position: vector::xy(target_position),
                curve: Default::default(),
            });
        }
        self.doc.extras.rig.bindings[index].pins.push(Pin {
            id,
            name: format!("Pin {number}"),
            rest: vector::xy(rest),
            keys: keys.into(),
        });
        self.rigging.pin = Some(id);
        self.rigging.target = Some(target);
        self.recover();
    }
    pub fn paint_rig_weights(&mut self, p: Pos2, remove: bool, smooth: bool) {
        let Some(bone) = self
            .rigging
            .bone
            .and_then(|id| self.doc.extras.rig.bones.iter().find(|b| b.id == id))
            .cloned()
        else {
            return;
        };
        let poses = self.doc.extras.rig.poses(self.frame);
        let available: std::collections::HashSet<_> =
            self.rig_targets().iter().map(|t| t.track).collect();
        let indices: Vec<_> = self
            .doc
            .extras
            .rig
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| bone.targets.contains(&b.target) && available.contains(&b.target))
            .map(|(i, _)| i)
            .collect();
        for i in indices {
            let binding = &self.doc.extras.rig.bindings[i];
            let deformer = Deformer::new(binding, &poses, self.frame);
            let delta = self.binding_delta(binding);
            let cells = binding.cells.clone();
            let rect = binding.rect();
            let radius = self
                .rigging
                .radius
                .max(rect.size().length() / (rig::GRID - 1) as f32 * 0.65);
            let mut changes = vec![];
            for (n, cell) in cells.iter().enumerate() {
                let pos = rig::point(delta, deformer.point(binding.grid_point(n)));
                let d = pos.distance(p);
                if d > radius {
                    continue;
                }
                let amount = (1. - d / radius).powi(2) * self.rigging.strength;
                let old = cell.weight(bone.id);
                let desired = if smooth {
                    let x = n % rig::GRID;
                    let y = n / rig::GRID;
                    let mut sum = 0.;
                    let mut count = 0.;
                    for yy in y.saturating_sub(1)..=(y + 1).min(rig::GRID - 1) {
                        for xx in x.saturating_sub(1)..=(x + 1).min(rig::GRID - 1) {
                            sum += cells[yy * rig::GRID + xx].weight(bone.id);
                            count += 1.;
                        }
                    }
                    sum / count
                } else if remove {
                    0.
                } else {
                    1.
                };
                changes.push((n, old + (desired - old) * amount));
            }
            let binding = &mut self.doc.extras.rig.bindings[i];
            for (n, w) in changes {
                binding.cells[n].set(bone.id, w);
            }
        }
    }
    pub fn rig_input(&mut self, ui: &egui::Ui, area: Rect, view: View) -> bool {
        if !matches!(self.tool, Tool::Bones | Tool::Puppet) || self.playing {
            return false;
        }
        if ui.input(|i| {
            i.key_down(egui::Key::Space) || i.pointer.button_down(egui::PointerButton::Middle)
        }) {
            return false;
        }
        let Some(screen) = ui.input(|i| i.pointer.interact_pos()) else {
            return true;
        };
        let world = view.world(screen);
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self
                .rigging
                .drag
                .take()
                .is_some_and(|g| !matches!(g, Gesture::Create { .. }))
            {
                self.history(false);
            }
            self.rigging.paint = false;
        }
        if ui.input(|i| i.pointer.primary_pressed())
            && area.contains(screen)
            && ui.ctx().layer_id_at(screen) == Some(ui.layer_id())
        {
            let poses = self.doc.extras.rig.poses(self.frame);
            if self.tool == Tool::Bones {
                if self.rigging.paint && self.rigging.bone.is_some() {
                    self.checkpoint();
                    self.rigging.drag = Some(Gesture::Paint { last: world });
                } else if self.rigging.create
                    || (!self.rigging.paint
                        && poses.values().any(|p| {
                            (view.screen(p.end) + Vec2::new(12., -12.)).distance(screen) < 9.
                        }))
                {
                    let parent = poses
                        .iter()
                        .filter(|(_, p)| {
                            view.screen(p.end).distance(screen)
                                < if self.rigging.create { 12. } else { 26. }
                        })
                        .min_by(|(_, a), (_, b)| {
                            a.end
                                .distance_sq(world)
                                .total_cmp(&b.end.distance_sq(world))
                        })
                        .map(|(id, _)| *id);
                    let start = parent
                        .and_then(|id| poses.get(&id))
                        .map_or(world, |p| p.end);
                    self.rigging.drag = Some(Gesture::Create {
                        start,
                        end: world,
                        parent,
                    });
                } else {
                    let id = poses
                        .iter()
                        .filter(|(_, p)| {
                            rig::segment_distance(world, p.start, p.end) * view.scale < 14.
                        })
                        .min_by(|(_, a), (_, b)| {
                            rig::segment_distance(world, a.start, a.end)
                                .total_cmp(&rig::segment_distance(world, b.start, b.end))
                        })
                        .map(|(id, _)| *id);
                    if let Some(id) = id {
                        self.rigging.bone = Some(id);
                        self.checkpoint();
                        self.rigging.drag = Some(Gesture::Bone { id });
                    }
                }
            } else {
                let mut found = None;
                let targets = self.rig_targets();
                for b in self
                    .doc
                    .extras
                    .rig
                    .bindings
                    .iter()
                    .filter(|b| targets.iter().any(|t| t.track == b.target))
                {
                    for p in &b.pins {
                        let current = rig::point(
                            self.binding_delta(b),
                            rig::skin_point(b, &poses, rig::pin_value(p, self.frame)),
                        );
                        if view.screen(current).distance(screen) < 12. {
                            found = Some((b.target, p.id));
                        }
                    }
                }
                if let Some((target, id)) = found {
                    self.rigging.target = Some(target);
                    self.rigging.pin = Some(id);
                    self.checkpoint();
                    self.rigging.drag = Some(Gesture::Pin { target, id });
                } else if let Some((l, m)) = self.rig_hit(world, 4. / view.scale) {
                    self.add_puppet_pin(l, m, world);
                }
            }
        }
        if let Some(gesture) = self.rigging.drag.clone() {
            match gesture {
                Gesture::Create { start, parent, .. } => {
                    self.rigging.drag = Some(Gesture::Create {
                        start,
                        end: world,
                        parent,
                    })
                }
                Gesture::Bone { id } => {
                    let Some(bone) = self
                        .doc
                        .extras
                        .rig
                        .bones
                        .iter()
                        .find(|b| b.id == id)
                        .cloned()
                    else {
                        self.rigging.drag = None;
                        return true;
                    };
                    if bone.ik {
                        self.doc.extras.rig.solve_ik(id, world, self.frame);
                    } else {
                        let poses = self.doc.extras.rig.poses(self.frame);
                        if let Some(p) = poses.get(&id) {
                            let mut key = rig::bone_value(&bone, self.frame);
                            let d = world - p.start;
                            if d.length_sq() > 1e-6 {
                                key.angle += crate::animation::shortest(d.angle() - p.angle);
                                key.frame = self.frame;
                                self.doc.extras.rig.set_bone_key(id, key);
                            }
                        }
                    }
                }
                Gesture::Pin { target, id } => {
                    let poses = self.doc.extras.rig.poses(self.frame);
                    if let Some(b) = self
                        .doc
                        .extras
                        .rig
                        .bindings
                        .iter()
                        .find(|b| b.target == target)
                    {
                        let deform = Deformer::new(b, &poses, self.frame);
                        let local = rig::point(
                            rig::inverse(self.binding_delta(b)).unwrap_or(rig::IDENTITY),
                            world,
                        );
                        let position = vector::xy(deform.inverse_skin(local));
                        let curve = b
                            .pins
                            .iter()
                            .find(|p| p.id == id)
                            .and_then(|p| p.keys.iter().rev().find(|k| k.frame <= self.frame))
                            .map(|k| k.curve.clone())
                            .unwrap_or_default();
                        self.doc.extras.rig.set_pin_key(
                            target,
                            id,
                            PinKey {
                                frame: self.frame,
                                position,
                                curve,
                            },
                        );
                    }
                }
                Gesture::Paint { last } => {
                    let (remove, smooth) =
                        ui.input(|i| (i.modifiers.ctrl || i.modifiers.command, i.modifiers.shift));
                    let steps = (last.distance(world) / (self.rigging.radius * 0.3).max(1.))
                        .ceil()
                        .clamp(1., 32.) as usize;
                    for step in 1..=steps {
                        self.paint_rig_weights(
                            last.lerp(world, step as f32 / steps as f32),
                            remove,
                            smooth,
                        );
                    }
                    self.rigging.drag = Some(Gesture::Paint { last: world });
                }
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                if let Some(Gesture::Create { start, end, parent }) = self.rigging.drag.take() {
                    if start.distance(end) * view.scale > 5. {
                        let inv = parent
                            .and_then(|id| {
                                self.doc
                                    .extras
                                    .rig
                                    .poses(self.frame)
                                    .get(&id)
                                    .and_then(|p| rig::inverse(p.matrix))
                            })
                            .unwrap_or(rig::IDENTITY);
                        self.add_bone(rig::point(inv, start), rig::point(inv, end), parent);
                    }
                } else {
                    self.rigging.drag = None;
                    self.recover();
                }
            }
            ui.ctx().request_repaint();
        }
        true
    }
    pub fn rig_overlays(&self, p: &egui::Painter, view: View) {
        if self.playing || !matches!(self.tool, Tool::Bones | Tool::Puppet) {
            return;
        }
        let poses = self.doc.extras.rig.poses(self.frame);
        if self.rigging.paint {
            if let Some(id) = self.rigging.bone {
                self.weight_overlay(p, view, id, &poses);
            }
        }
        if self.tool == Tool::Bones {
            for b in &self.doc.extras.rig.bones {
                let Some(pose) = poses.get(&b.id) else {
                    continue;
                };
                let a = view.screen(pose.start);
                let z = view.screen(pose.end);
                let d = (z - a).normalized();
                let n = Vec2::new(-d.y, d.x);
                let width = ((z - a).length() * 0.15).clamp(4., 12.);
                let c = if self.rigging.bone == Some(b.id) {
                    Color32::from_rgb(255, 197, 86)
                } else {
                    Color32::from_rgb(86, 211, 214)
                };
                p.add(egui::Shape::convex_polygon(
                    vec![a, a + d * width + n * width, z, a + d * width - n * width],
                    c.gamma_multiply(0.55),
                    Stroke::new(1.5, c),
                ));
                p.circle(a, 4., Color32::from_gray(30), Stroke::new(1.5, c));
                p.circle(z, 5., Color32::from_gray(30), Stroke::new(1.5, c));
                if !self.rigging.paint
                    && self.rigging.drag.is_none()
                    && p.ctx()
                        .input(|i| i.pointer.hover_pos().is_some_and(|q| q.distance(z) < 28.))
                {
                    p.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    let z = z + Vec2::new(12., -12.);
                    p.circle_filled(z, 9., crate::themes::panel());
                    p.circle_stroke(z, 9., Stroke::new(2., c));
                    p.line_segment([z - Vec2::X * 5., z + Vec2::X * 5.], Stroke::new(2., c));
                    p.line_segment([z - Vec2::Y * 5., z + Vec2::Y * 5.], Stroke::new(2., c));
                }
                p.text(
                    a + Vec2::new(8., -16.),
                    egui::Align2::LEFT_TOP,
                    format!("{}{}", b.name, if b.ik { " · IK" } else { "" }),
                    egui::FontId::proportional(11.),
                    c,
                );
            }
            if let Some(Gesture::Create { start, end, .. }) = &self.rigging.drag {
                p.line_segment(
                    [view.screen(*start), view.screen(*end)],
                    Stroke::new(2., crate::themes::accent()),
                );
            }
            if self.rigging.paint {
                if let Some(pointer) = p.ctx().input(|i| i.pointer.hover_pos()) {
                    p.circle_stroke(
                        pointer,
                        self.rigging.radius * view.scale,
                        Stroke::new(3., Color32::BLACK),
                    );
                    p.circle_stroke(
                        pointer,
                        self.rigging.radius * view.scale,
                        Stroke::new(1., Color32::WHITE),
                    );
                }
            }
        } else {
            let targets = self.rig_targets();
            for b in self
                .doc
                .extras
                .rig
                .bindings
                .iter()
                .filter(|b| targets.iter().any(|t| t.track == b.target))
            {
                for pin in &b.pins {
                    let world = rig::point(
                        self.binding_delta(b),
                        rig::skin_point(b, &poses, rig::pin_value(pin, self.frame)),
                    );
                    let q = view.screen(world);
                    let c = if self.rigging.target == Some(b.target)
                        && self.rigging.pin == Some(pin.id)
                    {
                        Color32::from_rgb(255, 199, 70)
                    } else {
                        crate::themes::accent()
                    };
                    p.circle(q, 7., Color32::from_gray(25), Stroke::new(2., c));
                    p.line_segment([q - Vec2::X * 4., q + Vec2::X * 4.], Stroke::new(1., c));
                    p.line_segment([q - Vec2::Y * 4., q + Vec2::Y * 4.], Stroke::new(1., c));
                }
            }
        }
    }
    fn weight_overlay(
        &self,
        p: &egui::Painter,
        view: View,
        bone: u64,
        poses: &std::collections::HashMap<u64, rig::Pose>,
    ) {
        let Some(bone) = self.doc.extras.rig.bones.iter().find(|b| b.id == bone) else {
            return;
        };
        for t in self
            .rig_targets()
            .iter()
            .filter(|t| bone.targets.contains(&t.track))
        {
            let Some(binding) = self
                .doc
                .extras
                .rig
                .bindings
                .iter()
                .find(|b| b.target == t.track)
            else {
                continue;
            };
            let Some(symbol) = self.doc.extras.symbols.iter().find(|s| s.id == t.symbol) else {
                continue;
            };
            let mut doc: Document = self.doc.clone();
            doc.layers = symbol.layers.clone();
            doc.extras.rig = symbol.rig.clone();
            doc.extras.cameras = Default::default();
            doc.extras.scenes = Default::default();
            let instance = crate::animation::marks(&self.doc.layers[t.layer], self.frame)[t.mark]
                .style
                .instance
                .clone()
                .unwrap();
            let f = crate::features::symbol_frame(
                &instance,
                symbol.layers[0].frames.len(),
                self.frame,
                self.animation.clock,
            );
            let deformer = Deformer::new(binding, poses, self.frame);
            let delta = rig::multiply(t.matrix, rig::inverse(binding.matrix).unwrap());
            for (_, _, mark) in crate::features::resolve(&doc, f, false) {
                let mark = crate::features::affine(&mark, binding.matrix);
                let mut fills = vec![];
                if mark.fill.is_some() {
                    let mut fill = mark.clone();
                    fill.stroke = false;
                    fills.push(fill);
                }
                if mark.stroke {
                    fills.extend(vector::polygons_to_marks(
                        crate::width::outline(&mark),
                        mark.color,
                    ));
                }
                for mark in fills {
                    crate::paint::weight_overlay(p, &mark, view, |q| {
                        (
                            rig::point(delta, deformer.point(q)),
                            binding.weight(q, bone.id),
                        )
                    });
                }
            }
        }
    }
    pub fn rig_properties(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.spacing_mut().slider_width = 105.;
        ui.spacing_mut().text_edit_width = ui.available_width().min(250.);
        if self.tool == Tool::Puppet {
            self.puppet_properties(ui);
            return;
        }
        design::category(ui, "HUESOS");
        ui.horizontal(|ui| {
            if design::button(
                ui,
                Icon::Bones,
                "Crear",
                "Clic y arrastrar crea un hueso",
                self.rigging.create,
                true,
            )
            .clicked()
            {
                self.rigging.create = true;
                self.rigging.paint = false;
            }
            if design::button(
                ui,
                Icon::Rotate,
                "Posar",
                "FK rota · IK mueve",
                !self.rigging.create && !self.rigging.paint,
                true,
            )
            .clicked()
            {
                self.rigging.create = false;
                self.rigging.paint = false;
            }
        });
        design::category(ui, "JERARQUÍA");
        let bones = self.doc.extras.rig.bones.clone();
        egui::ScrollArea::vertical()
            .id_salt("bone-tree")
            .max_height(170.)
            .show(ui, |ui| {
                for b in &bones {
                    let mut depth = 0;
                    let mut parent = b.parent;
                    while let Some(id) = parent {
                        depth += 1;
                        if depth > 64 {
                            break;
                        }
                        parent = bones.iter().find(|b| b.id == id).and_then(|b| b.parent);
                    }
                    ui.horizontal(|ui| {
                        ui.add_space(depth as f32 * 10.);
                        if design::button(
                            ui,
                            if b.ik { Icon::IK } else { Icon::Bones },
                            &b.name,
                            "Seleccionar hueso",
                            self.rigging.bone == Some(b.id),
                            true,
                        )
                        .clicked()
                        {
                            self.rigging.bone = Some(b.id);
                        }
                    });
                }
            });
        let Some(index) = self
            .rigging
            .bone
            .and_then(|id| self.doc.extras.rig.bones.iter().position(|b| b.id == id))
        else {
            ui.small(tr(
                "Crea huesos sobre los símbolos. Cada hueso puede influir en varias piezas.",
            ));
            return;
        };
        let mut bone = self.doc.extras.rig.bones[index].clone();
        let mut changed = ui.text_edit_singleline(&mut bone.name).changed();
        let old_parent = bone.parent;
        let parent_name = bone
            .parent
            .and_then(|id| bones.iter().find(|b| b.id == id))
            .map_or(tr("Sin padre"), |b| b.name.as_str());
        egui::ComboBox::from_id_salt("bone-parent")
            .selected_text(format!("{}: {parent_name}", tr("Padre")))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut bone.parent, None, tr("Sin padre"));
                for b in &bones {
                    if self.doc.extras.rig.can_parent(bone.id, Some(b.id)) {
                        ui.selectable_value(&mut bone.parent, Some(b.id), &b.name);
                    }
                }
            });
        changed |= old_parent != bone.parent;
        changed |= ui.checkbox(&mut bone.ik, tr("Habilitar IK")).changed();
        changed |= ui
            .add(egui::Slider::new(&mut bone.radius, 1. ..=500.).text(tr("Radio")))
            .changed();
        if changed {
            self.checkpoint();
            self.doc.extras.rig.bones[index] = bone.clone();
            self.recover();
        }
        design::category(ui, "SÍMBOLOS AFECTADOS");
        let targets = self.rig_targets();
        let mut attachment = None;
        for t in targets {
            let mut attached = t.track != 0 && bone.targets.contains(&t.track);
            if ui
                .checkbox(&mut attached, format!("{} · {}", t.name, t.mark + 1))
                .changed()
            {
                attachment = Some((t, attached));
            }
        }
        if let Some((t, attached)) = attachment {
            self.checkpoint();
            if attached {
                if let Some(track) = self.rig_bind(t.layer, t.mark) {
                    if !self.doc.extras.rig.bones[index].targets.contains(&track) {
                        self.doc.extras.rig.bones[index].targets.push(track);
                    }
                    self.doc.extras.rig.auto_weights(track);
                }
            } else {
                self.doc.extras.rig.detach(bone.id, t.track);
            }
            self.recover();
        }
        design::category(ui, "INFLUENCIAS");
        if design::button(
            ui,
            Icon::WeightPaint,
            "Pintado de pesos",
            "Negro: 0 · blanco: 1",
            self.rigging.paint,
            true,
        )
        .clicked()
        {
            self.rigging.paint = !self.rigging.paint;
            self.rigging.create = false;
        }
        if self.rigging.paint {
            ui.add(egui::Slider::new(&mut self.rigging.radius, 2. ..=300.).text(tr("Radio")));
            ui.add(egui::Slider::new(&mut self.rigging.strength, 0.01..=1.).text(tr("Intensidad")));
            ui.small(tr("Clic añade · Ctrl quita · Mayús suaviza"));
            ui.horizontal(|ui| {
                ui.label(crate::i18n::tr("0"));
                let (r, _) = ui.allocate_exact_size(Vec2::new(140., 12.), egui::Sense::hover());
                let mut mesh = egui::Mesh::default();
                for (pos, color) in [
                    (r.left_top(), Color32::BLACK),
                    (r.right_top(), Color32::WHITE),
                    (r.right_bottom(), Color32::WHITE),
                    (r.left_bottom(), Color32::BLACK),
                ] {
                    mesh.colored_vertex(pos, color);
                }
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                ui.painter().add(mesh);
                ui.label(crate::i18n::tr("1"));
            });
        }
        if design::button(
            ui,
            Icon::AutoWeights,
            "Autopintado de huesos",
            "Recalcular pesos de los símbolos vinculados",
            false,
            !bone.targets.is_empty(),
        )
        .clicked()
        {
            self.checkpoint();
            for target in bone.targets.iter().copied() {
                self.doc.extras.rig.auto_weights(target);
            }
            self.recover();
        }
        design::category(ui, "ANIMACIÓN");
        let mut value = rig::bone_value(&bone, self.frame);
        let mut angle = value.angle.to_degrees();
        let mut pose_changed = ui
            .add(
                egui::DragValue::new(&mut angle)
                    .prefix(tr("Rotación: "))
                    .suffix(crate::i18n::tr("°")),
            )
            .changed();
        value.angle = angle.to_radians();
        pose_changed |= ui
            .add(egui::Slider::new(&mut value.scale, 0.1..=10.).text(tr("Escala")))
            .changed();
        if pose_changed {
            self.checkpoint();
            value.frame = self.frame;
            self.doc.extras.rig.set_bone_key(bone.id, value);
            self.recover();
        }
        self.rig_animation_buttons(ui);
        let current = self.doc.extras.rig.bones[index]
            .keys
            .iter()
            .rposition(|k| k.frame <= self.frame)
            .unwrap_or(0);
        if let Some(key) = self.doc.extras.rig.bones[index].keys.get(current) {
            let mut curve = key.curve.clone();
            let old = curve.clone();
            let frames = self.doc.extras.rig.bones[index]
                .keys
                .get(current + 1)
                .map_or(
                    self.doc.layers[0]
                        .frames
                        .len()
                        .saturating_sub(key.frame)
                        .max(1),
                    |next| next.frame - key.frame + 1,
                );
            crate::visual_controls::timing_controls(
                ui,
                &mut curve,
                &mut self.rigging.timing,
                frames,
            );
            if curve != old {
                self.checkpoint();
                self.doc.extras.rig.bones[index].keys[current].curve = curve;
                self.recover();
            }
        }
        if design::button(
            ui,
            Icon::Trash,
            "Eliminar hueso",
            "Conservar el dibujo original",
            false,
            true,
        )
        .clicked()
        {
            self.checkpoint();
            self.doc.extras.rig.remove_bone(bone.id);
            self.rigging.bone = None;
            self.recover();
        }
    }
    fn puppet_properties(&mut self, ui: &mut egui::Ui) {
        design::category(ui, "PUPPET VECTORIAL");
        ui.small(tr(
            "Clic en un símbolo añade un pin. Arrastra sus pines para deformar sin rasterizar.",
        ));
        let mut select = None;
        for binding in &self.doc.extras.rig.bindings {
            if binding.pins.is_empty() {
                continue;
            }
            for pin in &binding.pins {
                if design::button(
                    ui,
                    Icon::Puppet,
                    &pin.name,
                    "Seleccionar pin",
                    self.rigging.target == Some(binding.target) && self.rigging.pin == Some(pin.id),
                    true,
                )
                .clicked()
                {
                    select = Some((binding.target, pin.id));
                }
            }
        }
        if let Some((t, p)) = select {
            self.rigging.target = Some(t);
            self.rigging.pin = Some(p);
        }
        let Some((bi, pi)) = self
            .rigging
            .target
            .and_then(|t| {
                self.doc
                    .extras
                    .rig
                    .bindings
                    .iter()
                    .position(|b| b.target == t)
            })
            .and_then(|b| {
                self.rigging
                    .pin
                    .and_then(|id| {
                        self.doc.extras.rig.bindings[b]
                            .pins
                            .iter()
                            .position(|p| p.id == id)
                    })
                    .map(|p| (b, p))
            })
        else {
            return;
        };
        let binding = &self.doc.extras.rig.bindings[bi];
        let pin = binding.pins[pi].clone();
        let target = binding.target;
        let mut name = pin.name.clone();
        if ui.text_edit_singleline(&mut name).changed() {
            self.checkpoint();
            self.doc.extras.rig.bindings[bi].pins[pi].name = name;
            self.recover();
        }
        design::category(ui, "POSICIÓN DEL PIN");
        let position = rig::pin_value(&pin, self.frame);
        let mut values = [position.x, position.y];
        let mut changed = false;
        ui.horizontal(|ui| {
            for (i, v) in values.iter_mut().enumerate() {
                changed |= ui
                    .add(
                        egui::DragValue::new(v)
                            .prefix(if i == 0 { "X " } else { "Y " })
                            .range(-1e6..=1e6),
                    )
                    .changed();
            }
        });
        if changed {
            self.checkpoint();
            self.doc.extras.rig.set_pin_key(
                target,
                pin.id,
                PinKey {
                    frame: self.frame,
                    position: values,
                    curve: Default::default(),
                },
            );
            self.recover();
        }
        design::category(ui, "ANIMACIÓN");
        self.rig_animation_buttons(ui);
        let current = pin
            .keys
            .iter()
            .rposition(|k| k.frame <= self.frame)
            .unwrap_or(0);
        if let Some(key) = pin.keys.get(current) {
            let mut curve = key.curve.clone();
            let old = curve.clone();
            let frames = pin.keys.get(current + 1).map_or(
                self.doc.layers[0]
                    .frames
                    .len()
                    .saturating_sub(key.frame)
                    .max(1),
                |next| next.frame - key.frame + 1,
            );
            crate::visual_controls::timing_controls(
                ui,
                &mut curve,
                &mut self.rigging.timing,
                frames,
            );
            if curve != old {
                self.checkpoint();
                self.doc.extras.rig.bindings[bi].pins[pi].keys[current].curve = curve;
                self.recover();
            }
        }
        if design::button(
            ui,
            Icon::Trash,
            "Eliminar pin",
            "Conservar la geometría original",
            false,
            true,
        )
        .clicked()
        {
            self.checkpoint();
            self.doc.extras.rig.bindings[bi]
                .pins
                .retain(|p| p.id != pin.id);
            self.rigging.pin = None;
            self.recover();
        }
    }
    fn rig_animation_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if design::button(
                ui,
                Icon::Keyframe,
                "Clave",
                "Crear clave de pose en el fotograma actual",
                false,
                true,
            )
            .clicked()
            {
                self.rig_key_action(0);
            }
            if design::button(
                ui,
                Icon::Trash,
                "",
                "Eliminar clave actual",
                false,
                self.frame > 0,
            )
            .clicked()
            {
                self.rig_key_action(4);
            }
        });
        ui.small(tr(
            "Cambia de fotograma y posa: las claves se interpolan automáticamente.",
        ));
    }
    pub fn rig_key_action(&mut self, action: usize) {
        if action == 2 || action == 3 {
            let count = self.doc.layers[0].frames.len();
            let next = if action == 2 {
                (count + 1).min(10000)
            } else {
                count.saturating_sub(1).max(1)
            };
            self.set_duration(next as f32 / self.doc.fps as f32);
            return;
        }
        self.checkpoint();
        if self.tool == Tool::Bones {
            if let Some(id) = self.rigging.bone {
                if action == 4 && self.frame > 0 {
                    if let Some(b) = self.doc.extras.rig.bones.iter_mut().find(|b| b.id == id) {
                        b.keys.retain(|k| k.frame != self.frame);
                    }
                } else if let Some(b) = self.doc.extras.rig.bones.iter().find(|b| b.id == id) {
                    let mut key = rig::bone_value(b, self.frame);
                    if action == 1 {
                        key = BoneKey::default();
                    }
                    key.frame = self.frame;
                    self.doc.extras.rig.set_bone_key(id, key);
                }
            }
        } else if let (Some(target), Some(id)) = (self.rigging.target, self.rigging.pin) {
            let pin = self
                .doc
                .extras
                .rig
                .bindings
                .iter()
                .find(|b| b.target == target)
                .and_then(|b| b.pins.iter().find(|p| p.id == id))
                .cloned();
            if let Some(pin) = pin {
                if action == 4 && self.frame > 0 {
                    if let Some(p) = self
                        .doc
                        .extras
                        .rig
                        .bindings
                        .iter_mut()
                        .find(|b| b.target == target)
                        .and_then(|b| b.pins.iter_mut().find(|p| p.id == id))
                    {
                        p.keys.retain(|k| k.frame != self.frame);
                    }
                } else {
                    let position = if action == 1 {
                        pin.rest
                    } else {
                        vector::xy(rig::pin_value(&pin, self.frame))
                    };
                    self.doc.extras.rig.set_pin_key(
                        target,
                        id,
                        PinKey {
                            frame: self.frame,
                            position,
                            curve: Default::default(),
                        },
                    );
                }
            }
        }
        self.recover();
    }
}

#[cfg(test)]
mod audit_gesture_tests {
    use super::*;
    #[test]
    fn switching_tools_and_history_discard_pending_rig_gestures() {
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        e.recovery =
            std::env::temp_dir().join(format!("audit-gesture-{}.json", std::process::id()));
        e.tool = Tool::Bones;
        e.rigging.drag = Some(Gesture::Create {
            start: Pos2::ZERO,
            end: Pos2::new(20., 0.),
            parent: None,
        });
        e.activate_tool(Tool::Pan);
        assert!(e.rigging.drag.is_none());
        assert!(e.doc.extras.rig.bones.is_empty());
        e.tool = Tool::Puppet;
        e.checkpoint();
        e.rigging.drag = Some(Gesture::Pin {
            target: 123,
            id: 456,
        });
        e.history(false);
        assert!(e.rigging.drag.is_none());
    }
}
