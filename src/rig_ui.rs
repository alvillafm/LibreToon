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
    ScaleBone {
        id: u64,
        distance: f32,
        scale: f32,
        stretch: Option<f32>,
    },
    ReanchorBone {
        id: u64,
        grab: Pos2,
        original: Bone,
    },
    ReanchorPin {
        target: u64,
        id: u64,
    },
    MoveBone {
        id: u64,
        grab: Pos2,
        translation: Vec2,
        parent_angle: f32,
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
    reanchor: bool,
    pub paint: bool,
    show_weights: bool,
    show_unweighted: bool,
    pub radius: f32,
    pub strength: f32,
    pub timing: Option<usize>,
    character_setup: bool,
    pub(crate) edit_character_bones: bool,
    pub(crate) test_character_pose: Option<[f32; 2]>,
    turn_drag: Option<(bool, f32, f32)>,
    selection_rotation: Option<(u64, usize, f32, Pos2, bool)>,
    turn_slider: Option<(bool, usize, Rect)>,
    drag: Option<Gesture>,
    grab: Option<Pos2>,
    gesture_started: bool,
    grab_offset: Vec2,
    angle_offset: f32,
}
impl RigEditor {
    pub fn cancel_gesture(&mut self) -> bool {
        let changed = self.gesture_started;
        self.gesture_started = false;
        self.grab = None;
        self.drag
            .take()
            .is_some_and(|g| matches!(g, Gesture::Paint { .. }) || changed)
    }
}
impl Default for RigEditor {
    fn default() -> Self {
        Self {
            bone: None,
            target: None,
            pin: None,
            create: true,
            reanchor: false,
            paint: false,
            show_weights: false,
            show_unweighted: false,
            radius: 40.,
            strength: 0.35,
            timing: None,
            character_setup: false,
            edit_character_bones: false,
            test_character_pose: None,
            turn_drag: None,
            selection_rotation: None,
            turn_slider: None,
            grab: None,
            gesture_started: false,
            grab_offset: Vec2::ZERO,
            angle_offset: 0.,
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
        for (l, layer) in self.doc.layers.iter().enumerate().filter(|(_, l)| {
            l.visible
                && !l.options.locked
                && crate::folder_groups::layer_active(&self.doc, l, self.frame)
        }) {
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
                && self.depth_hit(*l, mark, p, tolerance)
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
        let rig_frame = self.group_rig_frame();
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
        let poses = self.doc.extras.rig.poses(rig_frame);
        let deformer = Deformer::new(binding, &poses, rig_frame);
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
        if rig_frame == 0 {
            keys[0].position = vector::xy(target_position);
        } else {
            keys.push(PinKey {
                frame: rig_frame,
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
        let rig_frame = self.group_rig_frame();
        let Some(bone) = self
            .rigging
            .bone
            .and_then(|id| self.doc.extras.rig.bones.iter().find(|b| b.id == id))
            .cloned()
        else {
            return;
        };
        if self.doc.extras.rig.weight_locks.contains(&bone.id) {
            return;
        }
        let poses = self.doc.extras.rig.poses(rig_frame);
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
            let deformer = Deformer::new(binding, &poses, rig_frame);
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
    fn character_setup(&mut self, ctx: &egui::Context, frame: usize) {
        if !self.rigging.character_setup {
            return;
        }
        let rig = &self.doc.extras.rig;
        let mut draft = rig.character_turns.clone();
        let mut open = true;
        let mut changed = false;
        egui::Window::new(tr("Character Setup"))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(tr("Assign saved rig poses to the front and side views."));
                for (title, channel) in [
                    ("Head turn", &mut draft.head),
                    ("Body turn", &mut draft.body),
                ] {
                    egui::CollapsingHeader::new(tr(title))
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ComboBox::from_id_salt("root")
                                .selected_text(
                                    rig.bones
                                        .iter()
                                        .find(|bone| Some(bone.id) == channel.root)
                                        .map_or(tr("Choose root bone"), |bone| bone.name.as_str()),
                                )
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(&mut channel.root, None, tr("None"))
                                        .changed();
                                    for bone in &rig.bones {
                                        changed |= ui
                                            .selectable_value(
                                                &mut channel.root,
                                                Some(bone.id),
                                                &bone.name,
                                            )
                                            .changed();
                                    }
                                });
                            for (slot, label) in [
                                "Left profile",
                                "Left three-quarter",
                                "Front",
                                "Right three-quarter",
                                "Right profile",
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                ui.push_id(slot, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(tr(label));
                                        let target = &mut channel.views[slot];
                                        egui::ComboBox::from_id_salt("pose")
                                            .selected_text(target.as_ref().map_or("—", |p| &p.name))
                                            .show_ui(ui, |ui| {
                                                for pose in &self.doc.extras.rig_poses {
                                                    if ui
                                                        .selectable_label(false, &pose.name)
                                                        .clicked()
                                                    {
                                                        *target =
                                                            Some(std::sync::Arc::new(pose.clone()));
                                                        changed = true;
                                                    }
                                                }
                                            });
                                        if design::button(
                                            ui,
                                            Icon::Save,
                                            "",
                                            "Capture current bone pose",
                                            false,
                                            channel.root.is_some(),
                                        )
                                        .clicked()
                                        {
                                            *target = Some(std::sync::Arc::new(rig::SavedPose {
                                                controls: Vec::new(),
                                                name: tr(label).into(),
                                                bones: rig
                                                    .bones
                                                    .iter()
                                                    .map(|bone| {
                                                        (bone.id, rig::bone_value(bone, frame))
                                                    })
                                                    .collect(),
                                            }));
                                            changed = true;
                                        }
                                        if design::button(
                                            ui,
                                            Icon::Trash,
                                            "",
                                            "Clear view",
                                            false,
                                            target.is_some(),
                                        )
                                        .clicked()
                                        {
                                            *target = None;
                                            changed = true;
                                        }
                                    })
                                });
                            }
                        });
                }
            });
        self.rigging.character_setup = open;
        if changed {
            self.checkpoint();
            self.doc.extras.rig.character_turns = draft;
            self.recover();
        }
    }

    fn selected_bone_ids(&self, selected: &[(usize, usize)]) -> std::collections::HashSet<u64> {
        let rig = &self.doc.extras.rig;
        let mut visible = std::collections::HashSet::new();
        let tracks = self.selected_rig_tracks(selected);
        for bone in &rig.bones {
            if !bone.targets.iter().any(|track| tracks.contains(track)) {
                continue;
            }
            let mut root = bone;
            for _ in 0..rig.bones.len() {
                let Some(parent) = root
                    .parent
                    .and_then(|id| rig.bones.iter().find(|bone| bone.id == id))
                else {
                    break;
                };
                root = parent;
            }
            if !visible.contains(&root.id) {
                visible.extend(rig.pose_branch(root.id, None));
            }
        }
        visible
    }

    fn turn_controls(&mut self, ui: &egui::Ui, area: Rect, view: View, frame: usize) -> bool {
        if self.tool != Tool::Bones
            || self.playing
            || !self.feature.symbol_stack.is_empty()
            || (self.tool == Tool::Bones && self.rigging.create)
        {
            if self.rigging.turn_drag.take().is_some() {
                self.recover();
            }
            return false;
        }
        let rig = &self.doc.extras.rig;
        let turns = &rig.character_turns;
        if turns.head.root.is_none() && turns.body.root.is_none() {
            return false;
        }
        let poses = crate::character::poses(&self.doc, rig, frame);
        let mut edit = None;
        let mut consumed = false;
        for (head, channel, title) in [
            (true, &turns.head, "Head turn"),
            (false, &turns.body, "Body turn"),
        ] {
            let Some(pose) = channel.root.and_then(|id| poses.get(&id)) else {
                continue;
            };
            let character = rig.character_symbol.is_some();
            if !character
                && (channel.views[2].is_none() || channel.views.iter().flatten().count() < 2)
            {
                continue;
            }
            let half_rail = 38.;
            let spacing = half_rail / if character { 4. } else { 2. };
            let center = view.screen(pose.start) + Vec2::new(0., if head { -24. } else { 24. });
            let rect = Rect::from_center_size(center, Vec2::new(90., 22.));
            let response = ui
                .interact(
                    rect.intersect(area),
                    ui.scope_id().with(("turn-control", head)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
                .on_hover_text(tr(title));
            let painter = ui
                .ctx()
                .layer_painter(egui::LayerId::new(
                    egui::Order::Middle,
                    ui.scope_id().with("character-turn-overlay"),
                ))
                .with_clip_rect(area);
            let color = crate::themes::accent();
            painter.rect_filled(rect, 5., crate::themes::panel());
            painter.line_segment(
                [center - Vec2::X * half_rail, center + Vec2::X * half_rail],
                Stroke::new(1., color),
            );
            let range = if character { 4 } else { 2 };
            for i in -range..=range {
                if character || channel.views[(i + 2) as usize].is_some() {
                    painter.circle_filled(center + Vec2::X * i as f32 * spacing, 2.5, color);
                }
            }
            painter.circle_filled(center + Vec2::X * channel.value(frame) * spacing, 5., color);
            consumed |= response.is_pointer_button_down_on();
            if let Some(pointer) = response.interact_pointer_pos() {
                if response.clicked() {
                    let raw = (pointer.x - center.x) / spacing;
                    let index = (raw.round() as i32 + 2).clamp(0, 4) as usize;
                    let view = index as f32 - 2.;
                    let value = if character {
                        raw.round()
                    } else if channel.views[index].is_some() && (view - raw).abs() < 0.25 {
                        view
                    } else {
                        raw
                    };
                    edit = Some((head, value, true, true));
                } else if response.drag_started() {
                    self.rigging.turn_drag = Some((head, pointer.x, channel.value(frame)));
                    edit = Some((head, channel.value(frame), true, false));
                } else if let Some((active, start, value)) = self.rigging.turn_drag
                    && active == head
                {
                    edit = Some((
                        head,
                        value + (pointer.x - start) / spacing,
                        false,
                        response.drag_stopped(),
                    ));
                }
            }
        }
        if let Some((head, value, start, end)) = edit {
            if start {
                self.checkpoint();
            }
            let turns = &mut self.doc.extras.rig.character_turns;
            let limit = if self.doc.extras.rig.character_symbol.is_some() {
                4.
            } else {
                2.
            };
            let channel = if head {
                &mut turns.head
            } else {
                &mut turns.body
            };
            channel.set(frame, value.clamp(-limit, limit));
            if end {
                self.rigging.turn_drag = None;
                self.recover();
            }
            return true;
        }
        if !ui.input(|i| i.pointer.primary_down()) && self.rigging.turn_drag.take().is_some() {
            self.recover();
        }
        consumed
    }

    fn selection_bone_input(
        &mut self,
        ui: &egui::Ui,
        area: Rect,
        view: View,
        frame: usize,
        selected: &[(usize, usize)],
    ) -> bool {
        let active = self.rigging.selection_rotation;
        let (pointer, pressed, down, cancel, panning) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.key_pressed(egui::Key::Escape),
                i.key_down(egui::Key::Space) || i.pointer.middle_down(),
            )
        });
        if let Some((id, frame, offset, start, mut edited)) = active {
            if cancel {
                self.rigging.selection_rotation = None;
                if edited {
                    self.history(false);
                }
                return true;
            }
            if !edited && pointer.is_some_and(|pointer| pointer.distance(start) >= 3.) {
                self.checkpoint();
                edited = true;
                self.rigging.selection_rotation = Some((id, frame, offset, start, edited));
            }
            let rig = &self.doc.extras.rig;
            if edited
                && let Some(pointer) = pointer
                && let Some(bone) = rig.bones.iter().find(|bone| bone.id == id)
                && let Some(pose) = crate::character::poses(&self.doc, rig, frame).get(&id)
            {
                let direction = view.world(pointer) - pose.start;
                if direction.length_sq() > 1e-6 {
                    let mut key = rig::bone_value(bone, frame);
                    key.frame = frame;
                    key.angle +=
                        crate::animation::shortest(direction.angle() + offset - pose.angle);
                    self.doc.extras.rig.set_bone_key(id, key);
                }
            }
            if !down {
                self.rigging.selection_rotation = None;
                if edited {
                    self.recover();
                }
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            return true;
        }
        let Some(pointer) =
            pointer.filter(|&p| area.contains(p) && ui.ctx().layer_id_at(p) == Some(ui.layer_id()))
        else {
            return false;
        };
        if panning || selected.is_empty() {
            return false;
        }
        let visible = self.selected_bone_ids(selected);
        if visible.is_empty() {
            return false;
        }
        let poses = crate::character::poses(&self.doc, &self.doc.extras.rig, frame);
        let hit = poses
            .iter()
            .filter(|(id, _)| visible.contains(id))
            .filter_map(|(&id, pose)| {
                let start = view.screen(pose.start);
                let axis = view.screen(pose.end) - start;
                let t = ((pointer - start).dot(axis) / axis.length_sq().max(1e-6)).clamp(0., 1.);
                let distance = pointer.distance(start + axis * t);
                (distance < 9.).then_some((distance, id, pose))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let Some((_, id, pose)) = hit else {
            return false;
        };
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        if !pressed {
            return false;
        }
        self.rigging.bone = Some(id);
        self.rigging.selection_rotation = Some((
            id,
            frame,
            pose.angle - (view.world(pointer) - pose.start).angle(),
            pointer,
            false,
        ));
        true
    }

    fn selection_turn_slider(
        &mut self,
        ui: &egui::Ui,
        area: Rect,
        view: View,
        frame: usize,
        selected: &[(usize, usize)],
    ) -> bool {
        let rig = &self.doc.extras.rig;
        let Some((layer, index)) = selected.last().copied() else {
            return false;
        };
        let Some(layer) = self.doc.layers.get(layer) else {
            return false;
        };
        let marks = crate::animation::marks(layer, self.frame);
        let Some(mark) = marks.get(index) else {
            return false;
        };
        if mark.style.instance.is_none() || mark.style.track == 0 {
            return false;
        }
        if rig.character_symbol.is_none() {
            return false;
        }
        let turns = &rig.character_turns;
        let drives_piece = |root| {
            rig.pose_branch(root, None).iter().any(|id| {
                rig.bones
                    .iter()
                    .any(|bone| bone.id == *id && bone.targets.contains(&mark.style.track))
            })
        };
        let head = self
            .rigging
            .turn_slider
            .map(|(head, _, _)| head)
            .or_else(|| {
                if turns.head.root.is_some_and(drives_piece) {
                    Some(true)
                } else if rig.artwork_turn_target(mark.style.track) {
                    Some(false)
                } else {
                    turns
                        .body
                        .root
                        .filter(|&root| drives_piece(root))
                        .map(|_| false)
                }
            });
        let Some(head) = head else { return false };
        let frame = self
            .rigging
            .turn_slider
            .map_or(frame, |(_, frame, _)| frame);
        let channel = if head { &turns.head } else { &turns.body };
        let value = channel.value(frame);
        let name = channel
            .root
            .and_then(|id| rig.bones.iter().find(|bone| bone.id == id))
            .map_or(tr(if head { "Head turn" } else { "Body turn" }), |bone| {
                bone.name.as_str()
            });
        let bounds = crate::vector::control_bounds(mark);
        if !bounds.is_finite() || area.width() < 120. {
            return false;
        }
        let bounds = Rect::from_two_pos(view.screen(bounds.min), view.screen(bounds.max));
        let width = area.width().min(198.);
        let right = bounds.right() + 12.;
        let x = if right + width <= area.right() {
            right
        } else {
            bounds.left() - width - 12.
        };
        let rect = Rect::from_min_size(
            Pos2::new(
                x.clamp(area.left(), area.right() - width),
                (bounds.center().y - 25.).clamp(area.top(), (area.bottom() - 50.).max(area.top())),
            ),
            Vec2::new(width, 50.),
        );
        let rect = self.rigging.turn_slider.map_or(rect, |(_, _, rect)| rect);
        let center = rect.center() + Vec2::Y * 10.;
        let rail = rect.width() / 2. - 14.;
        let response = ui
            .interact(
                rect.intersect(area),
                ui.scope_id().with("selection-turn-slider"),
                egui::Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        let painter = ui
            .ctx()
            .layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                ui.scope_id().with("selection-turn-overlay"),
            ))
            .with_clip_rect(area);
        let color = crate::themes::accent();
        painter.rect(
            rect,
            7.,
            crate::themes::panel(),
            Stroke::new(1., crate::themes::line()),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center() - Vec2::Y * 12.,
            egui::Align2::CENTER_CENTER,
            format!("{name} · {}  {:.0}°", tr("Turnaround"), value * 45.),
            egui::FontId::proportional(11.),
            crate::themes::muted(),
        );
        painter.line_segment(
            [center - Vec2::X * rail, center + Vec2::X * rail],
            Stroke::new(1., color),
        );
        painter.circle_filled(
            center + Vec2::X * rail * (value / 4.).clamp(-1., 1.),
            5.,
            color,
        );
        if let Some(pointer) = response.interact_pointer_pos()
            && (response.clicked()
                || response.drag_started()
                || response.dragged()
                || response.drag_stopped())
        {
            if self.rigging.turn_slider.is_none() {
                self.checkpoint();
                self.rigging.turn_slider = Some((head, frame, rect));
            }
            let turns = &mut self.doc.extras.rig.character_turns;
            let channel = if head {
                &mut turns.head
            } else {
                &mut turns.body
            };
            channel.set(frame, ((pointer.x - center.x) / rail).clamp(-1., 1.) * 4.);
        }
        if !ui.input(|i| i.pointer.primary_down()) && self.rigging.turn_slider.take().is_some() {
            self.recover();
        }
        response.is_pointer_button_down_on() || response.clicked() || response.drag_stopped()
    }

    pub fn rig_input(
        &mut self,
        ui: &egui::Ui,
        area: Rect,
        view: View,
        selected: &[(usize, usize)],
    ) -> bool {
        let rig_frame = self.group_rig_frame();
        if self.turn_controls(ui, area, view, rig_frame) {
            return true;
        }
        if self.tool == Tool::Select && !self.playing {
            return self.selection_turn_slider(ui, area, view, rig_frame, selected)
                || self.selection_bone_input(ui, area, view, rig_frame, selected);
        }
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
            if self.rigging.cancel_gesture() {
                self.history(false);
            }
            self.rigging.paint = false;
        }
        if ui.input(|i| i.pointer.primary_pressed())
            && area.contains(screen)
            && ui.ctx().layer_id_at(screen) == Some(ui.layer_id())
        {
            self.rigging.grab = Some(screen);
            self.rigging.gesture_started = false;
            self.rigging.grab_offset = Vec2::ZERO;
            self.rigging.angle_offset = 0.;
            let poses = self.doc.extras.rig.poses(rig_frame);
            if self.tool == Tool::Bones {
                if self.rigging.paint && self.rigging.bone.is_some() {
                    self.checkpoint();
                    self.rigging.drag = Some(Gesture::Paint { last: world });
                } else if self.rigging.create
                    || (!self.rigging.paint
                        && !self.rigging.reanchor
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
                        let pose = &poses[&id];
                        if self
                            .doc
                            .extras
                            .rig
                            .bones
                            .iter()
                            .any(|bone| bone.id == id && bone.ik)
                        {
                            self.rigging.grab_offset = world - pose.end;
                        }
                        self.rigging.angle_offset = (world - pose.start).angle() - pose.angle;
                        self.rigging.drag = if self.rigging.reanchor
                            || ui.input(|i| i.modifiers.shift && !i.modifiers.command)
                        {
                            self.doc
                                .extras
                                .rig
                                .bones
                                .iter()
                                .find(|bone| bone.id == id)
                                .cloned()
                                .map(|original| Gesture::ReanchorBone {
                                    id,
                                    grab: world,
                                    original,
                                })
                        } else if ui.input(|i| {
                            (i.modifiers.alt || i.modifiers.command) && !i.modifiers.shift
                        }) {
                            let bone = self
                                .doc
                                .extras
                                .rig
                                .bones
                                .iter()
                                .find(|bone| bone.id == id)
                                .unwrap();
                            let distance = world.distance(poses[&id].start).max(1.);
                            Some(Gesture::ScaleBone {
                                id,
                                distance,
                                scale: rig::bone_value(bone, rig_frame).scale,
                                stretch: ui
                                    .input(|i| i.modifiers.command)
                                    .then(|| rig::bone_value(bone, rig_frame).stretch),
                            })
                        } else if ui.input(|i| i.modifiers.shift) {
                            let bone = self
                                .doc
                                .extras
                                .rig
                                .bones
                                .iter()
                                .find(|bone| bone.id == id)
                                .unwrap();
                            let key = rig::bone_value(bone, rig_frame);
                            Some(Gesture::MoveBone {
                                id,
                                grab: world,
                                translation: vector::pos(key.translation).to_vec2(),
                                parent_angle: bone
                                    .parent
                                    .and_then(|parent| poses.get(&parent))
                                    .map_or(0., |pose| pose.angle),
                            })
                        } else {
                            Some(Gesture::Bone { id })
                        };
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
                            rig::skin_point(b, &poses, rig::pin_value(p, rig_frame)),
                        );
                        if view.screen(current).distance(screen) < 12. {
                            found = Some((b.target, p.id));
                        }
                    }
                }
                if let Some((target, id)) = found {
                    self.rigging.target = Some(target);
                    self.rigging.pin = Some(id);
                    if let Some(binding) = self
                        .doc
                        .extras
                        .rig
                        .bindings
                        .iter()
                        .find(|b| b.target == target)
                        && let Some(pin) = binding.pins.iter().find(|p| p.id == id)
                    {
                        self.rigging.grab_offset = world
                            - rig::point(
                                self.binding_delta(binding),
                                rig::skin_point(binding, &poses, rig::pin_value(pin, rig_frame)),
                            );
                    }
                    self.rigging.drag = Some(
                        if self.rigging.reanchor || ui.input(|i| i.modifiers.shift) {
                            Gesture::ReanchorPin { target, id }
                        } else {
                            Gesture::Pin { target, id }
                        },
                    );
                } else if !self.rigging.reanchor
                    && let Some((l, m)) = self.rig_hit(world, 4. / view.scale)
                {
                    self.add_puppet_pin(l, m, world);
                }
            }
        }
        if let Some(gesture) = self.rigging.drag.clone() {
            if !matches!(gesture, Gesture::Create { .. } | Gesture::Paint { .. })
                && !self.rigging.gesture_started
            {
                if self
                    .rigging
                    .grab
                    .is_none_or(|grab| grab.distance(screen) < 3.)
                {
                    if !ui.input(|i| i.pointer.primary_down()) {
                        self.rigging.cancel_gesture();
                    }
                    return true;
                }
                let selection = self.canvas.selected.clone();
                self.checkpoint();
                self.canvas.selected = selection;
                self.rigging.gesture_started = true;
            }
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
                        self.doc.extras.rig.solve_ik(
                            id,
                            world - self.rigging.grab_offset,
                            rig_frame,
                        );
                    } else {
                        let poses = self.doc.extras.rig.poses(rig_frame);
                        if let Some(p) = poses.get(&id) {
                            let mut key = rig::bone_value(&bone, rig_frame);
                            let d = world - p.start;
                            if d.length_sq() > 1e-6 {
                                key.angle += crate::animation::shortest(
                                    d.angle() - p.angle - self.rigging.angle_offset,
                                );
                                key.frame = rig_frame;
                                self.doc.extras.rig.set_bone_key(id, key);
                            }
                        }
                    }
                }
                Gesture::ReanchorBone { id, grab, original } => {
                    if !ui.input(|i| i.pointer.primary_down())
                        && let Some(index) = self
                            .doc
                            .extras
                            .rig
                            .bones
                            .iter()
                            .position(|bone| bone.id == id)
                    {
                        self.doc.extras.rig.bones[index] = original.clone();
                        let poses = self.doc.extras.rig.poses(rig_frame);
                        if let Some(pose) = poses.get(&id)
                            && let Some(inverse) = rig::inverse(pose.matrix)
                        {
                            let delta = rig::point(inverse, world) - rig::point(inverse, grab);
                            let mut moved = original;
                            moved.start = vector::xy(vector::pos(moved.start) + delta);
                            moved.end = vector::xy(vector::pos(moved.end) + delta);
                            let mut frames: Vec<_> =
                                moved.keys.iter().map(|key| key.frame).collect();
                            if !frames.contains(&rig_frame) {
                                frames.push(rig_frame);
                            }
                            let mut keys = Vec::with_capacity(frames.len());
                            for frame in frames {
                                let poses = self.doc.extras.rig.poses(frame);
                                let Some(pose) = poses.get(&id) else { continue };
                                let parent = moved.parent.and_then(|parent| poses.get(&parent));
                                let inherited =
                                    parent.map_or(rig::IDENTITY, |parent| parent.matrix);
                                let base = vector::pos(self.doc.extras.rig.bones[index].start);
                                let new_base = base + delta;
                                let compensation = (rig::point(pose.matrix, new_base)
                                    - rig::point(pose.matrix, base))
                                    - (rig::point(inherited, new_base)
                                        - rig::point(inherited, base));
                                let mut key =
                                    rig::bone_value(&self.doc.extras.rig.bones[index], frame);
                                key.frame = frame;
                                key.translation = vector::xy(
                                    vector::pos(key.translation)
                                        + crate::canvas::rotate(
                                            compensation,
                                            -parent.map_or(0., |parent| parent.angle),
                                        ),
                                );
                                keys.push(key);
                            }
                            keys.sort_unstable_by_key(|key| key.frame);
                            moved.keys = keys.into();
                            self.doc.extras.rig.bones[index] = moved;
                        }
                    }
                }
                Gesture::ReanchorPin { target, id } => {
                    let world = world - self.rigging.grab_offset;
                    let rig = &self.doc.extras.rig;
                    let poses = rig.poses(rig_frame);
                    if let Some(index) = rig
                        .bindings
                        .iter()
                        .position(|binding| binding.target == target)
                    {
                        let binding = &rig.bindings[index];
                        // Moving a constraint in an already warped multi-pin mesh
                        // changes the interpolation. Preserve those poses as-is.
                        if binding
                            .pins
                            .iter()
                            .any(|pin| pin.keys.iter().any(|key| key.position != pin.rest))
                        {
                            self.status = tr("Move pin anchors before posing them.").into();
                        } else {
                            let deform = Deformer::new(binding, &poses, rig_frame);
                            let local = rig::point(
                                rig::inverse(self.binding_delta(binding)).unwrap_or(rig::IDENTITY),
                                world,
                            );
                            let position = vector::xy(deform.inverse_skin(local));
                            if let Some(pin) = self.doc.extras.rig.bindings[index]
                                .pins
                                .iter_mut()
                                .find(|pin| pin.id == id)
                            {
                                pin.rest = position;
                                for key in &mut pin.keys {
                                    key.position = position;
                                }
                            }
                        }
                    }
                }
                Gesture::ScaleBone {
                    id,
                    distance,
                    scale,
                    stretch,
                } => {
                    let poses = self.doc.extras.rig.poses(rig_frame);
                    if let Some(bone) = self.doc.extras.rig.bones.iter().find(|bone| bone.id == id)
                        && let Some(pose) = poses.get(&id)
                    {
                        let mut key = rig::bone_value(bone, rig_frame);
                        key.frame = rig_frame;
                        let ratio = world.distance(pose.start) / distance;
                        if let Some(stretch) = stretch {
                            key.stretch = (stretch * ratio).clamp(0.1, 10.);
                        } else {
                            key.scale = (scale * ratio).clamp(0.01, 100.);
                        }
                        self.doc.extras.rig.set_bone_key(id, key);
                    }
                }
                Gesture::MoveBone {
                    id,
                    grab,
                    translation,
                    parent_angle,
                } => {
                    if let Some(bone) = self.doc.extras.rig.bones.iter().find(|bone| bone.id == id)
                    {
                        let mut key = rig::bone_value(bone, rig_frame);
                        let moved =
                            translation + crate::canvas::rotate(world - grab, -parent_angle);
                        key.translation = vector::xy(moved.to_pos2());
                        key.frame = rig_frame;
                        self.doc.extras.rig.set_bone_key(id, key);
                    }
                }
                Gesture::Pin { target, id } => {
                    let world = world - self.rigging.grab_offset;
                    let poses = self.doc.extras.rig.poses(rig_frame);
                    if let Some(b) = self
                        .doc
                        .extras
                        .rig
                        .bindings
                        .iter()
                        .find(|b| b.target == target)
                    {
                        let deform = Deformer::new(b, &poses, rig_frame);
                        let local = rig::point(
                            rig::inverse(self.binding_delta(b)).unwrap_or(rig::IDENTITY),
                            world,
                        );
                        let position = vector::xy(deform.inverse_skin(local));
                        let curve = b
                            .pins
                            .iter()
                            .find(|p| p.id == id)
                            .and_then(|p| p.keys.iter().rev().find(|k| k.frame <= rig_frame))
                            .map(|k| k.curve.clone())
                            .unwrap_or_default();
                        self.doc.extras.rig.set_pin_key(
                            target,
                            id,
                            PinKey {
                                frame: rig_frame,
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
                                    .poses(rig_frame)
                                    .get(&id)
                                    .and_then(|p| rig::inverse(p.matrix))
                            })
                            .unwrap_or(rig::IDENTITY);
                        self.add_bone(rig::point(inv, start), rig::point(inv, end), parent);
                    }
                } else {
                    self.rigging.drag = None;
                    self.rigging.gesture_started = false;
                    self.rigging.grab = None;
                    self.recover();
                }
            }
            ui.ctx().request_repaint();
        }
        true
    }
    pub fn rig_overlays(&self, p: &egui::Painter, view: View, selected: &[(usize, usize)]) {
        let rig_frame = self.group_rig_frame();
        if self.playing || !matches!(self.tool, Tool::Bones | Tool::Puppet | Tool::Select) {
            return;
        }
        let rig = &self.doc.extras.rig;
        let mut visible = std::collections::HashSet::new();
        let selecting = self.tool == Tool::Select;
        if selecting {
            visible = self.selected_bone_ids(selected);
            if visible.is_empty() {
                for &(layer, index) in selected {
                    let Some(layer) = self.doc.layers.get(layer) else {
                        continue;
                    };
                    let marks = crate::animation::marks(layer, self.frame);
                    let Some(instance) = marks
                        .get(index)
                        .and_then(|mark| mark.style.instance.as_ref())
                    else {
                        continue;
                    };
                    let Some(symbol) = self
                        .doc
                        .extras
                        .symbols
                        .iter()
                        .find(|symbol| symbol.id == instance.symbol)
                    else {
                        continue;
                    };
                    if symbol.rig.bones.is_empty() {
                        continue;
                    }
                    let Some(layer) = symbol.layers.first() else {
                        continue;
                    };
                    let frame = crate::features::symbol_frame(
                        instance,
                        layer.frames.len(),
                        self.frame,
                        self.animation.clock,
                    );
                    let poses = crate::character::poses(&self.doc, &symbol.rig, frame);
                    let color = Color32::from_rgb(86, 211, 214);
                    for pose in poses.values() {
                        let a = view.screen(rig::point(instance.matrix, pose.start));
                        let b = view.screen(rig::point(instance.matrix, pose.end));
                        p.line_segment([a, b], Stroke::new(2., color));
                        p.circle(a, 4., Color32::from_gray(30), Stroke::new(1.5, color));
                        p.circle(b, 4., Color32::from_gray(30), Stroke::new(1.5, color));
                    }
                }
                return;
            }
        }
        let poses = crate::character::poses(&self.doc, rig, rig_frame);
        if !selecting && (self.rigging.paint || self.rigging.show_weights) {
            if let Some(id) = self.rigging.bone {
                self.weight_overlay(p, view, id, &poses);
            }
        }
        if let Some(Gesture::ReanchorBone { id, grab, .. }) = &self.rigging.drag
            && let Some(pose) = poses.get(id)
            && let Some(pointer) = p.ctx().input(|i| i.pointer.interact_pos())
        {
            let offset = pointer - view.screen(*grab);
            let start = view.screen(pose.start) + offset;
            let end = view.screen(pose.end) + offset;
            let color = Color32::from_rgb(255, 197, 86);
            p.line_segment([start, end], Stroke::new(2., color));
            p.circle_stroke(start, 6., Stroke::new(2., color));
            p.circle_stroke(end, 4., Stroke::new(1., color));
        }
        if self.tool == Tool::Bones || selecting {
            for b in &self.doc.extras.rig.bones {
                if selecting && !visible.contains(&b.id) {
                    continue;
                }
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
                if !selecting
                    && !self.rigging.paint
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
                        rig::skin_point(b, &poses, rig::pin_value(pin, rig_frame)),
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
        let rig_frame = self.group_rig_frame();
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
            let deformer = Deformer::new(binding, poses, rig_frame);
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
                            if self.rigging.show_unweighted {
                                (1. - binding
                                    .sample(q)
                                    .iter()
                                    .map(|influence| influence.weight)
                                    .sum::<f32>())
                                .clamp(0., 1.)
                            } else {
                                binding.weight(q, bone.id)
                            },
                        )
                    });
                }
            }
        }
    }
    pub fn rig_properties(&mut self, ui: &mut egui::Ui) {
        let rig_frame = self.group_rig_frame();
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
                self.rigging.reanchor = false;
            }
            if design::button(
                ui,
                Icon::Rotate,
                "Posar",
                "Shift: move anchor · Ctrl/Cmd+Shift: move pose · Alt: scale · Ctrl/Cmd: stretch",
                !self.rigging.create && !self.rigging.paint && !self.rigging.reanchor,
                true,
            )
            .clicked()
            {
                self.rigging.create = false;
                self.rigging.paint = false;
                self.rigging.reanchor = false;
            }
        });
        self.rig_anchor_mode_button(ui);
        if self.feature.symbol_stack.is_empty() && !self.doc.extras.rig.bones.is_empty() {
            if design::button(
                ui,
                Icon::Head,
                "Set Up Character",
                "",
                self.rigging.character_setup,
                true,
            )
            .clicked()
            {
                self.open_character_symbol();
            }
            if design::button(
                ui,
                Icon::Rotate,
                "Turn controls",
                "",
                self.rigging.character_setup,
                true,
            )
            .clicked()
            {
                self.rigging.character_setup ^= true;
            }
            self.character_setup(ui.ctx(), rig_frame);
            self.face_controls(ui);
        }
        design::category(ui, "JERARQUÍA");
        let bones = self.doc.extras.rig.bones.clone();
        let targets = self.rig_targets();
        let local_tracks: std::collections::HashSet<_> = targets
            .iter()
            .filter(|target| target.layer == self.layer)
            .map(|target| target.track)
            .collect();
        let shown: Vec<_> = bones
            .iter()
            .filter(|bone| {
                bone.targets.is_empty()
                    || bone
                        .targets
                        .iter()
                        .any(|track| local_tracks.contains(track))
            })
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("bone-tree")
            .max_height(170.)
            .show(ui, |ui| {
                for b in &shown {
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
        if !shown.is_empty() {
            egui::CollapsingHeader::new(tr("Rename visible bones")).show(ui, |ui| {
                let id = ui.scope_id().with("rig-name-prefix");
                let mut prefix = ui
                    .data(|data| data.get_temp::<String>(id))
                    .unwrap_or_default();
                ui.add(
                    egui::TextEdit::singleline(&mut prefix)
                        .char_limit(48)
                        .hint_text(tr("Bone name prefix")),
                );
                if design::button(
                    ui,
                    Icon::Rename,
                    "Rename visible bones",
                    "",
                    false,
                    !prefix.trim().is_empty(),
                )
                .clicked()
                {
                    self.checkpoint();
                    for (index, visible) in shown.iter().enumerate() {
                        if let Some(bone) = self
                            .doc
                            .extras
                            .rig
                            .bones
                            .iter_mut()
                            .find(|b| b.id == visible.id)
                        {
                            bone.name = format!("{} {:02}", prefix.trim(), index + 1);
                        }
                    }
                    self.recover();
                }
                ui.data_mut(|data| data.insert_temp(id, prefix));
            });
        }
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
            self.begin_property_edit(("bone-settings", bone.id));
            self.doc.extras.rig.bones[index] = bone.clone();
            self.recover();
        }
        let mut rule = self
            .doc
            .extras
            .rig
            .ik_rules
            .get(&bone.id)
            .copied()
            .unwrap_or_default();
        let before = rule;
        ui.collapsing(tr("IK constraints"), |ui| {
            ui.checkbox(&mut rule.stop_chain, tr("Stop IK chain at this bone"));
            let mut limited = rule.limits.is_some();
            if ui.checkbox(&mut limited, tr("Limit joint angle")).changed() {
                rule.limits = limited.then_some([-90., 90.]);
            }
            if let Some([min, max]) = &mut rule.limits {
                ui.horizontal(|ui| {
                    ui.label(tr("Min / Max"));
                    ui.add(egui::DragValue::new(min).range(-360. ..=*max).suffix("°"));
                    ui.add(egui::DragValue::new(max).range(*min..=360.).suffix("°"));
                });
            }
        });
        if rule != before {
            self.begin_property_edit(("bone-limits", bone.id));
            if rule == Default::default() {
                self.doc.extras.rig.ik_rules.remove(&bone.id);
            } else {
                self.doc.extras.rig.ik_rules.insert(bone.id, rule);
            }
            self.recover();
        }
        design::category(ui, "SÍMBOLOS AFECTADOS");
        let unbound: Vec<_> = targets
            .iter()
            .filter(|target| {
                target.layer == self.layer
                    && self.canvas.selected.contains(&(target.layer, target.mark))
                    && !bone.targets.contains(&target.track)
            })
            .map(|target| (target.layer, target.mark))
            .collect();
        if design::button(
            ui,
            Icon::AutoWeights,
            "Bind selected symbols",
            "",
            false,
            !unbound.is_empty(),
        )
        .clicked()
        {
            let selection = self.canvas.selected.clone();
            self.checkpoint();
            for (layer, mark) in unbound {
                if let Some(track) = self.rig_bind(layer, mark) {
                    self.doc.extras.rig.bones[index].targets.push(track);
                    self.doc.extras.rig.auto_weights(track);
                }
            }
            self.canvas.selected = selection;
            self.recover();
            return;
        }
        if design::button(
            ui,
            Icon::Bones,
            "Bind Rigid to Bone",
            "Use only this bone for the selected pieces",
            false,
            self.canvas.selected.iter().any(|&(layer, mark)| {
                targets
                    .iter()
                    .any(|target| target.layer == layer && target.mark == mark)
            }),
        )
        .clicked()
        {
            let selected = self.canvas.selected.clone();
            self.checkpoint();
            for &(layer, mark) in &selected {
                if let Some(track) = self.rig_bind(layer, mark) {
                    for other in &mut self.doc.extras.rig.bones {
                        other.targets.retain(|target| *target != track);
                    }
                    self.doc.extras.rig.bones[index].targets.push(track);
                    if let Some(binding) = self
                        .doc
                        .extras
                        .rig
                        .bindings
                        .iter_mut()
                        .find(|binding| binding.target == track)
                    {
                        let mut cell = crate::rig::Cell::default();
                        cell.set(bone.id, 1.);
                        binding.cells.fill(cell);
                    }
                }
            }
            self.canvas.selected = selected;
            self.recover();
            return;
        }
        if design::button(
            ui,
            Icon::LayerRemove,
            "Remove Selected Binding",
            "Remove only the selected pieces’ weights and pins",
            false,
            self.canvas.selected.iter().any(|&(layer, mark)| {
                targets.iter().any(|target| {
                    target.layer == layer
                        && target.mark == mark
                        && self
                            .doc
                            .extras
                            .rig
                            .bindings
                            .iter()
                            .any(|binding| binding.target == target.track)
                })
            }),
        )
        .clicked()
        {
            let selected = self.canvas.selected.clone();
            let detached: std::collections::HashSet<_> = targets
                .iter()
                .filter(|target| selected.contains(&(target.layer, target.mark)))
                .map(|target| target.track)
                .collect();
            self.checkpoint();
            let rig = &mut self.doc.extras.rig;
            rig.bindings
                .retain(|binding| !detached.contains(&binding.target));
            for bone in &mut rig.bones {
                bone.targets.retain(|target| !detached.contains(target));
            }
            self.rigging.target = None;
            self.rigging.pin = None;
            self.canvas.selected = selected;
            self.recover();
            return;
        }
        let mut attachment = None;
        for t in targets
            .into_iter()
            .filter(|target| target.layer == self.layer)
        {
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
        let mut locked = self.doc.extras.rig.weight_locks.contains(&bone.id);
        if ui
            .checkbox(&mut locked, tr("Lock weights for this bone"))
            .changed()
        {
            self.checkpoint();
            if locked {
                self.doc.extras.rig.weight_locks.insert(bone.id);
            } else {
                self.doc.extras.rig.weight_locks.remove(&bone.id);
            }
            self.recover();
        }
        if self.rigging.show_weights {
            ui.checkbox(
                &mut self.rigging.show_unweighted,
                tr("Show unweighted areas"),
            );
        }
        if design::button(
            ui,
            Icon::Eye,
            "Show bone weights",
            "",
            self.rigging.show_weights,
            !bone.targets.is_empty(),
        )
        .clicked()
        {
            self.rigging.show_weights = !self.rigging.show_weights;
        }

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
            self.rigging.reanchor = false;
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
        let mut value = rig::bone_value(&bone, rig_frame);
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
        pose_changed |= ui
            .add(
                egui::Slider::new(&mut value.stretch, 0.1..=10.)
                    .logarithmic(true)
                    .text(tr("Stretch / squash")),
            )
            .changed();
        ui.horizontal(|ui| {
            ui.label(tr("Posición"));
            for (coordinate, axis) in value.translation.iter_mut().zip(["X ", "Y "]) {
                pose_changed |= ui
                    .add(
                        egui::DragValue::new(coordinate)
                            .prefix(axis)
                            .range(-100000.0..=100000.0),
                    )
                    .changed();
            }
        });
        if pose_changed {
            self.begin_property_edit(("bone-pose", bone.id, rig_frame));
            value.frame = rig_frame;
            self.doc.extras.rig.set_bone_key(bone.id, value);
            self.recover();
        }
        self.rig_animation_buttons(ui);
        let current = self.doc.extras.rig.bones[index]
            .keys
            .iter()
            .rposition(|k| k.frame <= rig_frame)
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
    fn rig_anchor_mode_button(&mut self, ui: &mut egui::Ui) {
        if design::button(
            ui,
            Icon::Transform,
            "Edit anchors",
            "Move anchors without posing the artwork",
            self.rigging.reanchor,
            true,
        )
        .clicked()
        {
            self.rigging.reanchor = !self.rigging.reanchor;
            self.rigging.create = false;
            self.rigging.paint = false;
        }
    }

    fn puppet_properties(&mut self, ui: &mut egui::Ui) {
        self.rig_anchor_mode_button(ui);
        let rig_frame = self.group_rig_frame();
        design::category(ui, "PUPPET VECTORIAL");
        ui.small(tr(
            "Clic en un símbolo añade un pin. Arrastra sus pines para deformar sin rasterizar.",
        ));
        let mut select = None;
        let targets = self.rig_targets();
        for binding in &self.doc.extras.rig.bindings {
            let Some(target) = targets
                .iter()
                .find(|target| target.layer == self.layer && target.track == binding.target)
            else {
                continue;
            };
            if binding.pins.is_empty() {
                continue;
            }
            ui.small(format!("{} · {}", target.name, target.mark + 1));
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
            self.begin_property_edit(("pin-name", target, pin.id));
            self.doc.extras.rig.bindings[bi].pins[pi].name = name;
            self.recover();
        }
        design::category(ui, "POSICIÓN DEL PIN");
        let position = rig::pin_value(&pin, rig_frame);
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
            self.begin_property_edit(("pin-position", target, pin.id, rig_frame));
            self.doc.extras.rig.set_pin_key(
                target,
                pin.id,
                PinKey {
                    frame: rig_frame,
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
            .rposition(|k| k.frame <= rig_frame)
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
        let rig_frame = self.group_rig_frame();
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
                rig_frame > 0,
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
        let rig_frame = self.group_rig_frame();
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
                if action == 4 && rig_frame > 0 {
                    if let Some(b) = self.doc.extras.rig.bones.iter_mut().find(|b| b.id == id) {
                        b.keys.retain(|k| k.frame != rig_frame);
                    }
                } else if let Some(b) = self.doc.extras.rig.bones.iter().find(|b| b.id == id) {
                    let mut key = rig::bone_value(b, rig_frame);
                    if action == 1 {
                        key = BoneKey::default();
                    }
                    key.frame = rig_frame;
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
                if action == 4 && rig_frame > 0 {
                    if let Some(p) = self
                        .doc
                        .extras
                        .rig
                        .bindings
                        .iter_mut()
                        .find(|b| b.target == target)
                        .and_then(|b| b.pins.iter_mut().find(|p| p.id == id))
                    {
                        p.keys.retain(|k| k.frame != rig_frame);
                    }
                } else {
                    let position = if action == 1 {
                        pin.rest
                    } else {
                        vector::xy(rig::pin_value(&pin, rig_frame))
                    };
                    self.doc.extras.rig.set_pin_key(
                        target,
                        id,
                        PinKey {
                            frame: rig_frame,
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

impl Editor {
    pub(crate) fn delete_rigging_track(&mut self) -> bool {
        if self.doc.extras.rig.is_empty() {
            return false;
        }
        self.finish_rig_gesture();
        self.checkpoint();
        self.doc.extras.rig = Default::default();
        self.rigging = Default::default();
        self.feature.frame_selection = None;
        self.canvas.clear_selection();
        self.recover();
        true
    }
}
