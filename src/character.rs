use crate::{
    Editor, Mark,
    design::{Icon, button},
    document::Layer,
    features::{LayerOptions, Symbol},
    i18n::tr,
    rig::{self, Matrix},
    shared::SharedVec,
};
use eframe::egui;
use serde::{Deserialize, Serialize};

const VARIANTS: usize = 16;
pub(crate) const FACE_FRAMES: usize = 8 * (1 + VARIANTS);
pub(crate) const MAX_FACE_FRAMES: usize = 8 * (1 + 128);
const EYES: &[&str] = &[
    "Open",
    "Half closed",
    "Closed",
    "Look left",
    "Look right",
    "Look up",
    "Look down",
];
const BROWS: &[&str] = &["Neutral", "Raised", "Lowered", "Tilt in", "Tilt out"];
const BROW_POINTS: [&str; 3] = ["Left point", "Center point", "Right point"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
enum FacePart {
    #[default]
    None,
    Mouth,
    Eye,
    Brow,
    Pupil,
    Hand,
    Slider,
    Toggle,
    Switch,
}

const HAND_VIEWS: [&str; 8] = [
    "Palm",
    "Palm ¾ R",
    "Edge R",
    "Back ¾ R",
    "Back",
    "Back ¾ L",
    "Edge L",
    "Palm ¾ L",
];

impl FacePart {
    fn control(self) -> bool {
        matches!(
            self,
            Self::Hand | Self::Slider | Self::Toggle | Self::Switch
        )
    }
    fn control_poses(self) -> &'static [&'static str] {
        match self {
            Self::Hand => &["Relaxed", "Open", "Fist", "Point", "Grip"],
            Self::Slider => &["Start", "Middle", "End"],
            Self::Toggle => &["On", "Off"],
            Self::Switch => &["Neutral", "Smile", "Frown", "Surprise"],
            _ => &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FaceKey {
    frame: usize,
    variant: usize,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct FaceMotion {
    frame: usize,
    axes: [f32; 3],
    #[serde(default, skip_serializing_if = "RigEase::is_linear")]
    ease: RigEase,
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub(crate) enum RigEase {
    #[default]
    Linear,
    In,
    Out,
    InOut,
}

impl RigEase {
    fn is_linear(&self) -> bool {
        *self == Self::Linear
    }
    fn value(self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::In => t * t,
            Self::Out => t * (2. - t),
            Self::InOut => t * t * (3. - 2. * t),
        }
    }
}

pub(crate) const VIEWS: [&str; 8] = [
    "Front",
    "Front ¾ R",
    "Right",
    "Back ¾ R",
    "Back",
    "Back ¾ L",
    "Left",
    "Front ¾ L",
];

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Piece {
    pub track: u64,
    pub matrix: Matrix,
    #[serde(default)]
    hidden: [bool; 8],
    #[serde(default)]
    order: [i16; 8],
    #[serde(default)]
    joints: [Vec<(u64, [[f32; 2]; 2])>; 8],
    #[serde(default)]
    face: FacePart,
    #[serde(default)]
    face_keys: SharedVec<FaceKey>,
    #[serde(default, skip_serializing_if = "is_zero")]
    shape_transitions: u8,
    #[serde(default, skip_serializing_if = "SharedVec::is_empty")]
    face_motion: SharedVec<FaceMotion>,
    #[serde(default)]
    gaze_limits: [Option<[f32; 4]>; 8],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    eye_mask: Option<u64>,
    #[serde(default)]
    pupil_area: [Option<[[f32; 2]; 2]>; 8],
    #[serde(default, skip_serializing_if = "is_zero")]
    circular_pupil_areas: u8,
    #[serde(default)]
    brow_points: [[Option<[f32; 2]>; 3]; 8],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    eye_parent: Option<u64>,
    #[serde(default)]
    vertical_tweens: [u8; 8],
    #[serde(default, skip_serializing_if = "SharedVec::is_empty")]
    pub(crate) lip_poses: SharedVec<crate::lip::Pose>,
    #[serde(default, skip_serializing_if = "SharedVec::is_empty")]
    lip_tweens: SharedVec<u8>,
    #[serde(default, skip_serializing_if = "SharedVec::is_empty")]
    wrist_points: SharedVec<Option<[f32; 2]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    wrist_origin: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    tween_hand_poses: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    renamed_bone: Option<u64>,
}

fn opposite_view(view: usize) -> usize {
    (8 - view) % 8
}

fn is_zero(value: &u8) -> bool {
    *value == 0
}

fn brow_axes_control(ui: &mut egui::Ui, mut axes: [f32; 3]) -> Option<([f32; 3], bool)> {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(90.), 62.),
        egui::Sense::hover(),
    );
    let rest = |axis: usize| {
        egui::pos2(
            egui::lerp(rect.left() + 17. ..=rect.right() - 17., axis as f32 / 2.),
            rect.center().y + if axis == 1 { -7. } else { 5. },
        )
    };
    let mut changed = None;
    let mut handles = [None; 3];
    for (axis, value) in axes.iter_mut().enumerate() {
        let anchor = rest(axis);
        let center = anchor + egui::vec2(0., *value * 16.);
        let response = ui
            .interact(
                egui::Rect::from_center_size(center, egui::Vec2::splat(21.)),
                ui.scope_id().with(("brow-control", axis)),
                egui::Sense::drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeVertical)
            .on_hover_text(tr(BROW_POINTS[axis]));
        let y = (*value + response.drag_delta().y / 16.).clamp(-1., 1.);
        if *value != y {
            *value = y;
            changed = Some(response.drag_started());
        }
        handles[axis] = Some((
            anchor + egui::vec2(0., *value * 16.),
            response.hovered() || response.dragged(),
        ));
    }
    let [left, center, right] = handles.map(|handle| handle.unwrap().0);
    let paint = ui.painter();
    let span = (right.x - left.x) * 0.16;
    let arc = |a, b, c, d, width| {
        egui::epaint::CubicBezierShape::from_points_stroke(
            [a, b, c, d],
            false,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(width, crate::themes::muted()),
        )
    };
    paint.add(arc(
        left,
        left + egui::vec2(span, -6.),
        center - egui::vec2(span, 0.),
        center,
        5.,
    ));
    paint.add(arc(
        center,
        center + egui::vec2(span, 0.),
        right - egui::vec2(span, 6.),
        right,
        3.,
    ));
    for (point, active) in handles.into_iter().flatten() {
        paint.circle_filled(point, 5., crate::themes::accent());
        if active {
            paint.circle_stroke(point, 7., egui::Stroke::new(1., crate::themes::accent()));
        }
    }
    changed.map(|started| (axes, started))
}

fn eye_closure_slider(ui: &mut egui::Ui, value: &mut f32) -> egui::Response {
    rig_slider(ui, value, 3, "Eye closure")
}

fn rig_slider(ui: &mut egui::Ui, value: &mut f32, ticks: usize, label: &str) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 25.),
        egui::Sense::click_and_drag(),
    );
    let start = rect.left() + 10.;
    let end = rect.right() - 10.;
    let mut next = *value;
    if let Some(pointer) = response.interact_pointer_pos()
        && (response.clicked() || response.dragged() || response.drag_stopped())
    {
        next = ((pointer.x - start) / (end - start).max(1.)).clamp(0., 1.);
        response.request_focus();
    }
    if response.has_focus() {
        next += ui.input(|i| {
            i.num_presses(egui::Key::ArrowRight) as f32 - i.num_presses(egui::Key::ArrowLeft) as f32
        }) * 0.05;
    }
    next = next.clamp(0., 1.);
    if next != *value {
        *value = next;
        response.mark_changed();
    }
    let at = |t: f32| egui::pos2(egui::lerp(start..=end, t), rect.center().y);
    let accent = crate::themes::accent();
    let painter = ui.painter();
    painter.line_segment([at(0.), at(1.)], egui::Stroke::new(1.5, accent));
    for tick in 0..ticks {
        painter.circle_filled(
            at(tick as f32 / ticks.saturating_sub(1).max(1) as f32),
            3.,
            accent,
        );
    }
    painter.circle_filled(at(*value), 6., accent);
    response
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
        .on_hover_text(format!("{} {:.0}%", tr(label), *value * 100.))
}

fn copy_wrist_point(layer: &mut Layer, destination: usize, source: usize) {
    if let Some(piece) = layer
        .options
        .character_piece
        .as_mut()
        .filter(|piece| piece.face == FacePart::Hand)
    {
        let point = piece.wrist_points.get(source).copied().flatten();
        if piece.wrist_points.len() <= destination && point.is_some() {
            piece.wrist_points.resize(destination + 1, None);
        }
        if let Some(destination) = piece.wrist_points.get_mut(destination) {
            *destination = point;
        }
    }
}

fn pose_thumbnail(
    ui: &mut egui::Ui,
    drawing: impl FnOnce() -> SharedVec<Mark>,
    name: &str,
    selected: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(62., 55.), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return false;
    }
    let marks = drawing();
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 4., ui.visuals().widgets.inactive.bg_fill);
    if selected || response.hovered() {
        painter.rect_stroke(
            rect,
            4.,
            egui::Stroke::new(1.5, crate::themes::accent()),
            egui::StrokeKind::Inside,
        );
    }
    let bounds = marks.iter().fold(egui::Rect::NOTHING, |bounds, mark| {
        bounds.union(crate::vector::control_bounds(mark))
    });
    if bounds.is_finite() {
        let view = crate::canvas::View {
            center: rect.center() - egui::vec2(0., 6.),
            stage_center: bounds.center(),
            scale: (46. / bounds.width().max(1.)).min(31. / bounds.height().max(1.)),
            angle: 0.,
        };
        for mark in &marks {
            crate::paint::draw(&painter, mark, view, false, false, None);
        }
    }
    painter.text(
        rect.center_bottom() - egui::vec2(0., 4.),
        egui::Align2::CENTER_BOTTOM,
        name,
        egui::FontId::proportional(10.),
        ui.visuals().text_color(),
    );
    response.on_hover_text(name).clicked()
}

fn drawn_transition(
    turn: f32,
    enabled: u8,
    drawn: impl Fn(usize) -> bool,
) -> Option<(usize, usize, f32)> {
    let floor = turn.floor() as i32;
    let slot = |step: i32| step.rem_euclid(8) as usize;
    let a = (floor - 7..=floor).rev().find(|&step| drawn(slot(step)))?;
    let b = (floor + 1..a + 8).find(|&step| drawn(slot(step)))?;
    let blend = (turn - a as f32) / (b - a) as f32;
    (blend > 0. && enabled & (1 << slot(a)) != 0).then_some((slot(a), slot(b), blend))
}

fn character_row_icon(ui: &mut egui::Ui, icon: Icon, name: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(15.), egui::Sense::hover());
        crate::design::draw(ui.painter(), rect, icon, crate::themes::muted());
        ui.add(egui::Label::new(name).sense(egui::Sense::click()))
    })
    .inner
}

fn shape_transition<'a>(
    layer: &'a Layer,
    piece: &Piece,
    offset: usize,
    from: usize,
) -> Option<(&'a [Mark], &'a [Mark])> {
    let to = (from + 1) % VIEWS.len();
    if piece.hidden[from] || piece.hidden[to] {
        return None;
    }
    let first = layer.frames.get(offset + from)?;
    let last = layer.frames.get(offset + to)?;
    crate::animation::shape_pair(first, last).then_some((first, last))
}

impl Piece {
    fn pupil_range(&self, slot: usize) -> Option<(egui::Pos2, egui::Vec2)> {
        let [a, b] = self.pupil_area[slot]?.map(crate::vector::pos);
        if self.circular_pupil_areas & (1 << slot) != 0 {
            Some((a, egui::Vec2::splat(a.distance(b))))
        } else {
            let rect = egui::Rect::from_two_pos(a, b);
            Some((rect.center(), rect.size() * 0.5))
        }
    }

    fn motion(&self, frame: usize) -> [f32; 3] {
        let index = self.face_motion.partition_point(|key| key.frame <= frame);
        let Some(a) = index.checked_sub(1).and_then(|i| self.face_motion.get(i)) else {
            return [0.; 3];
        };
        self.face_motion.get(index).map_or(a.axes, |b| {
            let t = a
                .ease
                .value((frame - a.frame) as f32 / (b.frame - a.frame) as f32);
            std::array::from_fn(|axis| a.axes[axis] + (b.axes[axis] - a.axes[axis]) * t)
        })
    }

    fn set_motion(&mut self, frame: usize, axes: [f32; 3]) {
        if frame != 0 && self.face_motion.is_empty() {
            self.face_motion.push(FaceMotion {
                frame: 0,
                axes: [0.; 3],
                ease: Default::default(),
            });
        }
        match self
            .face_motion
            .binary_search_by_key(&frame, |key| key.frame)
        {
            Ok(index) => self.face_motion[index].axes = axes,
            Err(index) => self.face_motion.insert(
                index,
                FaceMotion {
                    frame,
                    axes,
                    ease: Default::default(),
                },
            ),
        }
    }
    fn variant(&self, frame: usize) -> usize {
        let index = self.face_keys.partition_point(|key| key.frame <= frame);
        index
            .checked_sub(1)
            .map_or(0, |index| self.face_keys[index].variant)
    }

    fn set_variant(&mut self, frame: usize, variant: usize) {
        match self.face_keys.binary_search_by_key(&frame, |key| key.frame) {
            Ok(index) => self.face_keys[index].variant = variant,
            Err(index) => self.face_keys.insert(index, FaceKey { frame, variant }),
        }
    }

    pub(crate) fn is_lip(&self) -> bool {
        self.face == FacePart::Mouth && !self.lip_poses.is_empty()
    }

    pub(crate) fn valid(&self) -> bool {
        self.track != 0
            && self.lip_poses.len() <= 128
            && self.lip_poses.iter().all(|pose| pose.valid())
            && self.lip_tweens.len() == self.lip_poses.len()
            && self.wrist_points.len() <= MAX_FACE_FRAMES
            && self
                .wrist_points
                .iter()
                .flatten()
                .flatten()
                .chain(self.wrist_origin.iter().flatten())
                .all(|value| value.is_finite() && value.abs() <= 1e7)
            && self
                .pupil_area
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .chain(self.brow_points.iter().flatten().flatten().flatten())
                .all(|value| value.is_finite() && value.abs() <= 1e7)
            && self.face_motion.len() <= 10000
            && self
                .face_motion
                .windows(2)
                .all(|keys| keys[0].frame < keys[1].frame)
            && self.face_motion.iter().all(|key| {
                key.frame < 10000 && key.axes.iter().all(|value| (-1. ..=1.).contains(value))
            })
            && self
                .gaze_limits
                .iter()
                .flatten()
                .flatten()
                .all(|value| (0. ..=10000.).contains(value))
            && self.face_keys.len() <= 10000
            && self
                .face_keys
                .windows(2)
                .all(|keys| keys[0].frame < keys[1].frame)
            && self.face_keys.iter().all(|key| {
                key.frame < 10000
                    && key.variant
                        < if self.face.control() {
                            self.lip_poses.len()
                        } else {
                            VARIANTS
                        }
            })
            && rig::inverse(self.matrix).is_some()
            && self.matrix.iter().all(|value| value.is_finite())
            && self.joints.iter().all(|view| {
                view.len() <= 128
                    && view.iter().all(|(id, points)| {
                        *id != 0
                            && points
                                .iter()
                                .flatten()
                                .all(|value| value.is_finite() && value.abs() <= 1e7)
                    })
            })
    }

    pub(crate) fn remap(
        &mut self,
        mut track: impl FnMut(u64) -> u64,
        mut bone: impl FnMut(u64) -> u64,
    ) {
        self.eye_mask = self.eye_mask.map(&mut track);
        self.eye_parent = self.eye_parent.map(&mut track);
        self.renamed_bone = self.renamed_bone.map(&mut bone);
        self.track = track(self.track);
        for (id, _) in self.joints.iter_mut().flatten() {
            *id = bone(*id);
        }
    }
}

pub(crate) fn view_joints(
    doc: &crate::Document,
    rig: &rig::Rig,
    frame: usize,
) -> Vec<(u64, [[f32; 2]; 2])> {
    let Some(symbol) = doc
        .extras
        .symbols
        .iter()
        .find(|s| Some(s.id) == rig.character_symbol)
    else {
        return Vec::new();
    };
    let Some(settings) = symbol
        .layers
        .first()
        .and_then(|l| l.options.character_piece.as_ref())
    else {
        return Vec::new();
    };
    let body = rig.character_turns.body.value(frame);
    let head = rig.character_turns.head.value(frame);
    let head_branch = rig
        .character_turns
        .head
        .root
        .map_or_else(Vec::new, |root| rig.pose_branch(root, None));
    rig.bones
        .iter()
        .filter_map(|bone| {
            let angle = if head_branch.contains(&bone.id) {
                body + head
            } else {
                body
            };
            let a = angle.floor();
            let value = angle - a;
            let at = |slot: i32| {
                settings.joints[slot.rem_euclid(8) as usize]
                    .iter()
                    .find(|(id, _)| *id == bone.id)
                    .map_or([bone.start, bone.end], |(_, joints)| *joints)
            };
            let left = at(a as i32);
            let right = at(a as i32 + 1);
            let joints = std::array::from_fn(|end| {
                std::array::from_fn(|axis| {
                    left[end][axis] * (1. - value) + right[end][axis] * value
                })
            });
            (joints != [bone.start, bone.end]).then_some((bone.id, joints))
        })
        .collect()
}

pub(crate) fn poses(
    doc: &crate::Document,
    rig: &rig::Rig,
    frame: usize,
) -> std::collections::HashMap<u64, rig::Pose> {
    let joints = view_joints(doc, rig, frame);
    if joints.is_empty() {
        return rig.poses(frame);
    }
    let mut rig = rig.clone();
    for bone in &mut rig.bones {
        if let Some((_, [start, end])) = joints.iter().find(|(id, _)| *id == bone.id)
            && crate::vector::pos(*start).distance(crate::vector::pos(*end)) > 0.01
        {
            bone.start = *start;
            bone.end = *end;
        }
    }
    rig.poses(frame)
}

pub(crate) fn artwork(
    doc: &crate::Document,
    track: u64,
    frame: usize,
    mouth_pose: Option<usize>,
) -> Option<(SharedVec<Layer>, usize)> {
    let rig = &doc.extras.rig;
    if track == 0 {
        return None;
    }
    let turn = rig.character_turn(track, frame)?;
    let view = (turn.round() as i32).rem_euclid(8) as usize;
    let symbol = doc
        .extras
        .symbols
        .iter()
        .find(|symbol| Some(symbol.id) == rig.character_symbol)?;
    let layer = symbol.layers.iter().find(|layer| {
        layer
            .options
            .character_piece
            .as_ref()
            .is_some_and(|p| p.track == track)
    })?;
    let piece = layer.options.character_piece.as_ref()?;
    if piece.hidden[view] {
        return Some((Default::default(), 0));
    }
    let inverse = rig::inverse(piece.matrix)?;
    let variant = match piece.face {
        FacePart::Mouth => Some((mouth_pose.filter(|&pose| pose < VARIANTS)? + 1) * 8),
        FacePart::Eye | FacePart::Brow => Some((piece.variant(frame) + 1) * 8),
        _ => None,
    };
    let nearest = |offset: usize| {
        (0..VIEWS.len())
            .filter(|&index| {
                layer
                    .frames
                    .get(index + offset)
                    .is_some_and(|marks| !marks.is_empty())
                    && !piece.hidden[index]
            })
            .min_by_key(|&index| {
                let distance = index.abs_diff(view);
                distance.min(8 - distance)
            })
            .map(|index| index + offset)
    };
    let from = (turn.floor() as i32).rem_euclid(8) as usize;
    let blend = turn - turn.floor();
    let motion = piece.motion(frame);
    let deform = |marks: &[Mark]| -> SharedVec<Mark> {
        let bounds = marks.iter().fold(egui::Rect::NOTHING, |rect, mark| {
            rect.union(crate::vector::control_bounds(mark))
        });
        let limits = |slot: usize| piece.gaze_limits[slot].unwrap_or([12., 12., 8., 8.]);
        let pupil_offset = |slot: usize| {
            let Some((center, radius)) = piece.pupil_range(slot) else {
                let limits = limits(slot);
                return egui::vec2(
                    motion[0] * limits[usize::from(motion[0] >= 0.)],
                    motion[1] * limits[2 + usize::from(motion[1] >= 0.)],
                );
            };
            center - bounds.center() + radius * egui::vec2(motion[0], motion[1])
        };
        let next = (from + 1) % 8;
        let gaze = pupil_offset(from) * (1. - blend) + pupil_offset(next) * blend;
        let brow = |p, slot| brow_offset(piece, bounds, p, slot, motion);
        marks
            .iter()
            .map(|mark| {
                let adjusted = if motion == [0.; 3] || !bounds.is_finite() {
                    mark.clone()
                } else {
                    match piece.face {
                        FacePart::Pupil => crate::vector::transform(mark, |p| p + gaze),
                        FacePart::Brow => crate::vector::transform(mark, |p| {
                            p + egui::vec2(0., brow(p, from) * (1. - blend) + brow(p, next) * blend)
                        }),
                        _ => mark.clone(),
                    }
                };
                crate::features::affine(&adjusted, inverse)
            })
            .collect()
    };
    let offset = variant.unwrap_or(0);
    let transition = drawn_transition(turn, piece.shape_transitions, |slot| {
        !piece.hidden[slot]
            && layer
                .frames
                .get(offset + slot)
                .is_some_and(|marks| !marks.is_empty())
    })
    .filter(|(a, b, _)| {
        crate::animation::shape_pair(&layer.frames[offset + a], &layer.frames[offset + b])
    });
    let mut marks = if let Some((a, b, blend)) = transition {
        let (first, last) = (&layer.frames[offset + a], &layer.frames[offset + b]);
        let morphed: Vec<_> = first
            .iter()
            .zip(last)
            .map(|(a, b)| crate::animation::morph(a, b, blend))
            .collect();
        deform(&morphed)
    } else {
        let slot = match variant {
            Some(offset) if piece.face == FacePart::Mouth => nearest(offset)?,
            Some(offset) => nearest(offset).or_else(|| nearest(0))?,
            None => nearest(0)?,
        };
        deform(&layer.frames[slot])
    };
    if let Some(eye) = piece.eye_mask.filter(|_| piece.face == FacePart::Pupil)
        && let Some(mask) = symbol
            .layers
            .iter()
            .filter_map(|layer| layer.options.character_piece.as_ref())
            .find(|piece| piece.track == eye && piece.face == FacePart::Eye)
    {
        let Some((layers, _)) = artwork(doc, eye, frame, None) else {
            return Some((Default::default(), 0));
        };
        let matrix = rig::multiply(inverse, mask.matrix);
        let outline = layers
            .iter()
            .flat_map(|layer| layer.frames[0].iter())
            .filter(|mark| mark.fill.is_some())
            .map(|mark| crate::features::affine(mark, matrix))
            .collect::<Vec<_>>();
        let area = crate::features::ink(&outline);
        marks = marks
            .iter()
            .flat_map(|mark| crate::features::clip_to(mark, &area))
            .collect();
    }
    Some((
        vec![Layer {
            frames: vec![marks].into(),
            options: LayerOptions {
                keys: vec![true].into(),
                ..Default::default()
            },
            ..layer.clone()
        }]
        .into(),
        0,
    ))
}

pub(crate) fn apply_bone_names(rig: &mut rig::Rig, layers: &[Layer]) {
    for layer in layers {
        if let Some(id) = layer
            .options
            .character_piece
            .as_ref()
            .and_then(|p| p.renamed_bone)
            && let Some(bone) = rig.bones.iter_mut().find(|bone| bone.id == id)
        {
            bone.name.clone_from(&layer.name);
        }
    }
}

fn brow_profile_offset(x: f32, anchors: [f32; 3], width: f32, motion: [f32; 3]) -> f32 {
    let [left, center, right] = anchors;
    let (x0, x1, v0, v1) = if x <= center {
        (left, center, motion[0], motion[1])
    } else {
        (center, right, motion[1], motion[2])
    };
    let t = ((x - x0) / (x1 - x0).max(0.01)).clamp(0., 1.);
    let t = t * t * (3. - 2. * t);
    width * 0.35 * (v0 * (1. - t) + v1 * t)
}

fn brow_offset(
    piece: &Piece,
    bounds: egui::Rect,
    p: egui::Pos2,
    slot: usize,
    motion: [f32; 3],
) -> f32 {
    if let [Some(a), Some(b), Some(c)] = piece.brow_points[slot] {
        return brow_profile_offset(p.x, [a[0], b[0], c[0]], bounds.width(), motion);
    }
    let x = ((p.x - bounds.center().x) / (bounds.width() / 2.).max(1.)).clamp(-1., 1.);
    bounds.width() * (motion[1] * 0.35 + motion[0] * x * 0.25 - motion[2] * (1. - x * x) * 0.3)
}

fn deform_brow_mark(mark: &Mark, offset: impl Fn(egui::Pos2) -> f32) -> Mark {
    let mut mark = mark.clone();
    if mark.style.instance.is_none()
        && mark.style.text.is_none()
        && mark.style.media_image.is_none()
        && (mark.nodes.len() > 1 || mark.points.len() > 1)
    {
        let curves = crate::vector::cubics(&mark);
        let spacing = (crate::vector::control_bounds(&mark).width() * 0.25).max(1.);
        let segments: Vec<_> = curves
            .into_iter()
            .flat_map(|curve| {
                let [a, b, c, d] = curve.0;
                let span = a.x.max(b.x).max(c.x).max(d.x) - a.x.min(b.x).min(c.x).min(d.x);
                let steps = ((span / spacing).ceil() as usize).clamp(1, 4);
                (0..steps).map(move |part| {
                    curve.sub(part as f32 / steps as f32, (part + 1) as f32 / steps as f32)
                })
            })
            .collect();
        if !segments.is_empty() {
            let mut subdivided = crate::vector::from_cubics(&segments, &mark);
            subdivided.holes = mark.holes;
            mark = subdivided;
        }
    }
    crate::vector::transform(&mark, |point| point + egui::vec2(0., offset(point)))
}

pub(crate) fn mouth_layers(symbol: &Symbol, track: u64) -> impl Iterator<Item = (&Layer, &Piece)> {
    symbol.layers.iter().filter_map(move |layer| {
        let piece = layer.options.character_piece.as_ref()?;
        (layer.visible && piece.eye_parent == Some(track) && piece.is_lip())
            .then_some((layer, piece))
    })
}

fn lip_drawing(layer: &Layer, piece: &Piece, pose: usize, turn: f32) -> SharedVec<Mark> {
    if piece.hidden[turn.round().rem_euclid(8.) as usize] {
        return Default::default();
    }
    let marks = |view: usize| layer.frames.get((pose + 1) * 8 + view);
    let aligned = |view| {
        let frame = (pose + 1) * 8 + view;
        let artwork = marks(view).unwrap();
        if let Some(point) = piece.wrist_points.get(frame).and_then(|point| *point)
            && let Some(origin) = piece.wrist_origin
        {
            let delta = crate::vector::pos(origin) - crate::vector::pos(point);
            if delta != egui::Vec2::ZERO {
                return artwork
                    .iter()
                    .map(|mark| crate::vector::transform(mark, |p| p + delta))
                    .collect();
            }
        }
        artwork.clone()
    };
    let drawn =
        |view: usize| !piece.hidden[view] && marks(view).is_some_and(|marks| !marks.is_empty());
    if let Some((a, b, t)) = drawn_transition(turn, piece.lip_tweens[pose], drawn)
        && let Some(drawing) = eye_tween(&aligned(a), &aligned(b), t)
    {
        return drawing;
    }
    let view = (turn.round() as i32).rem_euclid(8) as usize;
    (0usize..8)
        .filter(|&slot| drawn(slot))
        .min_by_key(|&slot| {
            let distance = slot.abs_diff(view);
            distance.min(8 - distance)
        })
        .map(aligned)
        .unwrap_or_default()
}

fn control_drawing(layer: &Layer, piece: &Piece, frame: usize, body_turn: f32) -> SharedVec<Mark> {
    let axes = piece.motion(frame);
    let count = piece.lip_poses.len();
    if count == 0 {
        return Default::default();
    }
    if piece.face == FacePart::Hand {
        let angle = axes[1] * 8.;
        let pose = piece.variant(frame).min(count - 1);
        let key_index = piece.face_keys.partition_point(|key| key.frame <= frame);
        if piece.tween_hand_poses
            && let Some(start) = key_index
                .checked_sub(1)
                .and_then(|key| piece.face_keys.get(key))
            && let Some(end) = piece.face_keys.get(key_index)
            && start.variant != end.variant
        {
            let t = (frame - start.frame) as f32 / (end.frame - start.frame) as f32;
            let first = lip_drawing(layer, piece, pose, angle);
            let last = lip_drawing(layer, piece, end.variant.min(count - 1), angle);
            if let Some(marks) = eye_tween(&first, &last, t) {
                return marks;
            }
        }
        return lip_drawing(layer, piece, pose, angle);
    }
    if piece.face != FacePart::Slider {
        return lip_drawing(layer, piece, piece.variant(frame).min(count - 1), body_turn);
    }
    let position = (axes[0] * 127.).clamp(0., (count - 1) as f32);
    let mut left = None;
    let mut right = None;
    for pose in 0..count {
        let has_art = (0..8)
            .any(|view| !piece.hidden[view] && !layer.frames[(pose + 1) * 8 + view].is_empty());
        if !has_art {
            continue;
        }
        if pose as f32 <= position {
            left = Some(pose);
        }
        if pose as f32 >= position {
            right = Some(pose);
            break;
        }
    }
    match (left, right) {
        (Some(a), Some(b)) if a != b => {
            let t = (position - a as f32) / (b - a) as f32;
            let first = lip_drawing(layer, piece, a, body_turn);
            let last = lip_drawing(layer, piece, b, body_turn);
            eye_tween(&first, &last, t).unwrap_or_else(|| if t < 0.5 { first } else { last })
        }
        (Some(pose), _) | (_, Some(pose)) => lip_drawing(layer, piece, pose, body_turn),
        _ => Default::default(),
    }
}

pub(crate) fn rigged_eye_artwork(
    doc: &crate::Document,
    track: u64,
    frame: usize,
    mouth: Option<&crate::lip::RouteScope<'_>>,
) -> Vec<Mark> {
    if track == 0 {
        return Vec::new();
    }
    let rig = &doc.extras.rig;
    let Some(turn) = rig.character_turn(track, frame) else {
        return Vec::new();
    };
    let view = (turn.round() as i32).rem_euclid(8) as usize;
    let Some(symbol) = doc
        .extras
        .symbols
        .iter()
        .find(|s| Some(s.id) == rig.character_symbol)
    else {
        return Vec::new();
    };
    let mut output = Vec::new();
    for layer in &symbol.layers {
        let Some(piece) = layer
            .options
            .character_piece
            .as_ref()
            .filter(|p| p.eye_parent.is_some())
        else {
            continue;
        };
        if !layer.visible || (piece.face != FacePart::Hand && piece.hidden[view]) {
            continue;
        }
        let eye_mask = piece
            .eye_mask
            .filter(|_| piece.face == FacePart::Pupil)
            .and_then(|id| {
                symbol.layers.iter().find_map(|eye| {
                    let mask = eye.options.character_piece.as_ref()?;
                    (mask.track == id && mask.face == FacePart::Eye && mask.eye_parent.is_some())
                        .then_some((eye, mask))
                })
            });
        let anchor = eye_mask.map_or(piece, |(_, eye)| eye);
        if anchor.eye_parent != Some(track) {
            continue;
        }
        let Some(inverse) = rig::inverse(anchor.matrix) else {
            continue;
        };
        if piece.face.control() {
            output.extend(
                control_drawing(layer, piece, frame, turn)
                    .iter()
                    .map(|mark| crate::features::affine(mark, inverse)),
            );
            continue;
        }
        if piece.is_lip() {
            let pose = mouth
                .and_then(|scope| {
                    scope.child_slot(crate::lip::character_mouth_slot(symbol.id, piece.track))
                })
                .and_then(|scope| scope.pose())
                .filter(|&pose| pose < piece.lip_poses.len())
                .unwrap_or(0);
            output.extend(
                lip_drawing(layer, piece, pose, turn)
                    .iter()
                    .map(|mark| crate::features::affine(mark, inverse)),
            );
            continue;
        }
        if piece.face == FacePart::Brow {
            let drawn = |slot: usize| !piece.hidden[slot] && !layer.frames[slot].is_empty();
            let Some(nearest) = (0..8).filter(|&slot| drawn(slot)).min_by_key(|&slot| {
                let distance = slot.abs_diff(view);
                distance.min(8 - distance)
            }) else {
                continue;
            };
            let transition =
                drawn_transition(turn, piece.shape_transitions, drawn).and_then(|(a, b, t)| {
                    eye_tween(&layer.frames[a], &layer.frames[b], t).map(|marks| (a, b, t, marks))
                });
            let (a, b, t, marks) =
                transition.unwrap_or_else(|| (nearest, nearest, 0., layer.frames[nearest].clone()));
            let motion = piece.motion(frame);
            let bounds_for = |marks: &[Mark]| {
                marks.iter().fold(egui::Rect::NOTHING, |bounds, mark| {
                    bounds.union(crate::vector::control_bounds(mark))
                })
            };
            let bounds = bounds_for(&marks);
            let profile = |view: usize| {
                let bounds = bounds_for(&*layer.frames[view]);
                let points: [Option<[f32; 2]>; 3] = piece.brow_points[view];
                if let [Some(left), Some(center), Some(right)] = points {
                    [left[0], center[0], right[0]]
                } else {
                    [bounds.left(), bounds.center().x, bounds.right()]
                }
            };
            let first = profile(a);
            let last = profile(b);
            let anchors = std::array::from_fn(|index| egui::lerp(first[index]..=last[index], t));
            output.extend(marks.iter().map(|mark| {
                if motion == [0.; 3] || !bounds.is_finite() {
                    return crate::features::affine(mark, inverse);
                }
                let bent = deform_brow_mark(mark, |point| {
                    brow_profile_offset(point.x, anchors, bounds.width(), motion)
                });
                crate::features::affine(&bent, inverse)
            }));
            continue;
        }
        if piece.face == FacePart::Pupil {
            let drawn = |slot: usize| !piece.hidden[slot] && !layer.frames[slot].is_empty();
            let Some(nearest) = (0..8).filter(|&slot| drawn(slot)).min_by_key(|slot| {
                let d = slot.abs_diff(view);
                d.min(8 - d)
            }) else {
                continue;
            };
            let [x, y, _] = piece.motion(frame);
            let offset = |slot: usize| {
                let bounds = layer.frames[slot]
                    .iter()
                    .fold(egui::Rect::NOTHING, |bounds, mark| {
                        bounds.union(crate::vector::control_bounds(mark))
                    });
                if !bounds.is_finite() {
                    return egui::Vec2::ZERO;
                }
                let (center, radius) = piece.pupil_range(slot).unwrap_or_else(|| {
                    let nearest_radius = (0usize..8)
                        .filter_map(|view| {
                            let distance = view.abs_diff(slot);
                            piece
                                .pupil_range(view)
                                .map(|(_, radius)| (distance.min(8 - distance), radius))
                        })
                        .min_by_key(|(distance, _)| *distance)
                        .map(|(_, radius)| radius);
                    (
                        bounds.center(),
                        nearest_radius.unwrap_or(bounds.size() * 0.5),
                    )
                });
                center - bounds.center() + radius * egui::vec2(x, y)
            };
            let pupil: Vec<_> = if let Some((a, b, t)) =
                drawn_transition(turn, piece.shape_transitions, drawn)
                && crate::animation::shape_pair(&layer.frames[a], &layer.frames[b])
            {
                let delta = offset(a) * (1. - t) + offset(b) * t;
                layer.frames[a]
                    .iter()
                    .zip(layer.frames[b].iter())
                    .map(|(a, b)| {
                        crate::vector::transform(&crate::animation::morph(a, b, t), |p| p + delta)
                    })
                    .collect()
            } else {
                let delta = offset(nearest);
                layer.frames[nearest]
                    .iter()
                    .map(|mark| crate::vector::transform(mark, |p| p + delta))
                    .collect()
            };
            if let Some((eye, mask)) = eye_mask {
                let drawing = eye_artwork(eye, mask, turn, frame);
                let outline = drawing
                    .iter()
                    .cloned()
                    .filter_map(|mut mark| {
                        if !mark.closed && !mark.rectangle && mark.fill.is_none() {
                            return None;
                        }
                        mark.fill = Some([255; 4]);
                        mark.stroke = false;
                        Some(mark)
                    })
                    .collect::<Vec<_>>();
                let area = crate::features::ink(&outline);
                let rim = drawing
                    .iter()
                    .filter(|mark| mark.stroke && mark.color[3] != 0)
                    .map(crate::width::outline)
                    .collect::<Vec<_>>();
                let area = if rim.is_empty() {
                    area
                } else {
                    use geo::BooleanOps;
                    area.difference(&geo::algorithm::bool_ops::unary_union(&rim))
                };
                output.extend(
                    pupil
                        .iter()
                        .flat_map(|mark| crate::features::clip_to(mark, &area))
                        .map(|mark| crate::features::affine(&mark, inverse)),
                );
            } else {
                output.extend(
                    pupil
                        .iter()
                        .map(|mark| crate::features::affine(mark, inverse)),
                );
            }
            continue;
        }
        output.extend(
            eye_artwork(layer, piece, turn, frame)
                .iter()
                .map(|mark| crate::features::affine(mark, inverse)),
        );
    }
    output
}

// A bucket-filled eye can retain scraps of its previous fill after the outline is edited.
// For a single outlined opening with one solid fill, use the edited contour for both.
fn eye_opening(marks: &[Mark]) -> Option<[Mark; 2]> {
    if let [mark] = marks
        && mark.closed
        && mark.stroke
        && mark.fill.is_some()
        && !mark.style.guide
        && !mark.style.raster_tile
        && mark.style.instance.is_none()
        && mark.style.text.is_none()
        && mark.style.media_image.is_none()
    {
        let mut fill = mark.clone();
        fill.stroke = false;
        let mut outline = mark.clone();
        outline.fill = None;
        outline.style.gradient = None;
        return Some([fill, outline]);
    }
    let mut outline = None;
    let mut fill = None::<&Mark>;
    for mark in marks {
        if mark.style.instance.is_some()
            || mark.style.text.is_some()
            || mark.style.media_image.is_some()
            || mark.style.gradient.is_some()
            || mark.style.guide
            || mark.style.raster_tile
        {
            return None;
        }
        if let Some(color) = mark.fill {
            if !mark.closed
                || mark.stroke
                || !mark.holes.is_empty()
                || fill.is_some_and(|first| {
                    first.fill != Some(color) || first.style.palette_fill != mark.style.palette_fill
                })
            {
                return None;
            }
            fill = Some(mark);
        } else if mark.closed && mark.stroke && outline.is_none() && mark.holes.is_empty() {
            outline = Some(mark);
        } else {
            return None;
        }
    }
    let outline = outline?;
    let source = fill?;
    let mut filled = outline.clone();
    filled.stroke = false;
    filled.fill = source.fill;
    filled.style.palette_fill = source.style.palette_fill;
    Some([filled, outline.clone()])
}

pub(crate) fn has_fragmented_eye_fill(marks: &[Mark]) -> bool {
    marks.len() > 1 && eye_opening(marks).is_some()
}

fn eye_pair(a: &[Mark], b: &[Mark]) -> bool {
    crate::animation::shape_pair_with(a, b, |_, _| true)
}

fn compatible_eye_tween(a: &[Mark], b: &[Mark]) -> bool {
    eye_pair(a, b)
        || eye_opening(a)
            .zip(eye_opening(b))
            .is_some_and(|(a, b)| eye_pair(&a, &b))
}

fn eye_tween(a: &[Mark], b: &[Mark], t: f32) -> Option<SharedVec<Mark>> {
    let morph = |a: &[Mark], b: &[Mark]| {
        a.iter()
            .zip(b)
            .map(|(a, b)| crate::animation::morph(a, b, t))
            .collect()
    };
    if eye_pair(a, b) {
        return Some(morph(a, b));
    }
    let a = eye_opening(a)?;
    let b = eye_opening(b)?;
    eye_pair(&a, &b).then(|| morph(&a, &b))
}

fn eye_artwork(layer: &Layer, piece: &Piece, turn: f32, frame: usize) -> SharedVec<Mark> {
    if !layer.visible {
        return Default::default();
    }
    let view = (turn.round() as i32).rem_euclid(8) as usize;
    if layer.frames.len() < 32 {
        return Default::default();
    }
    let Some(view) = (0..VIEWS.len())
        .filter(|&slot| {
            !piece.hidden[slot] && (1..=3).any(|state| !layer.frames[state * 8 + slot].is_empty())
        })
        .min_by_key(|&slot| {
            let distance = slot.abs_diff(view);
            distance.min(8 - distance)
        })
    else {
        return Default::default();
    };
    let value = piece.motion(frame)[0].clamp(0., 1.) * 2.;
    let drawing = |angle: usize| -> SharedVec<Mark> {
        let state = |index: usize| &layer.frames[(index + 1) * 8 + angle];
        let low = (value.floor() as usize).min(1);
        let (a, b, t, enabled) = if state(1).is_empty() {
            (
                state(0),
                state(2),
                value / 2.,
                piece.vertical_tweens[angle] != 0,
            )
        } else {
            (
                state(low),
                state(low + 1),
                value - low as f32,
                piece.vertical_tweens[angle] & (1 << low) != 0,
            )
        };
        if enabled && let Some(marks) = eye_tween(a, b, t) {
            return marks;
        }
        (0..3)
            .filter(|&index| !state(index).is_empty())
            .min_by(|&a, &b| {
                (a as f32 - value)
                    .abs()
                    .total_cmp(&(b as f32 - value).abs())
            })
            .map_or_else(SharedVec::default, |index| state(index).clone())
    };
    if let Some((from, next, blend)) = drawn_transition(turn, piece.shape_transitions, |slot| {
        !piece.hidden[slot] && (1..=3).any(|state| !layer.frames[state * 8 + slot].is_empty())
    }) {
        let a = drawing(from);
        let b = drawing(next);
        if let Some(marks) = eye_tween(&a, &b, blend) {
            return marks;
        }
    }
    drawing(view)
}

pub(crate) fn order(doc: &crate::Document, mark: &Mark, frame: usize) -> i16 {
    let rig = &doc.extras.rig;
    let Some(angle) = rig.character_angle(mark.style.track, frame) else {
        return 0;
    };
    doc.extras
        .symbols
        .iter()
        .find(|symbol| Some(symbol.id) == rig.character_symbol)
        .and_then(|symbol| {
            symbol.layers.iter().find_map(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .filter(|piece| piece.track == mark.style.track)
            })
        })
        .map_or(0, |piece| piece.order[angle])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TweenCheck {
    Ready,
    NoDrawing,
    NoDestination,
    Disabled,
    Incompatible,
}

impl TweenCheck {
    fn label(self, vertical: bool) -> &'static str {
        tr(match (self, vertical) {
            (Self::Ready, true) => "Vertical tween ready",
            (Self::Ready, false) => "Horizontal tween ready",
            (Self::NoDrawing, _) => "Draw this view",
            (Self::NoDestination, true) => "Draw another eyelid state",
            (Self::NoDestination, false) => "Draw another view",
            (Self::Disabled, true) => "Enable vertical tween",
            (Self::Disabled, false) => "Enable horizontal tween",
            (Self::Incompatible, true) => "Check eyelid state shapes",
            (Self::Incompatible, false) => "Check shapes between views",
        })
    }
}

struct RigTweenChecks {
    horizontal: TweenCheck,
    vertical: Option<TweenCheck>,
}

impl RigTweenChecks {
    fn ready(&self) -> bool {
        self.horizontal == TweenCheck::Ready
            && self.vertical.is_none_or(|check| check == TweenCheck::Ready)
    }
}

fn rig_view_drawn(layer: &Layer, piece: &Piece, view: usize) -> bool {
    !piece.hidden[view]
        && if piece.face == FacePart::Eye {
            [8, 16, 24].into_iter().any(|offset| {
                layer
                    .frames
                    .get(offset + view)
                    .is_some_and(|marks| !marks.is_empty())
            })
        } else {
            !layer.frames[view].is_empty()
        }
}

fn rig_tween_checks(layer: &Layer, piece: &Piece, view: usize) -> RigTweenChecks {
    use TweenCheck::*;
    let states = |view: usize| -> [&[Mark]; 3] {
        [8usize, 16, 24].map(|offset| {
            layer
                .frames
                .get(offset + view)
                .map_or(&[][..], |marks| &**marks)
        })
    };
    let horizontal = if !rig_view_drawn(layer, piece, view) {
        NoDrawing
    } else if let Some(next) = (1..8)
        .map(|step| (view + step) % 8)
        .find(|&other| rig_view_drawn(layer, piece, other))
    {
        let compatible = match piece.face {
            FacePart::Eye => {
                let mut pairs = states(view)
                    .into_iter()
                    .zip(states(next))
                    .filter(|(a, b)| !a.is_empty() && !b.is_empty())
                    .peekable();
                pairs.peek().is_some() && pairs.all(|(a, b)| compatible_eye_tween(a, b))
            }
            FacePart::Brow => compatible_eye_tween(&layer.frames[view], &layer.frames[next]),
            _ => crate::animation::shape_pair(&layer.frames[view], &layer.frames[next]),
        };
        if !compatible {
            Incompatible
        } else if piece.shape_transitions & (1 << view) == 0 {
            Disabled
        } else {
            Ready
        }
    } else {
        NoDestination
    };
    let vertical = (piece.face == FacePart::Eye).then(|| {
        let [open, half, closed] = states(view);
        if open.is_empty() || closed.is_empty() {
            return NoDestination;
        }
        let flags = piece.vertical_tweens[view];
        let (compatible, enabled) = if half.is_empty() {
            (compatible_eye_tween(open, closed), flags != 0)
        } else {
            (
                compatible_eye_tween(open, half) && compatible_eye_tween(half, closed),
                flags & 3 == 3,
            )
        };
        if !compatible {
            Incompatible
        } else if !enabled {
            Disabled
        } else {
            Ready
        }
    });
    RigTweenChecks {
        horizontal,
        vertical,
    }
}

fn rig_check_row(ui: &mut egui::Ui, ready: bool, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(13.), egui::Sense::hover());
        let color = if ready {
            egui::Color32::from_rgb(105, 220, 151)
        } else {
            crate::themes::muted()
        };
        ui.painter().rect_stroke(
            rect.shrink(1.),
            2.,
            egui::Stroke::new(1., color),
            egui::StrokeKind::Inside,
        );
        if ready {
            let check =
                [(-3., 0.), (-1., 2.), (3., -2.)].map(|(x, y)| rect.center() + egui::vec2(x, y));
            ui.painter().add(egui::Shape::line(
                check.to_vec(),
                egui::Stroke::new(1.4, color),
            ));
        }
        ui.add(egui::Label::new(egui::RichText::new(text).small().color(color)).truncate())
            .on_hover_text(text);
    });
}

impl Editor {
    pub(crate) fn capture_rig_controls(
        &self,
        frame: usize,
        group: usize,
    ) -> Vec<(u64, usize, [f32; 3])> {
        let Some(symbol) = self
            .doc
            .extras
            .symbols
            .iter()
            .find(|symbol| Some(symbol.id) == self.doc.extras.rig.character_symbol)
        else {
            return Vec::new();
        };
        symbol
            .layers
            .iter()
            .filter_map(|layer| {
                let piece = layer.options.character_piece.as_ref()?;
                let face = matches!(
                    piece.face,
                    FacePart::Eye
                        | FacePart::Pupil
                        | FacePart::Brow
                        | FacePart::Slider
                        | FacePart::Toggle
                        | FacePart::Switch
                );
                ((group == 1 && face)
                    || (group == 2 && piece.face == FacePart::Hand)
                    || (group == 3 && (face || piece.face == FacePart::Hand)))
                    .then(|| (piece.track, piece.variant(frame), piece.motion(frame)))
            })
            .collect()
    }

    pub(crate) fn apply_rig_controls(&mut self, frame: usize, values: &[(u64, usize, [f32; 3])]) {
        let Some(symbol) = self
            .doc
            .extras
            .symbols
            .iter_mut()
            .find(|symbol| Some(symbol.id) == self.doc.extras.rig.character_symbol)
        else {
            return;
        };
        for piece in symbol
            .layers
            .iter_mut()
            .filter_map(|layer| layer.options.character_piece.as_mut())
        {
            if let Some(&(_, variant, axes)) =
                values.iter().find(|(track, _, _)| *track == piece.track)
            {
                piece.set_motion(frame, axes);
                if matches!(
                    piece.face,
                    FacePart::Hand | FacePart::Toggle | FacePart::Switch
                ) && variant < piece.lip_poses.len()
                {
                    piece.set_variant(frame, variant);
                }
            }
        }
    }

    pub(crate) fn rig_control_channels(&self) -> Vec<(u64, bool, String)> {
        let Some(symbol) = self
            .doc
            .extras
            .symbols
            .iter()
            .find(|symbol| Some(symbol.id) == self.doc.extras.rig.character_symbol)
        else {
            return Vec::new();
        };
        let mut channels = Vec::new();
        for layer in &symbol.layers {
            let Some(piece) = layer.options.character_piece.as_ref() else {
                continue;
            };
            if matches!(
                piece.face,
                FacePart::Hand | FacePart::Toggle | FacePart::Switch
            ) || !piece.face_keys.is_empty()
            {
                channels.push((
                    piece.track,
                    false,
                    format!("{} · {}", layer.name, tr("Pose")),
                ));
            }
            if matches!(
                piece.face,
                FacePart::Hand
                    | FacePart::Slider
                    | FacePart::Pupil
                    | FacePart::Brow
                    | FacePart::Eye
            ) {
                channels.push((
                    piece.track,
                    true,
                    format!(
                        "{} · {}",
                        layer.name,
                        tr(match piece.face {
                            FacePart::Hand => "Turnaround",
                            FacePart::Eye => "Eye closure",
                            FacePart::Pupil => "Gaze",
                            FacePart::Brow => "Brow points",
                            _ => "Slider",
                        })
                    ),
                ));
            }
        }
        channels
    }

    pub(crate) fn rig_control_frames(&self, track: u64, motion: bool) -> Vec<usize> {
        let piece = self
            .doc
            .extras
            .symbols
            .iter()
            .find(|symbol| Some(symbol.id) == self.doc.extras.rig.character_symbol)
            .and_then(|symbol| {
                symbol
                    .layers
                    .iter()
                    .filter_map(|layer| layer.options.character_piece.as_ref())
                    .find(|piece| piece.track == track)
            });
        piece.map_or_else(Vec::new, |piece| {
            if motion {
                piece.face_motion.iter().map(|key| key.frame).collect()
            } else {
                piece.face_keys.iter().map(|key| key.frame).collect()
            }
        })
    }

    fn rig_control_piece_mut(&mut self, track: u64) -> Option<&mut Piece> {
        let character = self.doc.extras.rig.character_symbol?;
        self.doc
            .extras
            .symbols
            .iter_mut()
            .find(|symbol| symbol.id == character)?
            .layers
            .iter_mut()
            .filter_map(|layer| layer.options.character_piece.as_mut())
            .find(|piece| piece.track == track)
    }

    pub(crate) fn insert_rig_control_key(&mut self, track: u64, motion: bool, frame: usize) {
        if self
            .rig_control_frames(track, motion)
            .binary_search(&frame)
            .is_ok()
        {
            return;
        }
        self.checkpoint();
        if let Some(piece) = self.rig_control_piece_mut(track) {
            if motion {
                piece.set_motion(frame, piece.motion(frame));
            } else {
                piece.set_variant(frame, piece.variant(frame));
            }
            self.recover();
        }
    }

    pub(crate) fn edit_rig_control_range(
        &mut self,
        track: u64,
        motion: bool,
        range: std::ops::RangeInclusive<usize>,
        offset: Option<isize>,
        duplicate: bool,
        ease: Option<RigEase>,
    ) {
        let frames = self.rig_control_frames(track, motion);
        let selected: Vec<_> = frames
            .iter()
            .copied()
            .filter(|frame| range.contains(frame))
            .collect();
        if selected.is_empty() {
            return;
        }
        if let Some(delta) = offset
            && selected.iter().any(|&frame| {
                frame.checked_add_signed(delta).is_none_or(|to| {
                    to >= self.doc.layers[0].frames.len()
                        || (frames.binary_search(&to).is_ok()
                            && (duplicate || !range.contains(&to)))
                })
            })
        {
            return;
        }
        self.checkpoint();
        let Some(piece) = self.rig_control_piece_mut(track) else {
            return;
        };
        if let Some(ease) = ease {
            for key in piece
                .face_motion
                .iter_mut()
                .filter(|key| range.contains(&key.frame))
            {
                key.ease = ease;
            }
        } else if motion {
            let mut moved: Vec<_> = offset
                .into_iter()
                .flat_map(|delta| {
                    piece
                        .face_motion
                        .iter()
                        .filter(|key| range.contains(&key.frame))
                        .cloned()
                        .map(move |mut key| {
                            key.frame = key.frame.checked_add_signed(delta).unwrap();
                            key
                        })
                })
                .collect();
            if !duplicate {
                piece.face_motion.retain(|key| !range.contains(&key.frame));
            }
            piece.face_motion.append(&mut moved);
            piece.face_motion.sort_by_key(|key| key.frame);
        } else {
            let mut moved: Vec<_> = offset
                .into_iter()
                .flat_map(|delta| {
                    piece
                        .face_keys
                        .iter()
                        .filter(|key| range.contains(&key.frame))
                        .cloned()
                        .map(move |mut key| {
                            key.frame = key.frame.checked_add_signed(delta).unwrap();
                            key
                        })
                })
                .collect();
            if !duplicate {
                piece.face_keys.retain(|key| !range.contains(&key.frame));
            }
            piece.face_keys.append(&mut moved);
            piece.face_keys.sort_by_key(|key| key.frame);
        }
        self.recover();
    }

    pub(crate) fn trim_rig_control_keys(&mut self, count: usize) {
        let character = self.doc.extras.rig.character_symbol;
        if let Some(symbol) = self
            .doc
            .extras
            .symbols
            .iter_mut()
            .find(|symbol| Some(symbol.id) == character)
        {
            for piece in symbol
                .layers
                .iter_mut()
                .filter_map(|layer| layer.options.character_piece.as_mut())
            {
                piece.face_motion.retain(|key| key.frame < count);
                piece.face_keys.retain(|key| key.frame < count);
            }
        }
    }

    fn wrist_tool(&self) -> Option<egui::Id> {
        if !self.in_character_symbol() || self.frame < 8 {
            return None;
        }
        let piece = self.doc.layers[self.layer]
            .options
            .character_piece
            .as_ref()?;
        (piece.face == FacePart::Hand)
            .then(|| egui::Id::unique(("wrist-point", piece.track, self.frame)))
    }

    pub(crate) fn wrist_point_input(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        view: crate::canvas::View,
    ) -> bool {
        let Some(tool) = self.wrist_tool() else {
            return false;
        };
        let armed = ui.data(|data| data.get_temp::<bool>(tool)).unwrap_or(false);
        let piece = self.doc.layers[self.layer]
            .options
            .character_piece
            .as_ref()
            .unwrap();
        let mut point = piece.wrist_points.get(self.frame).copied().flatten();
        let cursor = ui
            .input(|i| i.pointer.interact_pos())
            .filter(|&p| area.contains(p) && ui.ctx().layer_id_at(p) == Some(ui.layer_id()));
        if armed {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                ui.data_mut(|data| data.remove::<bool>(tool));
            }
            if let Some(cursor) = cursor {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                if ui.input(|i| i.pointer.primary_pressed()) {
                    let next = crate::vector::xy(view.world(cursor));
                    self.checkpoint();
                    let piece = self.doc.layers[self.layer]
                        .options
                        .character_piece
                        .as_mut()
                        .unwrap();
                    if piece.wrist_points.len() <= self.frame {
                        piece.wrist_points.resize(self.frame + 1, None);
                    }
                    if piece.wrist_origin.is_none()
                        || piece.wrist_origin == point
                            && piece.wrist_points.iter().flatten().count() == 1
                    {
                        piece.wrist_origin = Some(next);
                    }
                    piece.wrist_points[self.frame] = Some(next);
                    point = Some(next);
                    ui.data_mut(|data| data.remove::<bool>(tool));
                    self.recover();
                }
            }
        }
        if let Some(point) = point {
            let screen = view.screen(crate::vector::pos(point));
            let painter = ui
                .ctx()
                .layer_painter(egui::LayerId::new(
                    egui::Order::Middle,
                    ui.scope_id().with("wrist-point-guide"),
                ))
                .with_clip_rect(area);
            painter.circle_stroke(screen, 6., egui::Stroke::new(1.5, crate::themes::accent()));
            painter.circle_filled(screen, 2., crate::themes::accent());
        }
        armed && cursor.is_some()
    }

    fn brow_point_tool(&self) -> Option<egui::Id> {
        self.in_character_symbol().then_some(())?;
        let piece = self.doc.layers[self.layer]
            .options
            .character_piece
            .as_ref()?;
        (piece.face == FacePart::Brow)
            .then(|| egui::Id::unique(("brow-point-tool", piece.track, self.frame % 8)))
    }

    pub(crate) fn brow_point_input(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        view: crate::canvas::View,
    ) -> bool {
        let Some(tool) = self.brow_point_tool() else {
            return false;
        };
        let slot = self.frame % 8;
        let armed = ui.data(|data| data.get_temp::<usize>(tool));
        let mut points = self.doc.layers[self.layer]
            .options
            .character_piece
            .as_ref()
            .unwrap()
            .brow_points[slot];
        let painter = ui
            .ctx()
            .layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                ui.scope_id().with("brow-point-guide"),
            ))
            .with_clip_rect(area);
        let cursor = ui
            .input(|input| input.pointer.interact_pos())
            .filter(|&point| {
                area.contains(point) && ui.ctx().layer_id_at(point) == Some(ui.layer_id())
            });
        if let Some(axis) = armed {
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                ui.data_mut(|data| data.remove::<usize>(tool));
            }
            if let Some(cursor) = cursor {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                if ui.input(|input| input.pointer.primary_pressed()) {
                    let point = crate::vector::xy(view.world(cursor));
                    let out_of_order = points.iter().enumerate().any(|(other, value)| {
                        value.is_some_and(|p| {
                            (other < axis && p[0] >= point[0]) || (other > axis && p[0] <= point[0])
                        })
                    });
                    if out_of_order {
                        self.status = tr("Place brow points in order from left to right.").into();
                    } else {
                        self.checkpoint();
                        points[axis] = Some(point);
                        self.doc.layers[self.layer]
                            .options
                            .character_piece
                            .as_mut()
                            .unwrap()
                            .brow_points[slot] = points;
                        ui.data_mut(|data| data.remove::<usize>(tool));
                        self.recover();
                    }
                }
            }
        }
        for (axis, point) in points
            .into_iter()
            .enumerate()
            .filter_map(|(i, p)| p.map(|p| (i, p)))
        {
            let p = view.screen(crate::vector::pos(point));
            let color = egui::Color32::from_rgb(245, 202, 102);
            painter.circle_filled(p, 4.5, color);
            painter.circle_stroke(p, 4.5, egui::Stroke::new(1., egui::Color32::BLACK));
            painter.text(
                p + egui::vec2(0., -8.),
                egui::Align2::CENTER_BOTTOM,
                ["L", "C", "R"][axis],
                egui::FontId::proportional(11.),
                color,
            );
        }
        armed.is_some() && cursor.is_some()
    }

    pub(crate) fn pupil_area_input(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        view: crate::canvas::View,
    ) -> bool {
        if !self.in_character_symbol() {
            return false;
        }
        let Some(piece) = self.doc.layers[self.layer]
            .options
            .character_piece
            .as_ref()
            .filter(|p| p.face == FacePart::Pupil)
        else {
            return false;
        };
        let tool = egui::Id::unique("pupil-area-tool");
        if !ui.data(|d| d.get_temp::<bool>(tool)).unwrap_or(false) {
            return false;
        }
        let slot = self.frame % 8;
        let id = ui.scope_id().with(("pupil-area", piece.track, slot));
        let start = ui.data(|d| d.get_temp::<egui::Pos2>(id));
        let (pointer, pressed, released) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_pressed(),
                i.pointer.primary_released(),
            )
        });
        let hit =
            pointer.filter(|&p| area.contains(p) && ui.ctx().layer_id_at(p) == Some(ui.layer_id()));
        if let Some(p) = hit {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            if pressed {
                ui.data_mut(|d| d.insert_temp(id, view.world(p)));
            }
        }
        let draft = start
            .zip(pointer.map(|p| view.world(p)))
            .map(|(a, b)| [crate::vector::xy(a), crate::vector::xy(b)]);
        if let Some((center, radius)) = draft
            .map(|[a, b]| {
                let a = crate::vector::pos(a);
                (a, egui::Vec2::splat(a.distance(crate::vector::pos(b))))
            })
            .or_else(|| piece.pupil_range(slot))
        {
            let center = view.screen(center);
            let radius = radius * view.scale.abs();
            let painter = ui
                .ctx()
                .layer_painter(egui::LayerId::new(
                    egui::Order::Middle,
                    ui.scope_id().with("pupil-area-guide"),
                ))
                .with_clip_rect(area);
            let stroke = egui::Stroke::new(2., egui::Color32::from_rgb(105, 220, 151));
            if radius.x == radius.y {
                painter.circle_stroke(center, radius.x, stroke);
            } else {
                painter.rect_stroke(
                    egui::Rect::from_center_size(center, radius * 2.),
                    3.,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            if released && draft.is_some() && radius.x >= 3. {
                self.checkpoint();
                let piece = self.doc.layers[self.layer]
                    .options
                    .character_piece
                    .as_mut()
                    .unwrap();
                piece.pupil_area[slot] = draft;
                piece.circular_pupil_areas |= 1 << slot;
                self.recover();
            }
        }
        if released {
            ui.data_mut(|d| d.remove::<egui::Pos2>(id));
        }
        hit.is_some() || start.is_some()
    }
    pub(crate) fn character_edit_artwork(&self) -> Vec<(usize, usize, Mark)> {
        let view = self.frame % 8;
        let state = (self.frame / 8).clamp(1, 3);
        let mut layers_by_state = vec![None; self.doc.layers.len()];
        let mut used = vec![false; self.doc.layers[0].frames.len() / 8];
        for (row, layer) in self.doc.layers.iter().enumerate() {
            if !layer.visible {
                continue;
            }
            let eye = layer
                .options
                .character_piece
                .as_ref()
                .is_some_and(|piece| piece.face == FacePart::Eye);
            let state = if row == self.layer {
                self.frame / 8
            } else if let Some(piece) = layer
                .options
                .character_piece
                .as_ref()
                .filter(|piece| !piece.lip_poses.is_empty())
            {
                (1..=piece.lip_poses.len())
                    .find(|state| !layer.frames[state * 8 + view].is_empty())
                    .unwrap_or(0)
            } else if eye {
                [state, 1, 2, 3]
                    .into_iter()
                    .find(|&state| {
                        layer
                            .frames
                            .get(state * 8 + view)
                            .is_some_and(|marks| !marks.is_empty())
                    })
                    .unwrap_or(1)
            } else {
                0
            };
            if state < used.len() {
                layers_by_state[row] = Some(state);
                used[state] = true;
            }
        }
        let dim = self.character_face_edit();
        let mut result = Vec::new();
        for (state, used) in used.into_iter().enumerate() {
            if !used {
                continue;
            }
            let slot = view + state * 8;
            for (row, index, mut mark) in crate::features::resolve_at(&self.doc, slot, false, slot)
            {
                if layers_by_state[row] != Some(state) {
                    continue;
                }
                if dim && row != self.layer {
                    crate::features::apply_color(&mut mark, [1., 1., 1., 0.28, 0., 0., 0., 0.]);
                }
                result.push((row, index, mark));
            }
        }
        if let Some([angle, value]) = self.rigging.test_character_pose {
            let layer = &self.doc.layers[self.layer];
            if let Some(piece) = &layer.options.character_piece {
                let mut test = piece.clone();
                test.face_motion = Default::default();
                test.face_keys = Default::default();
                let last = test.lip_poses.len().saturating_sub(1) as f32;
                test.set_motion(
                    0,
                    [
                        if test.face == FacePart::Slider {
                            value * last / 127.
                        } else {
                            value
                        },
                        angle,
                        0.,
                    ],
                );
                if matches!(
                    test.face,
                    FacePart::Hand | FacePart::Toggle | FacePart::Switch
                ) {
                    test.set_variant(0, (value * last).round() as usize);
                }
                let turn = angle * 8.;
                let pose = (self.frame / 8)
                    .saturating_sub(1)
                    .min(test.lip_poses.len().saturating_sub(1));
                let marks = if test.face.control() {
                    control_drawing(layer, &test, 0, turn)
                } else if test.is_lip() {
                    lip_drawing(layer, &test, pose, turn)
                } else if test.face == FacePart::Eye {
                    eye_artwork(layer, &test, turn, 0)
                } else {
                    let drawn = |view: usize| !test.hidden[view] && !layer.frames[view].is_empty();
                    let slot = turn.round().rem_euclid(8.) as usize;
                    drawn_transition(turn, test.shape_transitions, drawn)
                        .and_then(|(a, b, t)| eye_tween(&layer.frames[a], &layer.frames[b], t))
                        .unwrap_or_else(|| layer.frames[slot].clone())
                };
                result.retain(|(row, _, _)| *row != self.layer);
                result.extend(
                    marks
                        .iter()
                        .cloned()
                        .enumerate()
                        .map(|(index, mark)| (self.layer, index, mark)),
                );
            }
        }
        result.sort_by_key(|(row, _, _)| *row);
        result
    }

    pub(crate) fn edits_rigged_eye(&self, layer: usize) -> bool {
        self.in_character_symbol()
            && self.doc.layers[layer]
                .options
                .character_piece
                .as_ref()
                .is_some_and(|piece| piece.face == FacePart::Eye && piece.eye_parent.is_some())
    }

    fn character_face_edit(&self) -> bool {
        self.in_character_symbol()
            && self.doc.layers[self.layer]
                .options
                .character_piece
                .as_ref()
                .is_some_and(|piece| {
                    matches!(piece.face, FacePart::Eye | FacePart::Pupil | FacePart::Brow)
                        || !piece.lip_poses.is_empty()
                })
    }

    pub(crate) fn selected_rig_tracks(&self, selected: &[(usize, usize)]) -> Vec<u64> {
        selected
            .iter()
            .filter_map(|&(layer, index)| {
                let layer = self.doc.layers.get(layer)?;
                let marks = crate::animation::marks(layer, self.frame);
                let mark = marks.get(index)?;
                mark.style.instance.as_ref()?;
                (mark.style.track != 0).then_some(mark.style.track)
            })
            .collect()
    }

    pub(crate) fn selected_character_path(&self) -> Option<Vec<u64>> {
        if self.tool != crate::canvas::Tool::Select || self.in_character_symbol() {
            return None;
        }
        let rig = &self.doc.extras.rig;
        if let Some(id) = rig.character_symbol
            && self
                .selected_rig_tracks(&self.canvas.selected)
                .iter()
                .any(|track| rig.bindings.iter().any(|binding| binding.target == *track))
        {
            return Some(vec![id]);
        }
        fn find(
            symbols: &[Symbol],
            instance: &crate::features::Instance,
            frame: usize,
            depth: usize,
            visited: &mut std::collections::HashSet<u64>,
        ) -> Option<Vec<u64>> {
            let id = instance.symbol;
            if depth == 0 || !visited.insert(id) {
                return None;
            }
            let symbol = symbols.iter().find(|symbol| symbol.id == id)?;
            if let Some(character) = symbol.rig.character_symbol {
                return Some(vec![id, character]);
            }
            let frame = crate::features::symbol_frame(
                instance,
                symbol.layers.first()?.frames.len(),
                frame,
                frame,
            );
            for layer in &symbol.layers {
                for mark in &crate::animation::marks(layer, frame) {
                    if let Some(instance) = &mark.style.instance
                        && let Some(mut path) = find(symbols, instance, frame, depth - 1, visited)
                    {
                        path.insert(0, id);
                        return Some(path);
                    }
                }
            }
            None
        }
        let depth = 8usize.saturating_sub(self.feature.symbol_stack.len());
        for &(layer, index) in &self.canvas.selected {
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
            if rig.character_symbol == Some(instance.symbol) {
                return Some(vec![instance.symbol]);
            }
            if let Some(path) = find(
                &self.doc.extras.symbols,
                instance,
                self.frame,
                depth,
                &mut Default::default(),
            ) && path.len() <= depth
            {
                return Some(path);
            }
        }
        None
    }

    pub(crate) fn onion_frames(&self) -> std::ops::Range<usize> {
        let start = if self.in_character_symbol() {
            self.frame / 8 * 8
        } else {
            0
        };
        let end = if self.in_character_symbol() {
            start + 8
        } else {
            self.doc.layers[0].frames.len()
        };
        self.frame
            .saturating_sub(self.studio.onion_before)
            .max(start)
            ..self
                .frame
                .saturating_add(self.studio.onion_after)
                .saturating_add(1)
                .min(end)
    }

    pub(crate) fn face_controls(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.doc.extras.rig.character_symbol else {
            return;
        };
        let Some(index) = self
            .doc
            .extras
            .symbols
            .iter()
            .position(|symbol| symbol.id == id)
        else {
            return;
        };
        let frame = self.group_rig_frame();
        let mut edits = Vec::new();
        let mut motion_edits = Vec::new();
        egui::CollapsingHeader::new(tr("Face Controls"))
            .default_open(true)
            .show(ui, |ui| {
                let link_id = ui.scope_id().with("link-brows");
                let mut linked = ui
                    .data(|data| data.get_temp::<bool>(link_id))
                    .unwrap_or(false);
                if button(ui, Icon::Bones, "Link brows", "", linked, true).clicked() {
                    linked ^= true;
                    ui.data_mut(|data| data.insert_temp(link_id, linked));
                }
                let mut expression = None;
                let presets = button(ui, Icon::Mouth, "Expressions", "", false, true);
                egui::Popup::menu(&presets).show(|ui| {
                    for (name, brow, eye) in [
                        ("Neutral", [0., 0., 0.], 0),
                        ("Angry", [-0.8, 0.55, 0.], 1),
                        ("Surprised", [0., -0.7, 0.85], 0),
                    ] {
                        if ui.button(tr(name)).clicked() {
                            expression = Some((brow, eye));
                            ui.close();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    for (row, layer) in self.doc.extras.symbols[index].layers.iter().enumerate() {
                        let Some(piece) = layer
                            .options
                            .character_piece
                            .as_ref()
                            .filter(|piece| piece.face == FacePart::Pupil)
                        else {
                            continue;
                        };
                        ui.push_id(row, |ui| {
                            ui.vertical(|ui| {
                                ui.small(&layer.name);
                                let axes = piece.motion(frame);
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(62., 62.),
                                    egui::Sense::click_and_drag(),
                                );
                                let radius = rect.width() / 2. - 6.;
                                let center = rect.center();
                                ui.painter().circle_stroke(
                                    center,
                                    radius,
                                    egui::Stroke::new(1., crate::themes::muted()),
                                );
                                ui.painter().circle_filled(
                                    center + egui::vec2(axes[0], axes[1]) * radius,
                                    5.,
                                    crate::themes::accent(),
                                );
                                if let Some(pointer) = response.interact_pointer_pos()
                                    && (response.clicked() || response.dragged())
                                {
                                    let offset = (pointer - center) / radius;
                                    let offset = offset / offset.length().max(1.);
                                    motion_edits.push((row, [offset.x, offset.y, 0.]));
                                }
                            })
                        });
                    }
                });
                for (row, layer) in self.doc.extras.symbols[index].layers.iter().enumerate() {
                    let Some(piece) = &layer.options.character_piece else {
                        continue;
                    };
                    let variants = match piece.face {
                        FacePart::Eye => EYES,
                        FacePart::Brow => BROWS,
                        _ => continue,
                    };
                    ui.push_id(row, |ui| {
                        if piece.face == FacePart::Brow {
                            ui.label(&layer.name);
                            let change = brow_axes_control(ui, piece.motion(frame));
                            let mut changed = change.is_some();
                            let mut axes =
                                change.map_or_else(|| piece.motion(frame), |(axes, _)| axes);
                            if let Some((brow, _)) = expression {
                                axes = brow;
                                changed = true;
                            }
                            if changed {
                                if linked && expression.is_none() {
                                    motion_edits.extend(
                                        self.doc.extras.symbols[index]
                                            .layers
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, layer)| {
                                                layer
                                                    .options
                                                    .character_piece
                                                    .as_ref()
                                                    .is_some_and(|p| p.face == FacePart::Brow)
                                            })
                                            .map(|(row, _)| (row, axes)),
                                    );
                                } else {
                                    motion_edits.push((row, axes));
                                }
                            }
                        } else if let Some((_, eye)) = expression {
                            edits.push((row, Some(eye)));
                        }
                        if piece.eye_parent.is_some() {
                            let mut axes = piece.motion(frame);
                            if eye_closure_slider(ui, &mut axes[0]).changed() {
                                motion_edits.push((row, axes));
                            }
                            return;
                        }
                        if piece.face == FacePart::Eye {
                            let mut closure = piece.variant(frame).min(2) as f32 * 0.5;
                            if eye_closure_slider(ui, &mut closure).changed() {
                                let state = (closure * 2.).round() as usize;
                                if state != piece.variant(frame) {
                                    edits.push((row, Some(state)));
                                }
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label(&layer.name);
                            let value = piece.variant(frame).min(variants.len() - 1);
                            egui::ComboBox::from_id_salt("expression")
                                .selected_text(tr(variants[value]))
                                .show_ui(ui, |ui| {
                                    for (variant, label) in variants.iter().enumerate() {
                                        if ui
                                            .selectable_label(variant == value, tr(label))
                                            .clicked()
                                        {
                                            edits.push((row, Some(variant)));
                                        }
                                    }
                                });
                            if button(
                                ui,
                                Icon::KeyRemove,
                                "",
                                "Delete expression key",
                                false,
                                piece.face_keys.iter().any(|key| key.frame == frame),
                            )
                            .clicked()
                            {
                                edits.push((row, None));
                            }
                        });
                    });
                }
            });
        let gesture_id = ui.scope_id().with("face-motion-gesture");
        let continuing = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(gesture_id))
            .unwrap_or(false);
        let dragging = ui.input(|input| input.pointer.primary_down());
        if !edits.is_empty() || !motion_edits.is_empty() {
            let selected = self.canvas.selected.clone();
            if !continuing || !edits.is_empty() {
                self.checkpoint();
            }
            self.canvas.selected = selected;
            for (row, variant) in edits {
                let piece = self.doc.extras.symbols[index].layers[row]
                    .options
                    .character_piece
                    .as_mut()
                    .unwrap();
                if let Some(variant) = variant {
                    piece.set_variant(frame, variant);
                } else {
                    piece.face_keys.retain(|key| key.frame != frame);
                }
            }
            for (row, axes) in motion_edits {
                if let Some(piece) = self.doc.extras.symbols[index].layers[row]
                    .options
                    .character_piece
                    .as_mut()
                {
                    piece.set_motion(frame, axes);
                }
            }
            if dragging {
                ui.ctx().data_mut(|data| data.insert_temp(gesture_id, true));
            } else {
                self.recover();
            }
        }
        if continuing && !dragging {
            ui.ctx().data_mut(|data| data.remove::<bool>(gesture_id));
            self.recover();
        }
    }

    pub(crate) fn face_rig_panel(&mut self, ctx: &egui::Context, area: egui::Rect) {
        if self.tool != crate::canvas::Tool::Select || self.playing || self.in_character_symbol() {
            return;
        }
        let rig = &self.doc.extras.rig;
        let Some(id) = rig.character_symbol else {
            return;
        };
        let tracks = self.selected_rig_tracks(&self.canvas.selected);
        if !rig
            .bindings
            .iter()
            .any(|binding| tracks.contains(&binding.target))
        {
            return;
        }
        let Some(symbol) = self
            .doc
            .extras
            .symbols
            .iter()
            .find(|symbol| symbol.id == id)
        else {
            return;
        };
        let has_control = symbol
            .layers
            .iter()
            .filter(|layer| layer.visible)
            .any(|layer| {
                layer.options.character_piece.as_ref().is_some_and(|piece| {
                    piece.face.control()
                        && piece
                            .eye_parent
                            .is_some_and(|track| tracks.contains(&track))
                })
            });
        let is_face = symbol.layers.iter().any(|layer| {
            layer.options.character_piece.as_ref().is_some_and(|piece| {
                matches!(piece.face, FacePart::Eye | FacePart::Brow | FacePart::Pupil)
                    && (!has_control
                        || piece
                            .eye_parent
                            .is_some_and(|track| tracks.contains(&track)))
            })
        });
        if !has_control && !is_face {
            return;
        }
        egui::Window::new(tr(if is_face { "Face Rig" } else { "Rig Controls" }))
            .id(egui::Id::unique("scene-face-rig"))
            .default_pos(area.left_top() + egui::vec2(12., 12.))
            .default_size(egui::vec2(
                (area.width() * 0.30).clamp(220., 320.),
                (area.height() * 0.80).clamp(200., 460.),
            ))
            .min_size(egui::vec2(160., 120.))
            .movable(true)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if has_control {
                            self.scene_custom_controls(ui, id, &tracks);
                        }
                        if is_face {
                            self.scene_eye_controls(ui, id);
                        }
                    });
            });
    }

    fn scene_custom_controls(&mut self, ui: &mut egui::Ui, id: u64, tracks: &[u64]) {
        let symbol_index = self
            .doc
            .extras
            .symbols
            .iter()
            .position(|symbol| symbol.id == id)
            .unwrap();
        let frame = self.group_rig_frame();
        let mut edit = None;
        let icon = Icon::Puppet;
        for (row, layer) in self.doc.extras.symbols[symbol_index]
            .layers
            .iter()
            .enumerate()
        {
            let Some(piece) = layer.options.character_piece.as_ref().filter(|piece| {
                layer.visible
                    && piece.face.control()
                    && !piece.lip_poses.is_empty()
                    && piece
                        .eye_parent
                        .is_some_and(|track| tracks.contains(&track))
            }) else {
                continue;
            };
            ui.push_id(piece.track, |ui| {
                let mut axes = piece.motion(frame);
                let pose = piece.variant(frame);
                character_row_icon(
                    ui,
                    if piece.face == FacePart::Hand {
                        icon
                    } else {
                        Icon::Adjust
                    },
                    &layer.name,
                );
                match piece.face {
                    FacePart::Toggle => {
                        let mut on = pose == 0;
                        if ui.checkbox(&mut on, tr("On")).changed() {
                            edit = Some((row, Some(usize::from(!on)), axes));
                        }
                    }
                    FacePart::Slider => {
                        let range = (piece.lip_poses.len() - 1).max(1) as f32;
                        let mut value = (axes[0] * 127. / range).clamp(0., 1.);
                        if rig_slider(ui, &mut value, piece.lip_poses.len(), "Rig pose").changed() {
                            axes[0] = value * range / 127.;
                            edit = Some((row, None, axes));
                        }
                        ui.small(
                            &piece.lip_poses
                                [((value * range).round() as usize).min(piece.lip_poses.len() - 1)]
                            .name,
                        );
                    }
                    FacePart::Hand | FacePart::Switch => {
                        if piece.face == FacePart::Hand {
                            let mut angle = axes[1];
                            if rig_slider(ui, &mut angle, 9, "Hand turnaround").changed() {
                                axes[1] = angle;
                                edit = Some((row, None, axes));
                            }
                            ui.small(tr(HAND_VIEWS[(angle * 8.).round() as usize % 8]));
                        }
                        ui.horizontal_wrapped(|ui| {
                            for (index, entry) in piece.lip_poses.iter().enumerate() {
                                let clicked = if piece.face == FacePart::Hand {
                                    pose_thumbnail(
                                        ui,
                                        || lip_drawing(layer, piece, index, axes[1] * 8.),
                                        &entry.name,
                                        pose == index,
                                    )
                                } else {
                                    button(ui, Icon::Keyframe, &entry.name, "", pose == index, true)
                                        .clicked()
                                };
                                if clicked {
                                    edit = Some((row, Some(index), axes));
                                }
                            }
                        });
                    }
                    _ => {}
                }
                ui.separator();
            });
        }
        let gesture = ui.scope_id().with("custom-rig-gesture");
        let dragging = ui.input(|input| input.pointer.primary_down());
        let active = ui
            .data(|data| data.get_temp::<bool>(gesture))
            .unwrap_or(false);
        if let Some((row, pose, axes)) = edit {
            if !active || pose.is_some() {
                let selected = self.canvas.selected.clone();
                self.checkpoint();
                self.canvas.selected = selected;
            }
            let piece = self.doc.extras.symbols[symbol_index].layers[row]
                .options
                .character_piece
                .as_mut()
                .unwrap();
            if let Some(pose) = pose {
                piece.set_variant(frame, pose);
            } else {
                piece.set_motion(frame, axes);
            }
            if dragging {
                ui.data_mut(|data| data.insert_temp(gesture, true));
            } else {
                self.recover();
            }
        }
        if active && !dragging {
            ui.data_mut(|data| data.remove::<bool>(gesture));
            self.recover();
        }
    }

    fn scene_eye_controls(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(index) = self
            .doc
            .extras
            .symbols
            .iter()
            .position(|symbol| symbol.id == id)
        else {
            return;
        };
        let frame = self.group_rig_frame();
        let mut edit = None;
        let mut began = false;
        let mode = egui::Id::unique(("linked-pupil-controls", id));
        let mut linked = ui.data(|data| data.get_temp::<bool>(mode)).unwrap_or(false);
        let pupils: Vec<_> = self.doc.extras.symbols[index]
            .layers
            .iter()
            .enumerate()
            .filter_map(|(row, layer)| {
                let piece = layer.options.character_piece.as_ref()?;
                (piece.face == FacePart::Pupil).then_some((row, &layer.name, piece.motion(frame)))
            })
            .collect();
        if !pupils.is_empty() {
            ui.horizontal(|ui| {
                ui.strong(tr("Pupils"));
                if pupils.len() > 1 && ui.checkbox(&mut linked, tr("Move together")).changed() {
                    ui.data_mut(|data| data.insert_temp(mode, linked));
                }
            });
            ui.horizontal_wrapped(|ui| {
                for &(row, name, axes) in &pupils {
                    ui.push_id(row, |ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(64., 56.),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.set_width(64.);
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(42., 42.),
                                    egui::Sense::click_and_drag(),
                                );
                                let radius = 17.;
                                let center = rect.center();
                                ui.painter().circle_stroke(
                                    center,
                                    radius,
                                    egui::Stroke::new(1., crate::themes::muted()),
                                );
                                ui.painter().circle_filled(
                                    center + egui::vec2(axes[0], axes[1]) * radius,
                                    4.,
                                    egui::Color32::from_rgb(105, 220, 151),
                                );
                                if let Some(pointer) = response.interact_pointer_pos()
                                    && (response.clicked()
                                        || response.dragged()
                                        || response.drag_stopped())
                                {
                                    let delta = (pointer - center) / radius;
                                    let delta = delta / delta.length().max(1.);
                                    let value = [delta.x, delta.y, 0.];
                                    if value != axes
                                        || (linked
                                            && pupils.iter().any(|(_, _, axes)| *axes != value))
                                    {
                                        edit = Some((row, value));
                                        began = response.drag_started() || response.clicked();
                                    }
                                }
                                response.on_hover_text(name);
                                ui.add(
                                    egui::Label::new(egui::RichText::new(name).small()).truncate(),
                                )
                                .on_hover_text(name);
                            },
                        );
                    });
                }
            });
            ui.separator();
        }
        for (row, layer) in self.doc.extras.symbols[index].layers.iter().enumerate() {
            let Some(piece) = layer
                .options
                .character_piece
                .as_ref()
                .filter(|piece| matches!(piece.face, FacePart::Eye | FacePart::Brow))
            else {
                continue;
            };
            ui.push_id(row, |ui| {
                if piece.face == FacePart::Brow {
                    character_row_icon(ui, Icon::Pen, &layer.name);
                    if let Some((axes, drag_started)) = brow_axes_control(ui, piece.motion(frame)) {
                        edit = Some((row, axes));
                        began = drag_started;
                    }
                    ui.add_space(6.);
                    return;
                }

                character_row_icon(ui, Icon::Eye, &layer.name);
                let mut value = if piece.eye_parent.is_some() {
                    piece.motion(frame)[0]
                } else {
                    piece.variant(frame).min(2) as f32 * 0.5
                };
                let response = eye_closure_slider(ui, &mut value);
                if response.changed() {
                    edit = Some((row, [value, 0., 0.]));
                    began = response.drag_started() || !response.dragged();
                }
                ui.horizontal(|ui| {
                    ui.small(tr("Open"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.small(tr("Closed"));
                    });
                });
                ui.add_space(6.);
            });
        }
        let gesture = ui.scope_id().with("eye-closure-gesture");
        let active = ui
            .data(|data| data.get_temp::<bool>(gesture))
            .unwrap_or(false);
        if let Some((row, value)) = edit {
            if began || !active {
                let selection = self.canvas.selected.clone();
                self.checkpoint();
                self.canvas.selected = selection;
            }
            let layers = &mut self.doc.extras.symbols[index].layers;
            let together = linked
                && layers[row]
                    .options
                    .character_piece
                    .as_ref()
                    .is_some_and(|piece| piece.face == FacePart::Pupil);
            if together {
                for layer in layers {
                    if let Some(piece) = layer
                        .options
                        .character_piece
                        .as_mut()
                        .filter(|piece| piece.face == FacePart::Pupil)
                    {
                        piece.set_motion(frame, value);
                    }
                }
            } else {
                let piece = layers[row].options.character_piece.as_mut().unwrap();
                if piece.eye_parent.is_some()
                    || matches!(piece.face, FacePart::Pupil | FacePart::Brow)
                {
                    piece.set_motion(frame, value);
                } else {
                    piece.set_variant(frame, (value[0] * 2.).round() as usize);
                }
            }
            if ui.input(|i| i.pointer.primary_down()) {
                ui.data_mut(|data| data.insert_temp(gesture, true));
            } else {
                self.recover();
            }
        }
        if active && !ui.input(|i| i.pointer.primary_down()) {
            ui.data_mut(|data| data.remove::<bool>(gesture));
            self.recover();
        }
    }

    pub(crate) fn character_timeline(&mut self, ui: &mut egui::Ui) {
        struct CharacterLayerDrag(usize);
        ui.set_min_height(ui.available_height());
        ui.horizontal(|ui| {
            let add = button(
                ui,
                Icon::LayerAdd,
                "Add Rigged Layer",
                "",
                false,
                self.doc.layers.len() < 100,
            );
            egui::Popup::menu(&add).show(|ui| {
                for (kind, label) in [
                    (FacePart::Eye, "Eye"),
                    (FacePart::Pupil, "Pupil"),
                    (FacePart::Brow, "Brow"),
                    (FacePart::Hand, "Hand"),
                    (FacePart::Slider, "Slider Rig"),
                    (FacePart::Toggle, "On/Off Rig"),
                    (FacePart::Switch, "Switch Rig"),
                ] {
                    ui.menu_button(tr(label), |ui| {
                        ui.label(tr("Attach to bone"));
                        let targets = self.rigged_layer_targets();
                        if targets.is_empty() {
                            ui.label(tr("Bind artwork to a bone first."));
                        }
                        for (track, name) in targets {
                            if button(ui, Icon::Bones, &name, "", false, true).clicked() {
                                self.add_rigged_piece(kind, track, None);
                                ui.close();
                            }
                        }
                    });
                }
                ui.menu_button(tr("Lip Sync"), |ui| {
                    let sources: Vec<_> = self
                        .doc
                        .extras
                        .symbols
                        .iter()
                        .filter(|symbol| !symbol.mouth_poses.is_empty())
                        .map(|symbol| (symbol.id, symbol.name.clone()))
                        .collect();
                    if sources.is_empty() {
                        ui.label(tr("Create a Lip Sync symbol first."));
                    }
                    let targets = self.rigged_layer_targets();
                    for (source, name) in sources {
                        ui.menu_button(name, |ui| {
                            ui.label(tr("Attach to bone"));
                            if targets.is_empty() {
                                ui.label(tr("Bind artwork to a bone first."));
                            }
                            for (track, name) in &targets {
                                if button(ui, Icon::Bones, name, "", false, true).clicked() {
                                    self.add_rigged_piece(FacePart::Mouth, *track, Some(source));
                                    ui.close();
                                }
                            }
                        });
                    }
                });
            });
            if self.doc.layers[self.layer]
                .options
                .character_piece
                .as_ref()
                .is_some_and(|p| p.face == FacePart::Pupil)
            {
                let id = egui::Id::unique("pupil-area-tool");
                let enabled = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
                if button(
                    ui,
                    Icon::Rectangle,
                    "Area Tool",
                    "Drag from the center to set the pupil travel radius",
                    enabled,
                    true,
                )
                .clicked()
                {
                    ui.data_mut(|d| d.insert_temp(id, !enabled));
                }
            }
            if let Some(tool) = self.wrist_tool() {
                let active = ui.data(|data| data.get_temp::<bool>(tool)).unwrap_or(false);
                if button(
                    ui,
                    Icon::Focus,
                    "Wrist point",
                    "Pick the same wrist joint in each drawing",
                    active,
                    true,
                )
                .clicked()
                {
                    ui.data_mut(|data| data.insert_temp(tool, !active));
                }
            }
            if let Some(tool) = self.brow_point_tool() {
                ui.separator();
                ui.label(tr("Brow points"));
                let active = ui.data(|data| data.get_temp::<usize>(tool));
                for (axis, (label, short)) in
                    BROW_POINTS.into_iter().zip(["L", "C", "R"]).enumerate()
                {
                    if button(ui, Icon::Node, short, label, active == Some(axis), true).clicked() {
                        ui.data_mut(|data| {
                            if active == Some(axis) {
                                data.remove::<usize>(tool);
                            } else {
                                data.insert_temp(tool, axis);
                            }
                        });
                    }
                }
            }
            if button(ui, Icon::Onion, "", "Onion skin", self.onion, true).clicked() {
                self.onion ^= true;
            }
            if button(
                ui,
                Icon::Layers,
                "",
                "Layer objects",
                self.studio.layer_objects_open,
                true,
            )
            .clicked()
            {
                self.studio.layer_objects_open ^= true;
            }
            ui.add(
                egui::DragValue::new(&mut self.studio.onion_before)
                    .range(0..=7)
                    .prefix("‹ "),
            )
            .on_hover_text(tr("Previous views"));
            ui.add(
                egui::DragValue::new(&mut self.studio.onion_after)
                    .range(0..=7)
                    .prefix("› "),
            )
            .on_hover_text(tr("Next views"));
        });
        ui.separator();
        let mut selection = None;
        let mut transition = None;
        let mut vertical = None;
        let mut vertical_all = None;
        let mut copy_eye = None;
        let mut copy_mouth_view = None;
        let mut lip_transition = None;
        let mut new_pose = None;
        let mut hand_tween = None;
        let mut rename = None;
        let mut reorder = None;
        let mut attachment = None;
        let mut delete = None;
        let mut pupil_mask = None;
        let mut mirror = None;
        let mut duplicate_side = None;
        let mut renamed = None;
        let rename_id = ui.scope_id().with("character-row-rename");
        let mut draft = ui.data(|data| data.get_temp::<(usize, String)>(rename_id));
        egui::ScrollArea::both()
            .id_salt("character-view-grid")
            .show(ui, |ui| {
                egui::Grid::new("character-views")
                    .striped(true)
                    .min_col_width(82.)
                    .show(ui, |ui| {
                        character_row_icon(ui, Icon::Head, tr("Character Symbol"));
                        let views = if self.doc.layers[self.layer].options.character_piece.as_ref().is_some_and(|p| p.face == FacePart::Hand) { HAND_VIEWS } else { VIEWS };
                        for label in views {
                            ui.strong(tr(label));
                        }
                        ui.end_row();
                        for (index, layer) in self.doc.layers.iter().enumerate().rev() {
                            let eye = layer.options.character_piece.as_ref().filter(|p| p.face == FacePart::Eye && layer.frames.len() >= 32);
                            let mouth = layer.options.character_piece.as_ref().filter(|piece| !piece.lip_poses.is_empty());
                            let identity = layer.options.character_piece.as_ref().map_or(index as u64, |p| p.track);
                            let fold = ui.scope_id().with(("eye-fold", identity));
                            let mut collapsed = ui.data(|data| data.get_temp::<bool>(fold)).unwrap_or(mouth.is_some());
                            let title = ui.horizontal(|ui| {
                                if (eye.is_some() || mouth.is_some()) && ui.small_button(if collapsed { "▸" } else { "▾" }).clicked() {
                                    collapsed ^= true;
                                    ui.data_mut(|data| data.insert_temp(fold, collapsed));
                                }
                                if let Some((row, name)) = draft.as_mut().filter(|(row, _)| *row == index) {
                                    let response = ui.add(egui::TextEdit::singleline(name).desired_width(100.).char_limit(128));
                                    response.request_focus();
                                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) || response.lost_focus() {
                                        renamed = Some((*row, name.trim().to_owned()));
                                    }
                                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                        renamed = Some((*row, String::new()));
                                    }
                                } else {
                                    let (icon, color) = match layer.options.character_piece.as_ref().map(|p| p.face) {
                                        Some(FacePart::Eye) => (Icon::Eye, egui::Color32::from_rgb(245, 202, 102)),
                                        Some(FacePart::Mouth) => (Icon::Mouth, egui::Color32::from_rgb(238, 157, 184)),
                                        Some(FacePart::Hand) => (Icon::Puppet, egui::Color32::from_rgb(117, 181, 255)),
                                        Some(FacePart::Slider) => (Icon::Adjust, egui::Color32::from_rgb(117, 181, 255)),
                                        Some(FacePart::Toggle | FacePart::Switch) => (Icon::Layers, egui::Color32::from_rgb(117, 181, 255)),
                                        Some(FacePart::Pupil) => (Icon::Focus, egui::Color32::from_rgb(105, 220, 151)),
                                        Some(FacePart::Brow) => (Icon::Pen, egui::Color32::from_rgb(245, 202, 102)),
                                        _ => (Icon::Bones, egui::Color32::from_rgb(117, 181, 255)),
                                    };
                                    let response = ui.horizontal(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(15.), egui::Sense::hover());
                                        crate::design::draw(ui.painter(), rect, icon, color);
                                        ui.add(egui::Label::new(egui::RichText::new(&layer.name).color(color)).sense(egui::Sense::click_and_drag()))
                                    }).inner.on_hover_cursor(egui::CursorIcon::Grab);
                                    response.dnd_set_drag_payload(CharacterLayerDrag(index));
                                    if response.double_clicked() {
                                        rename = Some((index, layer.name.clone()));
                                    } else if response.clicked() {
                                        let view = self.frame % 8;
                                        let offset = if let Some(piece) = mouth {
                                            (if index == self.layer { (self.frame / 8).clamp(1, piece.lip_poses.len()) } else { 1 }) * 8
                                        } else if eye.is_some() { (self.frame / 8).clamp(1, 3) * 8 } else { 0 };
                                        selection = Some((index, offset + view));
                                    }
                                    response.context_menu(|ui| {
                                        if layer.options.character_piece.as_ref().is_some_and(|piece| {
                                            piece.eye_parent.is_some() && matches!(piece.face, FacePart::Eye | FacePart::Pupil | FacePart::Brow)
                                        }) && (layer.name.ends_with("_R") || layer.name.ends_with("_L"))
                                            && button(ui, Icon::Mirror, "Duplicate Side", "Mirror the matching eye, pupil, and brow", false, true).clicked() {
                                            duplicate_side = Some(index);
                                            ui.close();
                                        }
                                        if let Some(piece) = mouth {
                                            let view = self.frame % 8;
                                            let labels = if piece.face == FacePart::Hand { HAND_VIEWS } else { VIEWS };
                                            if piece.face.control() && piece.face != FacePart::Toggle && piece.lip_poses.len() < 128 {
                                                ui.menu_button(tr("Add pose"), |ui| {
                                                    let id = ui.scope_id().with(piece.track);
                                                    let mut name = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_default();
                                                    let field = ui.add(egui::TextEdit::singleline(&mut name).char_limit(48).hint_text(tr("Pose name")));
                                                    let text = name.trim();
                                                    let valid = !text.is_empty() && text.len() <= 48 && !piece.lip_poses.iter().any(|pose| pose.name == text);
                                                    if button(ui, Icon::Plus, "Add pose", "", false, valid).clicked()
                                                        || (valid && field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                                        new_pose = Some((index, text.to_owned()));
                                                        name.clear();
                                                        ui.close();
                                                    }
                                                    ui.data_mut(|d| d.insert_temp(id, name));
                                                });
                                            }
                                            if piece.face == FacePart::Hand {
                                                if button(ui, Icon::Tween, "Tween hand poses", "Interpolate between compatible pose keys in the scene", piece.tween_hand_poses, true).clicked() {
                                                    hand_tween = Some(index);
                                                    ui.close();
                                                }
                                                ui.menu_button(tr("Mirror to other hand"), |ui| {
                                                    for (track, label) in self.rigged_layer_targets() {
                                                        if Some(track) != piece.eye_parent && button(ui, Icon::Bones, &label, "", false, self.doc.layers.len() < 100).clicked() {
                                                            mirror = Some((index, true, Some(track)));
                                                            ui.close();
                                                        }
                                                    }
                                                });
                                            }
                                            ui.menu_button(format!("{} {}", tr("Copy all poses to"), tr(labels[view])), |ui| {
                                                for (source, name) in VIEWS.into_iter().enumerate() {
                                                    let drawn = (1..=piece.lip_poses.len()).any(|state| !layer.frames[state * 8 + source].is_empty());
                                                    if source != view && button(ui, Icon::Copy, name, "Fill empty poses in the current view", false, drawn).clicked() {
                                                        copy_mouth_view = Some((index, view, source));
                                                        ui.close();
                                                    }
                                                }
                                            });
                                        }
                                        if layer.options.character_piece.as_ref().is_some_and(|piece| {
                                            piece.eye_parent.is_some() && matches!(piece.face, FacePart::Eye | FacePart::Brow)
                                        }) {
                                            if button(ui, Icon::Mirror, "Mirror layer", "Reflect every drawing and keep its view column", false, self.doc.layers.len() < 100).clicked() {
                                                mirror = Some((index, false, None));
                                                ui.close();
                                            }
                                            if button(ui, Icon::Mirror, "Mirror to opposite views", "Reflect the drawings and swap left and right view columns", false, self.doc.layers.len() < 100).clicked() {
                                                mirror = Some((index, true, None));
                                                ui.close();
                                            }
                                        }
                                        if eye.is_some()
                                            && button(ui, Icon::Node, "Enable Vertical Tweens in drawn views", "Enable compatible blink transitions in every drawn view", false, true).clicked()
                                        {
                                            vertical_all = Some(index);
                                            ui.close();
                                        }
                                        if let Some(parent_track) = layer.options.character_piece.as_ref().and_then(|piece| piece.eye_parent) {
                                            ui.menu_button(tr("Attach to"), |ui| {
                                                for target in &self.doc.layers {
                                                    let Some(piece) = target.options.character_piece.as_ref().filter(|piece| piece.eye_parent.is_none()) else { continue };
                                                    if ui.selectable_label(parent_track == piece.track, &target.name).clicked() {
                                                        attachment = Some((index, piece.track, piece.matrix));
                                                        ui.close();
                                                    }
                                                }
                                            });
                                        }
                                        if let Some(pupil) = layer.options.character_piece.as_ref().filter(|piece| piece.face == FacePart::Pupil && piece.eye_parent.is_some()) {
                                            ui.menu_button(tr("Clip to eye"), |ui| {
                                                if ui.selectable_label(pupil.eye_mask.is_none(), tr("None")).clicked() {
                                                    pupil_mask = Some((index, None));
                                                    ui.close();
                                                }
                                                for eye in &self.doc.layers {
                                                    if let Some(piece) = eye.options.character_piece.as_ref().filter(|piece| piece.face == FacePart::Eye && piece.eye_parent.is_some())
                                                        && ui.selectable_label(pupil.eye_mask == Some(piece.track), &eye.name).clicked()
                                                    {
                                                        pupil_mask = Some((index, Some(piece.track)));
                                                        ui.close();
                                                    }
                                                }
                                            });
                                        }
                                        if layer.options.character_piece.as_ref().is_some_and(|piece| piece.eye_parent.is_some())
                                            && button(ui, Icon::Trash, "Delete rigged layer", "", false, true).clicked()
                                        {
                                            delete = Some(index);
                                            ui.close();
                                        }
                                        if button(ui, Icon::StepBack, "Move up", "", false, index + 1 < self.doc.layers.len()).clicked() {
                                            reorder = Some((index, index + 1)); ui.close();
                                        }
                                        if button(ui, Icon::StepFront, "Move down", "", false, index > 0).clicked() {
                                            reorder = Some((index, index - 1)); ui.close();
                                        }
                                    });
                                }
                            }).response;
                            if let Some(source) = title.dnd_hover_payload::<CharacterLayerDrag>()
                                && let Some(pointer) = ui.ctx().pointer_interact_pos()
                            {
                                let before = pointer.y < title.rect.center().y;
                                let slot = index + usize::from(before);
                                let destination = slot - usize::from(source.0 < slot);
                                if source.0 != destination {
                                    let y = if before { title.rect.top() } else { title.rect.bottom() };
                                    ui.painter().hline(title.rect.x_range(), y, egui::Stroke::new(2., crate::themes::accent()));
                                    if title.dnd_release_payload::<CharacterLayerDrag>().is_some() {
                                        reorder = Some((source.0, destination));
                                    }
                                }
                            }
                            if let Some(piece) = mouth {
                                ui.end_row();
                                if collapsed { continue; }
                                for (pose, name) in piece.lip_poses.iter().enumerate() {
                                    ui.label(format!("    {}", name.name));
                                    let pose_views = if piece.face == FacePart::Hand { HAND_VIEWS } else { VIEWS };
                                    for (view, view_name) in pose_views.into_iter().enumerate() {
                                        let offset = (pose + 1) * 8;
                                        let frame = offset + view;
                                        let drawn = !layer.frames[frame].is_empty();
                                        let tween = piece.lip_tweens[pose] & (1 << view) != 0;
                                        let icon = if drawn { Icon::Keyframe } else { Icon::BlankKeyAdd };
                                        let cell = button(ui, icon, if tween { "→" } else { "" }, view_name,
                                            self.layer == index && self.frame == frame, true);
                                        if cell.clicked() { selection = Some((index, frame)); }
                                        cell.context_menu(|ui| {
                                            ui.menu_button(tr("Copy pose from"), |ui| {
                                                for (source, name) in pose_views.into_iter().enumerate() {
                                                    let source_frame = offset + source;
                                                    if source != view && button(ui, Icon::Copy, name, "", false, !layer.frames[source_frame].is_empty()).clicked() {
                                                        copy_eye = Some((index, frame, source_frame));
                                                        ui.close();
                                                    }
                                                }
                                            });
                                            ui.menu_button(tr("Copy all poses from"), |ui| {
                                                for (source, name) in pose_views.into_iter().enumerate() {
                                                    let drawn = (1..=piece.lip_poses.len()).any(|state| !layer.frames[state * 8 + source].is_empty());
                                                    if source != view && button(ui, Icon::Copy, name, "Fill empty poses in this view", false, drawn).clicked() {
                                                        copy_mouth_view = Some((index, view, source));
                                                        ui.close();
                                                    }
                                                }
                                            });
                                            if button(ui, Icon::Tween, "Horizontal Shape Tween", "Interpolate to the next drawing of this pose", tween, drawn).clicked() {
                                                lip_transition = Some((index, pose, view));
                                                ui.close();
                                            }
                                        });
                                    }
                                    ui.end_row();
                                }
                                continue;
                            }
                            if let Some(piece) = eye {
                                ui.end_row();
                                if collapsed { continue; }
                                for (state, name) in EYES[..3].iter().enumerate() {
                                    ui.label(format!("    {}", tr(name)));
                                    for view in 0..8 {
                                        let frame = (state + 1) * 8 + view;
                                        let linked = piece.vertical_tweens[view] & (1 << state.min(1)) != 0;
                                        let icon = if layer.frames[frame].is_empty() { Icon::BlankKeyAdd } else { Icon::Keyframe };
                                        let horizontal = piece.shape_transitions & (1 << view) != 0;
                                        let arrows = match (linked, horizontal) { (true, true) => "↓ →", (true, false) => "↓", (false, true) => "→", _ => "" };
                                        let cell = button(ui, icon, arrows, VIEWS[view], self.layer == index && self.frame == frame, true);
                                        if cell.clicked() { selection = Some((index, frame)); }
                                        cell.context_menu(|ui| {
                                            for (source, label) in EYES[..3].iter().enumerate() {
                                                let source_frame = (source + 1) * 8 + view;
                                                if source != state && button(ui, Icon::Copy, &format!("{} {}", tr("Copy from"), tr(label)), "", false, !layer.frames[source_frame].is_empty()).clicked() {
                                                    copy_eye = Some((index, frame, source_frame));
                                                    ui.close();
                                                }
                                            }
                                            let open = &layer.frames[8 + view];
                                            let half = &layer.frames[16 + view];
                                            let closed = &layer.frames[24 + view];
                                            let (from, to) = if half.is_empty() {
                                                (open, closed)
                                            } else if state == 0 {
                                                (open, half)
                                            } else {
                                                (half, closed)
                                            };
                                            let compatible = compatible_eye_tween(from, to);
                                            let hint = if from.is_empty() || to.is_empty() {
                                                "Draw both eye states in this view."
                                            } else if !compatible {
                                                "Copy an eye state and reshape it without adding or removing nodes."
                                            } else if from == to {
                                                "Reshape the copied eye: both states currently look the same."
                                            } else {
                                                "Preview the transition with the eye slider in Face Rig."
                                            };
                                            if button(ui, Icon::Node, "Vertical Shape Tween", hint, linked, true).clicked() {
                                                vertical = Some((index, view, state.min(1)));
                                                ui.close();
                                            }
                                            if button(ui, Icon::Tween, "Horizontal Shape Tween", "", horizontal, true).clicked() {
                                                transition = Some((index, view));
                                                ui.close();
                                            }
                                        });
                                    }
                                    ui.end_row();
                                }
                                continue;
                            }
                            for slot in 0..8 {
                                let hidden = layer
                                    .options
                                    .character_piece
                                    .as_ref()
                                    .is_some_and(|piece| piece.hidden[slot]);
                                let offset = if self.layer == index {
                                    self.frame / 8 * 8
                                } else {
                                    0
                                };
                                let drawn = !layer.frames[offset + slot].is_empty();
                                let icon = if hidden {
                                    Icon::EyeOff
                                } else if drawn {
                                    Icon::Keyframe
                                } else {
                                    Icon::BlankKeyAdd
                                };
                                let piece = layer.options.character_piece.as_ref();
                                let linked = piece.is_some_and(|p| p.shape_transitions & (1 << slot) != 0);
                                let response = button(
                                    ui,
                                    icon,
                                    if linked { "→" } else { "" },
                                    VIEWS[slot],
                                    self.layer == index && self.frame % 8 == slot,
                                    true,
                                )
                                ;
                                if linked {
                                    ui.painter().line_segment(
                                        [response.rect.left_bottom(), response.rect.right_bottom()],
                                        egui::Stroke::new(2., egui::Color32::from_rgb(76, 176, 119)),
                                    );
                                }
                                if response.clicked() {
                                    selection = Some((index, slot));
                                }
                                response.context_menu(|ui| {
                                    let Some(piece) = piece else { return };
                                    let next = (slot + 1) % VIEWS.len();
                                    ui.label(format!("{} → {}", tr(VIEWS[slot]), tr(VIEWS[next])));
                                    if button(ui, Icon::Node, "Interpolate Shape", "", linked, true).clicked() {
                                        transition = Some((index, slot));
                                        ui.close();
                                    }
                                    if !linked && shape_transition(layer, piece, offset, slot).is_none() {
                                        ui.small(tr("Copy the drawing to the next view and edit its nodes. Compatible variants interpolate."));
                                    }

                                });
                            }
                            ui.end_row();
                        }
                    });
            });
        if let Some(row) = duplicate_side {
            self.duplicate_face_side(row);
        }
        if let Some((row, opposite, target)) = mirror {
            self.mirror_face_layer(row, opposite, target);
        }
        if let Some(row) = hand_tween {
            self.checkpoint();
            self.doc.layers[row]
                .options
                .character_piece
                .as_mut()
                .unwrap()
                .tween_hand_poses ^= true;
            self.recover();
        }
        if let Some((row, name)) = new_pose {
            self.finish_pending_pen();
            self.checkpoint();
            let piece = self.doc.layers[row]
                .options
                .character_piece
                .as_mut()
                .unwrap();
            piece.lip_poses.push(name.into());
            piece.lip_tweens.push(0);
            let slot = piece.lip_poses.len() * 8;
            if self.doc.layers[0].frames.len() < slot + 8 {
                for layer in &mut self.doc.layers {
                    layer.frames.resize(slot + 8, Default::default());
                    layer.options.keys.resize(slot + 8, true);
                }
            }
            selection = Some((row, slot + self.frame % 8));
            self.recover();
        }
        if let Some((row, mask)) = pupil_mask {
            self.finish_pending_pen();
            self.checkpoint();
            self.doc.layers[row]
                .options
                .character_piece
                .as_mut()
                .unwrap()
                .eye_mask = mask;
            self.recover();
        }
        if let Some(row) = delete {
            self.finish_pending_pen();
            self.checkpoint();
            let removed = self.doc.layers.remove(row);
            if let Some(removed_piece) = removed.options.character_piece {
                if row == 0
                    && let Some(first_piece) = self.doc.layers[0].options.character_piece.as_mut()
                {
                    first_piece.joints = removed_piece.joints;
                }
                if removed_piece.face == FacePart::Eye {
                    for layer in &mut self.doc.layers {
                        if let Some(piece) = layer.options.character_piece.as_mut()
                            && piece.eye_mask == Some(removed_piece.track)
                        {
                            piece.eye_mask = None;
                        }
                    }
                }
            }
            if self.layer == row {
                self.layer = self
                    .doc
                    .layers
                    .iter()
                    .position(|layer| {
                        layer
                            .options
                            .character_piece
                            .as_ref()
                            .is_some_and(|piece| piece.eye_parent.is_none())
                    })
                    .unwrap_or(0);
                self.frame %= 8;
            } else if self.layer > row {
                self.layer -= 1;
            }
            self.canvas.clear_selection();
            self.recover();
        }
        if let Some((row, track, matrix)) = attachment {
            let mask_tracks: Vec<_> = self
                .doc
                .layers
                .iter()
                .filter_map(|layer| layer.options.character_piece.as_ref())
                .filter(|piece| piece.face == FacePart::Eye && piece.eye_parent == Some(track))
                .map(|piece| piece.track)
                .collect();
            self.finish_pending_pen();
            self.checkpoint();
            let piece = self.doc.layers[row]
                .options
                .character_piece
                .as_mut()
                .unwrap();
            piece.eye_parent = Some(track);
            if piece
                .eye_mask
                .is_some_and(|mask| !mask_tracks.contains(&mask))
            {
                piece.eye_mask = None;
            }
            piece.matrix = matrix;
            self.recover();
        }
        if let Some((from, to)) = reorder.filter(|(from, to)| from != to) {
            self.finish_pending_pen();
            self.checkpoint();
            let joints = self.doc.layers[0]
                .options
                .character_piece
                .as_mut()
                .map(|p| std::mem::take(&mut p.joints));
            if from < to {
                self.doc.layers[from..=to].rotate_left(1);
            } else {
                self.doc.layers[to..=from].rotate_right(1);
            }
            if let Some(joints) = joints
                && let Some(piece) = self.doc.layers[0].options.character_piece.as_mut()
            {
                piece.joints = joints;
            }
            self.layer = if self.layer == from {
                to
            } else if from < self.layer && self.layer <= to {
                self.layer - 1
            } else if to <= self.layer && self.layer < from {
                self.layer + 1
            } else {
                self.layer
            };
            self.recover();
        }
        if let Some((row, name)) = renamed {
            draft = None;
            if !name.is_empty() && name != self.doc.layers[row].name {
                self.finish_pending_pen();
                self.checkpoint();
                let layer = &mut self.doc.layers[row];
                layer.name = name;
                if let Some(piece) = layer
                    .options
                    .character_piece
                    .as_mut()
                    .filter(|p| p.eye_parent.is_none())
                    && let Some((parent, _, _, _)) = self.feature.symbol_stack.last()
                {
                    let mut bones = parent
                        .extras
                        .rig
                        .bones
                        .iter()
                        .filter(|bone| bone.targets.contains(&piece.track));
                    if let Some(bone) = bones.next()
                        && bones.next().is_none()
                    {
                        piece.renamed_bone = Some(bone.id);
                    }
                }
                self.recover();
            }
        }
        ui.data_mut(|data| {
            if let Some(draft) = rename.or(draft) {
                data.insert_temp(rename_id, draft);
            } else {
                data.remove::<(usize, String)>(rename_id);
            }
        });
        if let Some((layer, slot)) = transition {
            self.finish_pending_pen();
            self.checkpoint();
            if let Some(piece) = &mut self.doc.layers[layer].options.character_piece {
                piece.shape_transitions ^= 1 << slot;
            }
            self.recover();
        }
        if let Some((layer, view, source)) = copy_mouth_view {
            self.finish_pending_pen();
            self.checkpoint();
            let layer_data = &mut self.doc.layers[layer];
            let count = layer_data
                .options
                .character_piece
                .as_ref()
                .unwrap()
                .lip_poses
                .len();
            for state in 1..=count {
                if layer_data.frames[state * 8 + view].is_empty() {
                    let destination = state * 8 + view;
                    let source = state * 8 + source;
                    layer_data.frames[destination] = layer_data.frames[source].clone();
                    copy_wrist_point(layer_data, destination, source);
                }
            }
            selection = Some((layer, 8 + view));
            self.recover();
        }
        if let Some((layer, pose, view)) = lip_transition {
            self.finish_pending_pen();
            self.checkpoint();
            self.doc.layers[layer]
                .options
                .character_piece
                .as_mut()
                .unwrap()
                .lip_tweens[pose] ^= 1 << view;
            self.recover();
        }
        if let Some((layer, frame, source)) = copy_eye {
            self.finish_pending_pen();
            self.checkpoint();
            let frames = &mut self.doc.layers[layer].frames;
            frames[frame] = frames[source].clone();
            copy_wrist_point(&mut self.doc.layers[layer], frame, source);
            selection = Some((layer, frame));
            self.recover();
        }
        if let Some(row) = vertical_all {
            self.finish_pending_pen();
            let layer = &self.doc.layers[row];
            let enabled: [u8; 8] = std::array::from_fn(|view| {
                let [open, half, closed] = [8, 16, 24].map(|offset| &*layer.frames[offset + view]);
                if half.is_empty() {
                    u8::from(compatible_eye_tween(open, closed)) * 3
                } else {
                    u8::from(compatible_eye_tween(open, half))
                        | (u8::from(compatible_eye_tween(half, closed)) << 1)
                }
            });
            if enabled != [0; 8] {
                self.checkpoint();
                let piece = self.doc.layers[row]
                    .options
                    .character_piece
                    .as_mut()
                    .unwrap();
                for (flags, mask) in piece.vertical_tweens.iter_mut().zip(enabled) {
                    *flags |= mask;
                }
                self.recover();
            } else {
                self.status = tr("Draw two compatible eye states in the same view.").into();
            }
        }
        if let Some((layer, view, state)) = vertical {
            self.finish_pending_pen();
            self.checkpoint();
            self.doc.layers[layer]
                .options
                .character_piece
                .as_mut()
                .unwrap()
                .vertical_tweens[view] ^= 1 << state;
            self.recover();
        }
        if let Some((layer, slot)) = selection {
            self.finish_pending_pen();
            self.layer = layer;
            self.frame = slot;
            self.canvas.clear_selection();
            self.playing = false;
        }
    }

    fn duplicate_face_side(&mut self, row: usize) {
        let source = &self.doc.layers[row];
        let Some(side) = source.name.rsplit_once('_').map(|(_, side)| side) else {
            return;
        };
        let parent = source
            .options
            .character_piece
            .as_ref()
            .and_then(|piece| piece.eye_parent);
        let candidates: Vec<_> = self
            .doc
            .layers
            .iter()
            .enumerate()
            .filter_map(|(index, layer)| {
                let piece = layer.options.character_piece.as_ref()?;
                (layer
                    .name
                    .rsplit_once('_')
                    .is_some_and(|(_, candidate)| candidate == side)
                    && piece.eye_parent == parent
                    && matches!(piece.face, FacePart::Eye | FacePart::Pupil | FacePart::Brow))
                .then_some((index, piece.track, piece.eye_mask))
            })
            .collect();
        if candidates.is_empty() || self.doc.layers.len() + candidates.len() > 100 {
            return;
        }
        self.finish_pending_pen();
        self.checkpoint();
        let mut tracks = std::collections::HashMap::new();
        let first = self.doc.layers.len();
        for (index, old, _) in &candidates {
            if let Some(track) = self.mirror_face_layer_inner(*index, false, None) {
                tracks.insert(*old, track);
            }
        }
        for (new, (_, _, mask)) in self.doc.layers[first..].iter_mut().zip(candidates) {
            if let Some(piece) = new.options.character_piece.as_mut() {
                piece.eye_mask = mask.and_then(|mask| tracks.get(&mask).copied());
            }
        }
        self.recover();
    }

    fn mirror_face_layer(&mut self, row: usize, opposite: bool, target: Option<u64>) {
        self.finish_pending_pen();
        self.checkpoint();
        if self
            .mirror_face_layer_inner(row, opposite, target)
            .is_some()
        {
            self.recover();
        }
    }

    fn mirror_face_layer_inner(
        &mut self,
        row: usize,
        opposite: bool,
        target: Option<u64>,
    ) -> Option<u64> {
        let mapped = |view| if opposite { opposite_view(view) } else { view };
        if self.doc.layers.len() >= 100 {
            return None;
        }
        let mut copy = self.doc.layers[row].clone();
        let piece = copy.options.character_piece.as_mut()?;
        let anchor = self.doc.layers.iter().find(|layer| {
            layer
                .options
                .character_piece
                .as_ref()
                .is_some_and(|p| Some(p.track) == piece.eye_parent)
        });
        let target_layer = target.and_then(|target| {
            self.doc.layers.iter().find(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .is_some_and(|piece| piece.track == target)
            })
        });
        let center = |layer: Option<&Layer>, view: usize| {
            layer
                .and_then(|layer| layer.frames.get(view))
                .map(|marks: &SharedVec<Mark>| {
                    marks.iter().fold(egui::Rect::NOTHING, |bounds, mark| {
                        bounds.union(crate::vector::control_bounds(mark))
                    })
                })
                .filter(|bounds| bounds.is_finite())
                .map(|bounds| bounds.center())
        };
        let default_axis = center(anchor, 0).map_or(self.doc.width as f32 * 0.5, |point| point.x);
        let hand_centers = center(anchor, 0).zip(center(target_layer, 0));
        let axis = |view| {
            if let Some((a, b)) = hand_centers {
                return ((a.x + b.x) * 0.5, b.y - a.y);
            }
            let a = center(anchor, view);
            let b = center(anchor, mapped(view));
            (
                match (a, b) {
                    (Some(a), Some(b)) => (a.x + b.x) * 0.5,
                    (a, b) => a.or(b).map_or(default_axis, |point| point.x),
                },
                0.,
            )
        };
        let axes = std::array::from_fn::<_, 8, _>(axis);
        let reflect = |point: egui::Pos2, view: usize| {
            egui::pos2(2. * axes[view].0 - point.x, point.y + axes[view].1)
        };
        let source = copy.frames.clone();
        let drawn = |view: usize| {
            (0..source.len())
                .step_by(8)
                .any(|start| !source[start + view].is_empty())
        };
        let mut transitions = if opposite { 0 } else { piece.shape_transitions };
        for view in 0..8 {
            if opposite
                && piece.shape_transitions & (1 << view) != 0
                && let Some(end) = (1..8)
                    .map(|step| (view + step) % 8)
                    .find(|&slot| drawn(slot))
            {
                transitions |= 1 << opposite_view(end);
            }
        }
        for (slot, marks) in copy.frames.iter_mut().enumerate() {
            let view = mapped(slot % 8);
            *marks = source[slot / 8 * 8 + view]
                .iter()
                .map(|mark| crate::vector::transform(mark, |p| reflect(p, view)))
                .collect();
        }
        if opposite && copy.options.keys.len() == source.len() {
            let keys = copy.options.keys.clone();
            for (slot, key) in copy.options.keys.iter_mut().enumerate() {
                *key = keys[slot / 8 * 8 + mapped(slot % 8)];
            }
        }
        if let Some(target) = target
            && let Some(anchor) =
                target_layer.and_then(|layer| layer.options.character_piece.as_ref())
        {
            piece.eye_parent = Some(target);
            piece.matrix = anchor.matrix;
        }
        if piece.face == FacePart::Hand {
            let points = piece.wrist_points.clone();
            piece.wrist_points = (0..copy.frames.len())
                .map(|slot| {
                    let view = mapped(slot % 8);
                    points
                        .get(slot / 8 * 8 + view)
                        .copied()
                        .flatten()
                        .map(|p| crate::vector::xy(reflect(crate::vector::pos(p), view)))
                })
                .collect();
            piece.wrist_origin = piece
                .wrist_origin
                .map(|p| crate::vector::xy(reflect(crate::vector::pos(p), 0)));
            if opposite {
                for (pose, flags) in piece.lip_tweens.iter_mut().enumerate() {
                    let offset = (pose + 1) * 8;
                    let mut mirrored = 0;
                    for view in 0..8 {
                        if *flags & (1 << view) != 0
                            && let Some(end) = (1..8)
                                .map(|step| (view + step) % 8)
                                .find(|&slot| !source[offset + slot].is_empty())
                        {
                            mirrored |= 1 << opposite_view(end);
                        }
                    }
                    *flags = mirrored;
                }
            }
        }
        piece.track = crate::features::id();
        piece.joints = Default::default();
        piece.hidden = std::array::from_fn(|view| piece.hidden[mapped(view)]);
        piece.order = std::array::from_fn(|view| piece.order[mapped(view)]);
        piece.vertical_tweens = std::array::from_fn(|view| piece.vertical_tweens[mapped(view)]);
        piece.shape_transitions = transitions;
        piece.brow_points = std::array::from_fn(|view| {
            let from = mapped(view);
            std::array::from_fn(|axis| {
                piece.brow_points[from][2 - axis]
                    .map(|p| crate::vector::xy(reflect(crate::vector::pos(p), from)))
            })
        });
        if piece.face == FacePart::Pupil {
            piece.pupil_area = std::array::from_fn(|view| {
                let source = mapped(view);
                piece.pupil_area[source].map(|points| {
                    points.map(|p| crate::vector::xy(reflect(crate::vector::pos(p), source)))
                })
            });
            let old = piece.circular_pupil_areas;
            piece.circular_pupil_areas =
                (0..8).fold(0, |mask, view| mask | (((old >> mapped(view)) & 1) << view));
        }
        for key in &mut piece.face_motion {
            if piece.face == FacePart::Pupil {
                key.axes[0] = -key.axes[0];
            } else if piece.face == FacePart::Brow {
                key.axes.swap(0, 2);
            } else if piece.face == FacePart::Hand && opposite {
                key.axes[1] = 1. - key.axes[1];
            }
        }
        copy.name = if let Some(name) = copy.name.strip_suffix("_R") {
            format!("{name}_L")
        } else if let Some(name) = copy.name.strip_suffix("_L") {
            format!("{name}_R")
        } else {
            format!("{} ({})", copy.name, tr("Mirrored"))
        };
        let eye = piece.face == FacePart::Eye;
        let control = piece.face.control();
        let count = piece.lip_poses.len();
        let track = piece.track;
        self.doc.layers.push(copy);
        self.layer = self.doc.layers.len() - 1;
        let offset = if eye {
            (self.frame / 8).clamp(1, 3) * 8
        } else if control {
            (self.frame / 8).clamp(1, count.max(1)) * 8
        } else {
            0
        };
        self.frame = offset + mapped(self.frame % 8);
        self.canvas.clear_selection();
        Some(track)
    }

    fn rigged_layer_targets(&self) -> Vec<(u64, String)> {
        let rig = &self.feature.symbol_stack.last().unwrap().0.extras.rig;
        self.doc
            .layers
            .iter()
            .filter_map(|layer| {
                let piece = layer.options.character_piece.as_ref()?;
                if piece.eye_parent.is_some()
                    || !rig
                        .bindings
                        .iter()
                        .any(|binding| binding.target == piece.track)
                {
                    return None;
                }
                let name = rig
                    .bones
                    .iter()
                    .find(|bone| bone.targets.contains(&piece.track))
                    .filter(|bone| bone.name != layer.name)
                    .map_or_else(
                        || layer.name.clone(),
                        |bone| format!("{} · {}", bone.name, layer.name),
                    );
                Some((piece.track, name))
            })
            .collect()
    }

    fn add_rigged_piece(&mut self, kind: FacePart, track: u64, source: Option<u64>) {
        if self.doc.layers.len() >= 100 {
            return;
        }
        let Some(mut piece) = self
            .doc
            .layers
            .iter()
            .filter_map(|layer| layer.options.character_piece.as_ref())
            .find(|piece| piece.track == track && piece.eye_parent.is_none())
            .cloned()
        else {
            return;
        };
        self.finish_pending_pen();
        let mut front: Vec<SharedVec<Mark>> = Vec::new();
        let mut layer_name = tr(match kind {
            FacePart::Eye => "Eye",
            FacePart::Brow => "Brow",
            FacePart::Mouth => "Mouth",
            FacePart::Hand => "Hand",
            FacePart::Slider => "Slider Rig",
            FacePart::Toggle => "On/Off Rig",
            FacePart::Switch => "Switch Rig",
            _ => "Pupil",
        })
        .to_owned();
        if kind.control() {
            piece.lip_poses = kind
                .control_poses()
                .iter()
                .copied()
                .map(crate::lip::Pose::from)
                .collect();
            piece.lip_tweens = vec![0; piece.lip_poses.len()].into();
        }
        if let Some(source) = source {
            let Some(symbol) = self
                .doc
                .extras
                .symbols
                .iter()
                .find(|symbol| symbol.id == source)
            else {
                return;
            };
            piece.lip_poses = symbol.mouth_poses.clone();
            piece.lip_tweens = vec![0; piece.lip_poses.len()].into();
            layer_name.clone_from(&symbol.name);
            let mut doc = self.doc.clone();
            doc.layers = symbol.layers.clone();
            doc.extras.rig = symbol.rig.clone();
            doc.extras.folders = symbol.folders.clone();
            doc.extras.cameras = Default::default();
            for pose in 0..piece.lip_poses.len() {
                front.push(
                    crate::features::resolve_export(&doc, pose, false)
                        .into_iter()
                        .map(|(_, _, mark)| mark)
                        .collect(),
                );
            }
            let bounds = |marks: &[Mark]| {
                marks.iter().fold(egui::Rect::NOTHING, |bounds, mark| {
                    bounds.union(crate::vector::control_bounds(mark))
                })
            };
            let source_bounds = front
                .iter()
                .map(|marks| bounds(marks))
                .find(|bounds| bounds.is_finite());
            let face = self.doc.layers.iter().find(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .is_some_and(|piece| piece.track == track)
            });
            if let Some(src) = source_bounds
                && let Some(dst) = face
                    .map(|layer| bounds(&layer.frames[0]))
                    .filter(|bounds| bounds.is_finite())
            {
                let center = egui::pos2(dst.center().x, egui::lerp(dst.top()..=dst.bottom(), 0.68));
                let delta = center - src.center();
                for marks in &mut front {
                    *marks = marks
                        .iter()
                        .map(|mark| crate::vector::transform(mark, |p| p + delta))
                        .collect();
                }
            }
        }
        self.checkpoint();
        piece.eye_parent = Some(piece.track);
        piece.renamed_bone = None;
        piece.track = crate::features::id();
        piece.face = kind;
        piece.face_keys = Default::default();
        piece.face_motion = Default::default();
        piece.vertical_tweens = Default::default();
        piece.wrist_points = Default::default();
        piece.wrist_origin = None;
        piece.tween_hand_poses = false;
        piece.shape_transitions = 0;
        piece.pupil_area = Default::default();
        piece.circular_pupil_areas = 0;
        piece.hidden = [false; 8];
        let count = self.doc.layers[0]
            .frames
            .len()
            .max(FACE_FRAMES)
            .max((piece.lip_poses.len() + 1) * 8);
        for layer in &mut self.doc.layers {
            layer.frames.resize(count, Default::default());
            layer.options.keys.resize(count, true);
        }
        let first_pose = front
            .iter()
            .position(|marks| !marks.is_empty())
            .unwrap_or(0);
        let mut frames = vec![SharedVec::default(); count];
        for (pose, marks) in front.into_iter().enumerate() {
            frames[(pose + 1) * 8] = marks;
        }
        self.doc.layers.push(Layer {
            name: layer_name,
            visible: true,
            frames: frames.into(),
            options: LayerOptions {
                keys: vec![true; count].into(),
                character_piece: Some(piece),
                ..Default::default()
            },
        });
        self.layer = self.doc.layers.len() - 1;
        self.frame = if kind == FacePart::Mouth {
            (first_pose + 1) * 8
        } else {
            (if kind == FacePart::Eye || kind.control() {
                8
            } else {
                0
            }) + self.frame % 8
        };
        self.canvas.clear_selection();
        self.recover();
    }

    pub(crate) fn in_character_symbol(&self) -> bool {
        self.feature
            .symbol_stack
            .last()
            .is_some_and(|(parent, _, _, id)| parent.extras.rig.character_symbol == Some(*id))
    }

    pub(crate) fn open_character_symbol(&mut self) {
        if let Some(id) = self.doc.extras.rig.character_symbol
            && self.doc.extras.symbols.iter().any(|symbol| symbol.id == id)
        {
            self.enter_symbol(id);
            return;
        }
        let mut layers = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for layer in &self.doc.layers {
            for mark in crate::animation::marks(layer, self.frame).iter() {
                let Some(instance) = &mark.style.instance else {
                    continue;
                };
                if !self
                    .doc
                    .extras
                    .rig
                    .bindings
                    .iter()
                    .any(|binding| binding.target == mark.style.track)
                    || !seen.insert(mark.style.track)
                {
                    continue;
                }
                let Some(symbol) = self
                    .doc
                    .extras
                    .symbols
                    .iter()
                    .find(|s| s.id == instance.symbol)
                else {
                    continue;
                };
                let mut local = self.doc.clone();
                local.layers = symbol.layers.clone();
                local.extras.rig = symbol.rig.clone();
                local.extras.folders = symbol.folders.clone();
                local.extras.cameras = Default::default();
                let frame = crate::features::symbol_frame(
                    instance,
                    local.layers[0].frames.len(),
                    self.frame,
                    self.frame,
                );
                let front = crate::features::resolve_export(&local, frame, false)
                    .into_iter()
                    .map(|(_, _, mark)| crate::features::affine(&mark, instance.matrix))
                    .collect();
                let mut frames = vec![SharedVec::default(); VIEWS.len()];
                frames[0] = front;
                layers.push(Layer {
                    name: symbol.name.clone(),
                    visible: true,
                    frames: frames.into(),
                    options: LayerOptions {
                        keys: vec![true; VIEWS.len()].into(),
                        character_piece: Some(Piece {
                            track: mark.style.track,
                            matrix: instance.matrix,
                            hidden: [false; 8],
                            order: [0; 8],
                            joints: Default::default(),
                            face: if symbol.mouth_poses.is_empty() {
                                FacePart::None
                            } else {
                                FacePart::Mouth
                            },
                            face_keys: Default::default(),
                            face_motion: Default::default(),
                            gaze_limits: Default::default(),
                            eye_mask: None,
                            eye_parent: None,
                            vertical_tweens: Default::default(),
                            lip_poses: Default::default(),
                            lip_tweens: Default::default(),
                            wrist_points: Default::default(),
                            wrist_origin: None,
                            tween_hand_poses: false,
                            renamed_bone: None,
                            pupil_area: Default::default(),
                            circular_pupil_areas: 0,
                            brow_points: Default::default(),
                            shape_transitions: 0,
                        }),
                        ..Default::default()
                    },
                });
            }
        }
        if layers.is_empty() {
            self.status =
                tr("Bind at least one symbol to the rig before setting up a character.").into();
            return;
        }
        if layers.len() > 100 || self.doc.extras.symbols.len() >= 500 {
            self.status = tr("Character Symbol exceeds the project limits.").into();
            return;
        }
        let id = crate::features::id();
        self.checkpoint();
        let rig = &mut self.doc.extras.rig;
        if rig.character_turns.body.root.is_none() {
            rig.character_turns.body.root = rig
                .bones
                .iter()
                .find(|bone| bone.parent.is_none())
                .map(|bone| bone.id);
        }
        self.doc.extras.rig.character_symbol = Some(id);
        self.doc.extras.symbols.push(Symbol {
            id,
            name: tr("Character Symbol").into(),
            layers: layers.into(),
            rig: Default::default(),
            library_folder: None,
            mouth_poses: Default::default(),
            folders: Default::default(),
            perspective: Default::default(),
        });
        self.recover();
        self.enter_symbol(id);
    }

    pub(crate) fn character_rig_checks(&mut self, ctx: &egui::Context, canvas: egui::Rect) {
        let row = self.layer;
        let layer = &self.doc.layers[row];
        let Some(piece) = layer.options.character_piece.as_ref() else {
            return;
        };
        let view = self.frame % 8;
        let target = |track| {
            self.doc.layers.iter().find(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .is_some_and(|p| p.track == track)
            })
        };
        let eye = piece.eye_mask.and_then(target).filter(|layer| {
            layer
                .options
                .character_piece
                .as_ref()
                .is_some_and(|p| p.face == FacePart::Eye)
        });
        let anchor = eye
            .and_then(|layer| layer.options.character_piece.as_ref())
            .and_then(|p| p.eye_parent)
            .or(piece.eye_parent)
            .unwrap_or(piece.track);
        let parent = &self.feature.symbol_stack.last().unwrap().0;
        let attached = target(anchor).filter(|_| {
            parent
                .extras
                .rig
                .bindings
                .iter()
                .any(|b| b.target == anchor)
        });
        let mut check_tweens = layer.options.rig_check_tweens;
        let result = egui::Area::new(egui::Id::unique("character-rig-checks"))
            .order(egui::Order::Middle)
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(canvas.right_top() + egui::vec2(-12., 12.))
            .constrain_to(canvas)
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.set_width(216.);
                    ui.horizontal(|ui| {
                        character_row_icon(ui, Icon::Bones, tr("Rig checklist"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.small(tr(VIEWS[view]));
                        });
                    });
                    ui.add(egui::Label::new(egui::RichText::new(&layer.name).strong()).truncate()).on_hover_text(&layer.name);
                    ui.separator();
                    rig_check_row(ui, attached.is_some(), &attached.map_or_else(
                        || tr("Attach to a rig layer").to_owned(),
                        |layer| format!("{} {}", tr("Attached to"), layer.name)));
                    match piece.face {
                        FacePart::Pupil => {
                            rig_check_row(ui, piece.pupil_range(view).is_some(), tr("Pupil area"));
                            rig_check_row(ui, eye.is_some(), &eye.map_or_else(
                                || tr("Eye clipping").to_owned(),
                                |eye| format!("{} {}", tr("Clipped to"), eye.name)));
                        }
                        kind if !piece.lip_poses.is_empty() => {
                            let count = (1..=piece.lip_poses.len()).filter(|state| !layer.frames[state * 8 + view].is_empty()).count();
                            let required = if kind == FacePart::Toggle && layer.frames[8 + view].is_empty() {
                                false
                            } else { count == piece.lip_poses.len() || kind == FacePart::Toggle };
                            rig_check_row(ui, required,
                                &format!("{count}/{} {}", piece.lip_poses.len(), tr("poses drawn")));
                            if kind == FacePart::Hand {
                                rig_check_row(ui, piece.wrist_points.get(self.frame).is_some_and(Option::is_some), tr("Wrist point"));
                            }
                        }
                        FacePart::Brow => {
                            for (name, point) in BROW_POINTS.into_iter().zip(piece.brow_points[view]) {
                                rig_check_row(ui, point.is_some(), tr(name));
                            }
                        }
                        _ => {}
                    }
                    let offset = if piece.face == FacePart::Eye || !piece.lip_poses.is_empty() { (self.frame / 8).max(1) * 8 } else { 0 };
                    let drawn = |slot: usize| !piece.hidden[slot] && layer.frames.get(offset + slot).is_some_and(|marks| !marks.is_empty());
                    if piece.hidden[view] {
                        ui.small(tr("Hidden in this view"));
                    } else if !drawn(view) && let Some(other) = (0usize..8).filter(|&slot| drawn(slot)).min_by_key(|&slot| {
                        let distance = slot.abs_diff(view);
                        distance.min(8 - distance)
                    }) {
                        if piece.face != FacePart::Toggle {
                            ui.small(format!("{} {}", tr("Nearest drawing"), tr(VIEWS[other])));
                        }
                    }
                    ui.separator();
                    let changed = ui.checkbox(&mut check_tweens, tr("Check Shape Tweens"))
                        .on_hover_text(tr("Check this layer’s drawn views for missing or incompatible transitions")).changed();
                    if check_tweens && !piece.lip_poses.is_empty() {
                        let mut drawn = 0;
                        let mut ready = 0;
                        for (pose, &flags) in piece.lip_tweens.iter().enumerate() {
                            let offset = (pose + 1) * 8;
                            let from = &layer.frames[offset + view];
                            if from.is_empty() { continue; }
                            drawn += 1;
                            if flags & (1 << view) != 0
                                && let Some(next) = (1..8).map(|step| (view + step) % 8).find(|&slot| {
                                    !piece.hidden[slot] && !layer.frames[offset + slot].is_empty()
                                })
                                && compatible_eye_tween(from, &layer.frames[offset + next]) {
                                ready += 1;
                            }
                        }
                        rig_check_row(ui, drawn > 0 && ready == drawn,
                            &format!("{ready}/{drawn} {}", tr("pose tweens ready")));
                    } else if check_tweens {
                        let checks = rig_tween_checks(layer, piece, view);
                        rig_check_row(ui, checks.horizontal == TweenCheck::Ready, checks.horizontal.label(false));
                        if let Some(vertical) = checks.vertical {
                            rig_check_row(ui, vertical == TweenCheck::Ready, vertical.label(true));
                        }
                        let pending: Vec<_> = (0..8).filter(|&other| other != view && rig_view_drawn(layer, piece, other)
                            && !rig_tween_checks(layer, piece, other).ready())
                            .map(|other| tr(VIEWS[other])).collect();
                        if !pending.is_empty() {
                            ui.small(format!("{} {}", pending.len(), tr("other views to check")))
                                .on_hover_text(pending.join(", "));
                        }
                    }
                    changed
                }).inner
            });
        if result.inner {
            self.checkpoint();
            self.doc.layers[row].options.rig_check_tweens = check_tweens;
            self.recover();
        }
    }

    pub(crate) fn floating_character_tools(
        &mut self,
        ctx: &egui::Context,
        timeline: egui::Rect,
        canvas: egui::Rect,
    ) {
        let editing = self.in_character_symbol();
        let path = if editing {
            None
        } else {
            self.selected_character_path()
        };
        let setup = !editing
            && path.is_none()
            && self.tool == crate::canvas::Tool::Select
            && self.doc.extras.rig.character_symbol.is_none()
            && self
                .selected_rig_tracks(&self.canvas.selected)
                .iter()
                .any(|track| {
                    self.doc
                        .extras
                        .rig
                        .bindings
                        .iter()
                        .any(|binding| binding.target == *track)
                });
        if !editing && path.is_none() && !setup {
            return;
        }
        egui::Area::new(egui::Id::unique(("floating-character-tools", editing)))
            .order(egui::Order::Middle)
            .pivot(egui::Align2::CENTER_BOTTOM)
            .fixed_pos(egui::pos2(timeline.center().x, timeline.top() - 8.))
            .constrain_to(canvas)
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    if editing {
                        self.character_toolbar(ui);
                    } else if setup
                        && button(
                            ui,
                            Icon::Head,
                            "Set Up Rig",
                            "Create the Character Symbol",
                            false,
                            true,
                        )
                        .clicked()
                    {
                        self.open_character_symbol();
                    } else if let Some(path) = path
                        && button(
                            ui,
                            Icon::Head,
                            "Edit Rig",
                            "Edit Character Symbol",
                            false,
                            true,
                        )
                        .clicked()
                    {
                        for id in path {
                            self.enter_symbol(id);
                        }
                    }
                });
            });
    }

    fn character_toolbar(&mut self, ui: &mut egui::Ui) {
        let frame = self.frame % 8;
        let layer = self.layer;
        let art_frame = self.frame;
        ui.vertical_centered(|ui| {
            ui.strong(tr("Character Tools"));
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Mode"));
                if button(
                    ui,
                    Icon::Brush,
                    "Artwork",
                    "",
                    !self.rigging.edit_character_bones,
                    true,
                )
                .clicked()
                {
                    self.rigging.edit_character_bones = false;
                    self.rigging.test_character_pose = None;
                    self.activate_tool(crate::canvas::Tool::Brush);
                }
                if button(
                    ui,
                    Icon::Bones,
                    "Skeleton",
                    "",
                    self.rigging.edit_character_bones,
                    true,
                )
                .clicked()
                {
                    self.rigging.edit_character_bones = true;
                    self.rigging.test_character_pose = None;
                }
                ui.separator();
                if button(
                    ui,
                    Icon::Play,
                    "Test Pose",
                    "Preview the selected layer without creating keys",
                    self.rigging.test_character_pose.is_some(),
                    true,
                )
                .clicked()
                {
                    self.rigging.edit_character_bones = false;
                    self.rigging.test_character_pose = if self.rigging.test_character_pose.is_some()
                    {
                        None
                    } else {
                        Some([frame as f32 / 8., 0.])
                    };
                    self.canvas.clear_selection();
                }
                let setup_copy = button(
                    ui,
                    Icon::Copy,
                    "Copy View Setup",
                    "Copy selected parts from another view",
                    false,
                    self.rigging.test_character_pose.is_none(),
                );
                egui::Popup::menu(&setup_copy).show(|ui| {
                    let id = ui.scope_id().with("copy-view-options");
                    let (mut artwork, mut bones, mut guides, mut empty) = ui
                        .data(|data| data.get_temp::<(bool, bool, bool, bool)>(id))
                        .unwrap_or((true, false, false, true));
                    ui.label(format!("{} {}", tr("Destination"), tr(VIEWS[frame])));
                    ui.checkbox(&mut artwork, tr("Artwork"));
                    ui.checkbox(&mut bones, tr("Bones"));
                    ui.checkbox(&mut guides, tr("Rig guides"));
                    ui.checkbox(&mut empty, tr("Only empty cells"));
                    ui.separator();
                    for (from, name) in VIEWS
                        .into_iter()
                        .enumerate()
                        .filter(|(view, _)| *view != frame)
                    {
                        if button(ui, Icon::Copy, name, "", false, artwork || bones || guides)
                            .clicked()
                        {
                            self.copy_character_view(from, frame, artwork, bones, guides, empty);
                            ui.close();
                        }
                    }
                    ui.data_mut(|data| data.insert_temp(id, (artwork, bones, guides, empty)));
                });
                let bones = self.rigging.edit_character_bones;
                let copy = button(
                    ui,
                    Icon::Copy,
                    if bones {
                        "Copy Bones from View"
                    } else {
                        "Copy Artwork from View"
                    },
                    "",
                    false,
                    true,
                );
                egui::Popup::menu(&copy).show(|ui| {
                    for (source, name) in VIEWS.into_iter().enumerate().filter(|(i, _)| *i != frame)
                    {
                        let source_frame = art_frame / 8 * 8 + source;
                        let available = if bones {
                            self.doc.layers[0]
                                .options
                                .character_piece
                                .as_ref()
                                .is_some_and(|piece| !piece.joints[source].is_empty())
                        } else {
                            !self.doc.layers[layer].frames[source_frame].is_empty()
                        };
                        if button(ui, Icon::Copy, name, "", false, available).clicked() {
                            self.finish_pending_pen();
                            self.checkpoint();
                            if bones {
                                if let Some(settings) =
                                    self.doc.layers[0].options.character_piece.as_mut()
                                {
                                    settings.joints[frame] = settings.joints[source].clone();
                                }
                            } else {
                                let frames = &mut self.doc.layers[layer].frames;
                                frames[art_frame] = frames[source_frame].clone();
                                copy_wrist_point(
                                    &mut self.doc.layers[layer],
                                    art_frame,
                                    source_frame,
                                );
                            }
                            self.recover();
                            ui.close();
                        }
                    }
                });
            });
            if let Some([mut angle, mut value]) = self.rigging.test_character_pose {
                ui.add(egui::Slider::new(&mut angle, 0. ..=1.).text(tr("Turnaround")));
                ui.add(egui::Slider::new(&mut value, 0. ..=1.).text(tr("Pose")));
                self.rigging.test_character_pose = Some([angle, value]);
            }
        });
    }

    fn copy_character_view(
        &mut self,
        source: usize,
        destination: usize,
        artwork: bool,
        bones: bool,
        guides: bool,
        empty: bool,
    ) {
        self.finish_pending_pen();
        self.checkpoint();
        if bones
            && let Some(piece) = self.doc.layers[0].options.character_piece.as_mut()
            && (!empty || piece.joints[destination].is_empty())
            && !piece.joints[source].is_empty()
        {
            piece.joints[destination] = piece.joints[source].clone();
        }
        for layer in &mut self.doc.layers {
            for base in (0..layer.frames.len()).step_by(8) {
                let from = base + source;
                let to = base + destination;
                if artwork
                    && !layer.frames[from].is_empty()
                    && (!empty || layer.frames[to].is_empty())
                {
                    layer.frames[to] = layer.frames[from].clone();
                }
                if guides
                    && layer.options.character_piece.as_ref().is_some_and(|piece| {
                        piece.wrist_points.get(from).is_some_and(Option::is_some)
                            && (!empty || piece.wrist_points.get(to).is_none_or(Option::is_none))
                    })
                {
                    copy_wrist_point(layer, to, from);
                }
            }
            if guides && let Some(piece) = layer.options.character_piece.as_mut() {
                if let Some(area) = piece.pupil_area[source]
                    && (!empty || piece.pupil_area[destination].is_none())
                {
                    piece.pupil_area[destination] = Some(area);
                    piece.circular_pupil_areas = (piece.circular_pupil_areas & !(1 << destination))
                        | (((piece.circular_pupil_areas >> source) & 1) << destination);
                }
                for index in 0..3 {
                    if let Some(point) = piece.brow_points[source][index]
                        && (!empty || piece.brow_points[destination][index].is_none())
                    {
                        piece.brow_points[destination][index] = Some(point);
                    }
                }
            }
        }
        self.recover();
    }

    pub(crate) fn character_bone_input(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        area: egui::Rect,
        view: crate::canvas::View,
    ) -> bool {
        if !self.in_character_symbol() || !self.rigging.edit_character_bones {
            return false;
        }
        let frame = self.frame % 8;
        let Some(settings) = self.doc.layers[0].options.character_piece.as_ref() else {
            return true;
        };
        let parent = &self.feature.symbol_stack.last().unwrap().0.extras.rig;
        let bones: Vec<_> = parent
            .bones
            .iter()
            .map(|bone| {
                let points = settings.joints[frame]
                    .iter()
                    .find(|(id, _)| *id == bone.id)
                    .map_or([bone.start, bone.end], |(_, points)| *points);
                (bone.id, points)
            })
            .collect();
        let overlay = painter
            .ctx()
            .layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                ui.scope_id().with("character-bones"),
            ))
            .with_clip_rect(area);
        let color = crate::themes::accent();
        for &(id, points) in &bones {
            let screen = points.map(|p| view.screen(crate::vector::pos(p)));
            overlay.line_segment(screen, egui::Stroke::new(2., color));
            for (end, center) in screen.into_iter().enumerate() {
                overlay.circle_filled(center, 5., color);
                let response = ui.interact(
                    egui::Rect::from_center_size(center, egui::Vec2::splat(16.)).intersect(area),
                    ui.scope_id().with(("character-joint", id, end)),
                    egui::Sense::drag(),
                );
                if response.drag_started() {
                    self.checkpoint();
                }
                if (response.dragged() || response.drag_stopped())
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    let mut points = points;
                    points[end] = crate::vector::xy(view.world(pointer));
                    if crate::vector::pos(points[0]).distance(crate::vector::pos(points[1])) > 0.01
                    {
                        let joints = &mut self.doc.layers[0]
                            .options
                            .character_piece
                            .as_mut()
                            .unwrap()
                            .joints[frame];
                        if let Some((_, stored)) = joints.iter_mut().find(|(bone, _)| *bone == id) {
                            *stored = points;
                        } else {
                            joints.push((id, points));
                        }
                    }
                }
                if response.drag_stopped() {
                    self.recover();
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turnaround_interpolates_across_blank_views_in_both_directions() {
        let drawn = |view| view == 0 || view == 2 || view == 7;
        assert_eq!(drawn_transition(1., 1, drawn), Some((0, 2, 0.5)));
        assert_eq!(drawn_transition(1.5, 1, drawn), Some((0, 2, 0.75)));
        assert_eq!(drawn_transition(-0.5, 1 << 7, drawn), Some((7, 0, 0.5)));
        assert_eq!(drawn_transition(1., 0, drawn), None);
    }

    #[test]
    fn brow_center_control_bends_a_line_even_without_a_center_node() {
        let line = Mark {
            points: vec![[0., 0.], [100., 0.]].into(),
            ..Default::default()
        };
        let bent = deform_brow_mark(&line, |p| (1. - (p.x - 50.).abs() / 50.) * -20.);
        assert_eq!(bent.nodes.first().unwrap().position, [0., 0.]);
        assert_eq!(bent.nodes.last().unwrap().position, [100., 0.]);
        assert_eq!(bent.nodes[2].position, [50., -20.]);
    }

    #[test]
    fn eye_tween_rebuilds_fragmented_fill_from_the_edited_outline() {
        let outline = |height| Mark {
            points: vec![[0., 0.], [20., 0.], [20., height], [0., height]].into(),
            stroke: true,
            closed: true,
            ..Default::default()
        };
        let fill = |mut mark: Mark| {
            mark.stroke = false;
            mark.fill = Some([255; 4]);
            mark
        };
        let open = [fill(outline(20.)), outline(20.)];
        let half = [fill(outline(3.)), fill(outline(10.)), outline(10.)];
        let tween = eye_tween(&open, &half, 0.5).unwrap();
        assert_eq!(tween.len(), 2);
        assert_eq!(tween[0].points, tween[1].points);
        assert_eq!(tween[0].points[2][1], 15.);

        let mut multicolor = half.clone();
        multicolor[0].fill = Some([0, 0, 0, 255]);
        assert!(eye_tween(&open, &multicolor, 0.5).is_none());
    }

    #[test]
    fn character_view_selects_artwork_and_keeps_missing_views_editable() {
        let mut doc = crate::fresh_document();
        let rig = &mut doc.extras.rig;
        rig.character_symbol = Some(5);
        rig.bones.push(rig::Bone {
            id: 7,
            name: "Torso".into(),
            parent: None,
            start: [0., 0.],
            end: [10., 0.],
            radius: 10.,
            ik: false,
            targets: vec![9].into(),
            keys: Default::default(),
        });
        rig.character_turns.body.root = Some(7);
        rig.character_turns.body.set(5, 2.);
        let mut frames = vec![SharedVec::default(); 8];
        frames[0] = vec![Mark {
            color: [255, 0, 0, 255],
            ..Default::default()
        }]
        .into();
        frames[2] = vec![Mark {
            color: [0, 0, 255, 255],
            ..Default::default()
        }]
        .into();
        let mut piece = Piece {
            track: 9,
            matrix: rig::IDENTITY,
            hidden: [false; 8],
            order: [0; 8],
            joints: Default::default(),
            face: FacePart::None,
            face_keys: Default::default(),
            face_motion: Default::default(),
            gaze_limits: Default::default(),
            eye_mask: None,
            eye_parent: None,
            vertical_tweens: Default::default(),
            lip_poses: Default::default(),
            lip_tweens: Default::default(),
            wrist_points: Default::default(),
            wrist_origin: None,
            tween_hand_poses: false,
            renamed_bone: None,
            pupil_area: Default::default(),
            circular_pupil_areas: 0,
            brow_points: Default::default(),
            shape_transitions: 0,
        };
        piece.hidden[4] = true;
        piece.joints[2].push((7, [[0., 0.], [5., 0.]]));
        doc.extras.symbols.push(Symbol {
            id: 5,
            name: "Character Symbol".into(),
            rig: Default::default(),
            library_folder: None,
            mouth_poses: Default::default(),
            folders: Default::default(),
            perspective: Default::default(),
            layers: vec![Layer {
                name: "Torso".into(),
                visible: true,
                frames: frames.into(),
                options: LayerOptions {
                    character_piece: Some(piece),
                    ..Default::default()
                },
            }]
            .into(),
        });
        let saved = serde_json::to_vec(&doc).unwrap();
        let mut doc: crate::Document = serde_json::from_slice(&saved).unwrap();
        assert_eq!(
            artwork(&doc, 9, 0, None).unwrap().0[0].frames[0][0].color,
            [255, 0, 0, 255]
        );
        assert_eq!(
            artwork(&doc, 9, 5, None).unwrap().0[0].frames[0][0].color,
            [0, 0, 255, 255]
        );
        assert_eq!(view_joints(&doc, &doc.extras.rig, 5)[0].1[1], [5., 0.]);
        doc.extras.rig.character_turns.body.set(7, 4.);
        assert!(artwork(&doc, 9, 7, None).unwrap().0.is_empty());
        assert!(artwork(&doc, 10, 5, None).is_none());
        let layer = &mut doc.extras.symbols[0].layers[0];
        layer.options.character_piece.as_mut().unwrap().face = FacePart::Mouth;
        layer.frames.resize(FACE_FRAMES, Default::default());
        let o = 6 * 8;
        layer.frames[o + 2] = vec![Mark {
            color: [33, 44, 55, 255],
            ..Default::default()
        }]
        .into();
        assert_eq!(
            artwork(&doc, 9, 5, Some(5)).unwrap().0[0].frames[0][0].color,
            [33, 44, 55, 255]
        );
        assert!(artwork(&doc, 9, 5, Some(2)).is_none());
        let mut eye_doc = doc.clone();
        let layer = &mut eye_doc.extras.symbols[0].layers[0];
        let piece = layer.options.character_piece.as_mut().unwrap();
        piece.face = FacePart::Eye;
        piece.eye_parent = Some(9);
        piece.track = 11;
        piece.vertical_tweens[2] = 1;
        piece.set_motion(5, [0.5, 0., 0.]);
        layer.frames[10] = vec![Mark {
            points: vec![[0., 0.], [10., 20.]].into(),
            ..Default::default()
        }]
        .into();
        layer.frames[10][0].style.track = 101;
        layer.frames[26] = vec![Mark {
            points: vec![[0., 0.], [10., 0.]].into(),
            ..Default::default()
        }]
        .into();
        layer.frames[26][0].style.track = 102;
        assert_eq!(
            rigged_eye_artwork(&eye_doc, 9, 5, None)[0].points[1],
            [10., 10.]
        );
        let layer = &mut eye_doc.extras.symbols[0].layers[0];
        let piece = layer.options.character_piece.as_mut().unwrap();
        piece.shape_transitions = 1 << 1;
        piece.vertical_tweens[1] = 1;
        layer.frames[9] = vec![Mark {
            points: vec![[0., 0.], [30., 40.]].into(),
            ..Default::default()
        }]
        .into();
        layer.frames[25] = vec![Mark {
            points: vec![[0., 0.], [30., 0.]].into(),
            ..Default::default()
        }]
        .into();
        eye_doc.extras.rig.character_turns.body.set(5, 1.5);
        assert_eq!(
            rigged_eye_artwork(&eye_doc, 9, 5, None)[0].points[1],
            [20., 15.]
        );

        let ctx = egui::Context::default();
        let mut editor = crate::Editor::new(&eframe::CreationContext::_new_kittest(ctx));
        editor.recovery = std::env::temp_dir().join(format!(
            "libretoon-mirror-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        editor.doc.layers = eye_doc.extras.symbols[0].layers.clone();
        editor.layer = 0;
        editor.frame = 10;
        editor.mirror_face_layer(0, false, None);
        let mirrored = &editor.doc.layers[1];
        for slot in [9, 10, 25, 26] {
            let source = &editor.doc.layers[0].frames[slot][0];
            let target = &mirrored.frames[slot][0];
            assert_eq!(
                target.points[1],
                [
                    editor.doc.width as f32 - source.points[1][0],
                    source.points[1][1]
                ]
            );
        }
        assert_eq!(
            mirrored
                .options
                .character_piece
                .as_ref()
                .unwrap()
                .vertical_tweens,
            editor.doc.layers[0]
                .options
                .character_piece
                .as_ref()
                .unwrap()
                .vertical_tweens
        );
        assert_eq!(editor.frame, 10);

        editor.doc = eye_doc.clone();
        editor.insert_rig_control_key(11, true, 2);
        editor.edit_rig_control_range(11, true, 2..=5, None, false, Some(RigEase::InOut));
        editor.edit_rig_control_range(11, true, 2..=5, Some(1), false, None);
        assert_eq!(editor.rig_control_frames(11, true), vec![0, 3, 6]);
        editor.edit_rig_control_range(11, true, 3..=6, Some(-3), true, None);
        assert_eq!(editor.rig_control_frames(11, true), vec![0, 3, 6]);
        editor.history(false);
        assert_eq!(editor.rig_control_frames(11, true), vec![0, 2, 5]);
        drop(editor);

        let mut brow_doc = eye_doc.clone();
        let brow = &mut brow_doc.extras.symbols[0].layers[0];
        let piece = brow.options.character_piece.as_mut().unwrap();
        piece.face = FacePart::Brow;
        piece.shape_transitions = 1;
        piece.brow_points[0] = [Some([0., 0.]), Some([50., 0.]), Some([100., 0.])];
        piece.brow_points[1] = [Some([300., 0.]), Some([350., 0.]), Some([400., 0.])];
        piece.set_motion(5, [0., -1., 0.]);
        brow.frames[..8].fill(Default::default());
        for (view, left) in [0., 300.].into_iter().enumerate() {
            brow.frames[view] = vec![Mark {
                points: vec![[left, 0.], [left + 100., 0.]].into(),
                ..Default::default()
            }]
            .into();
        }
        assert!(rig_tween_checks(brow, brow.options.character_piece.as_ref().unwrap(), 0).ready());
        assert!(
            rig_tween_checks(brow, brow.options.character_piece.as_ref().unwrap(), 1).horizontal
                == TweenCheck::Disabled
        );
        brow_doc.extras.rig.character_turns.body.set(5, 0.5);
        let bent = &rigged_eye_artwork(&brow_doc, 9, 5, None)[0];
        assert_eq!(bent.nodes[0].position, [150., 0.]);
        assert_eq!(bent.nodes[2].position, [200., -35.]);
        assert_eq!(bent.nodes[4].position, [250., 0.]);

        let mut lip_doc = eye_doc.clone();
        let mouth = &mut lip_doc.extras.symbols[0].layers[0];
        let piece = mouth.options.character_piece.as_mut().unwrap();
        piece.face = FacePart::Mouth;
        piece.lip_poses = vec!["Rest".into(), "A".into()].into();
        piece.lip_tweens = vec![0, 1].into();
        mouth.frames[..24].fill(Default::default());
        for (slot, x) in [(8, -10.), (16, 20.), (18, 60.)] {
            mouth.frames[slot] = vec![Mark {
                points: vec![[x, 0.], [x + 10., 5.]].into(),
                ..Default::default()
            }]
            .into();
        }
        let route = serde_json::json!({
            "path": [crate::lip::character_mouth_slot(5, 11)],
            "keys": [{"frame": 5, "pose": 1, "manual": true}]
        });
        let routes = [serde_json::from_value(route).unwrap()];
        let route = crate::lip::RouteScope::new(&routes, 5);
        lip_doc.extras.rig.character_turns.body.set(5, 1.);
        assert_eq!(
            rigged_eye_artwork(&lip_doc, 9, 5, Some(&route))[0].points[0],
            [40., 0.]
        );
        lip_doc.extras.rig.character_turns.body.set(5, 3.);
        assert_eq!(
            rigged_eye_artwork(&lip_doc, 9, 5, Some(&route))[0].points[0],
            [60., 0.]
        );
        assert_eq!(
            rigged_eye_artwork(&lip_doc, 9, 5, None)[0].points[0],
            [-10., 0.]
        );

        let mut controls = lip_doc.extras.symbols[0].layers[0].clone();
        controls.frames[18] = Default::default();
        let piece = controls.options.character_piece.as_mut().unwrap();
        piece.face = FacePart::Slider;
        piece.lip_poses = vec!["Start".into(), "Middle".into(), "End".into()].into();
        piece.lip_tweens = vec![0; 3].into();
        controls.frames[16] = Default::default();
        controls.frames[24] = vec![Mark {
            points: vec![[30., 0.], [40., 5.]].into(),
            ..Default::default()
        }]
        .into();
        piece.set_motion(5, [1. / 127., 0., 0.]);
        assert_eq!(
            control_drawing(
                &controls,
                controls.options.character_piece.as_ref().unwrap(),
                5,
                0.
            )[0]
            .points[0],
            [10., 0.]
        );

        let piece = controls.options.character_piece.as_mut().unwrap();
        piece.face = FacePart::Toggle;
        piece.set_variant(5, 1);
        assert!(
            control_drawing(
                &controls,
                controls.options.character_piece.as_ref().unwrap(),
                5,
                0.
            )
            .is_empty()
        );

        let piece = controls.options.character_piece.as_mut().unwrap();
        piece.face = FacePart::Hand;
        piece.set_variant(0, 0);
        piece.set_variant(10, 2);
        piece.set_variant(5, 2);
        piece.wrist_origin = Some([0., 0.]);
        piece.wrist_points.resize(25, None);
        piece.wrist_points[24] = Some([30., 0.]);
        assert_eq!(
            control_drawing(
                &controls,
                controls.options.character_piece.as_ref().unwrap(),
                5,
                4.
            )[0]
            .points[0],
            [0., 0.]
        );

        // The pupil uses the animated opening in scene space, including during a blink.
        eye_doc.extras.rig.character_turns.body.set(5, 0.);
        let layers = &mut eye_doc.extras.symbols[0].layers;
        let rectangle = |min, max, color| Mark {
            points: vec![min, max].into(),
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some(color),
            ..Default::default()
        };
        let white = [255; 4];
        layers[0].frames[8] = vec![rectangle([0., 0.], [20., 20.], white)].into();
        layers[0].frames[24] = vec![rectangle([0., 10.], [20., 10.], white)].into();
        let eye = layers[0].options.character_piece.as_mut().unwrap();
        eye.vertical_tweens[0] = 1;
        let mut pupil = layers[0].clone();
        let piece = pupil.options.character_piece.as_mut().unwrap();
        piece.face = FacePart::Pupil;
        piece.track = 12;
        piece.eye_mask = Some(11);
        pupil.frames[0] = vec![rectangle([5., -5.], [15., 25.], [0, 0, 0, 255])].into();
        layers.push(pupil);
        let rendered = rigged_eye_artwork(&eye_doc, 9, 5, None);
        assert_eq!(rendered.len(), 2);
        let pupil_bounds = crate::vector::control_bounds(&rendered[1]);
        assert_eq!((pupil_bounds.min.y, pupil_bounds.max.y), (5., 15.));
        eye_doc.extras.symbols[0].layers[0]
            .options
            .character_piece
            .as_mut()
            .unwrap()
            .set_motion(5, [1., 0., 0.]);
        assert_eq!(rigged_eye_artwork(&eye_doc, 9, 5, None).len(), 1);
        assert!(artwork(&doc, 9, 7, Some(5)).unwrap().0.is_empty());
        let layer = &mut doc.extras.symbols[0].layers[0];
        layer
            .options
            .character_piece
            .as_mut()
            .unwrap()
            .shape_transitions = 1 << 1;
        layer.frames[o + 1] = vec![Mark {
            points: vec![[0., 0.], [10., 10.]].into(),
            ..Default::default()
        }]
        .into();
        layer.frames[o + 2][0].points = vec![[0., 0.], [30., 10.]].into();
        doc.extras.rig.character_turns.body.set(5, 1.5);
        let saved = serde_json::to_vec(&doc).unwrap();
        let doc: crate::Document = serde_json::from_slice(&saved).unwrap();
        assert_eq!(
            artwork(&doc, 9, 5, Some(5)).unwrap().0[0].frames[0][0].points[1],
            [20., 10.]
        );
        assert!(artwork(&doc, 9, 5, Some(2)).is_none());
    }
}
