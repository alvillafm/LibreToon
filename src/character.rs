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

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
enum FacePart {
    #[default]
    None,
    Mouth,
    Eye,
    Brow,
    Pupil,
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
    #[serde(default)]
    brow_points: [[Option<[f32; 2]>; 3]; 8],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    eye_parent: Option<u64>,
    #[serde(default)]
    vertical_tweens: [u8; 8],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    renamed_bone: Option<u64>,
}

fn is_zero(value: &u8) -> bool {
    *value == 0
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
    fn motion(&self, frame: usize) -> [f32; 3] {
        let index = self.face_motion.partition_point(|key| key.frame <= frame);
        let Some(a) = index.checked_sub(1).and_then(|i| self.face_motion.get(i)) else {
            return [0.; 3];
        };
        self.face_motion.get(index).map_or(a.axes, |b| {
            let t = (frame - a.frame) as f32 / (b.frame - a.frame) as f32;
            std::array::from_fn(|axis| a.axes[axis] + (b.axes[axis] - a.axes[axis]) * t)
        })
    }

    fn set_motion(&mut self, frame: usize, axes: [f32; 3]) {
        if frame != 0 && self.face_motion.is_empty() {
            self.face_motion.push(FaceMotion {
                frame: 0,
                axes: [0.; 3],
            });
        }
        let key = FaceMotion { frame, axes };
        match self
            .face_motion
            .binary_search_by_key(&frame, |key| key.frame)
        {
            Ok(index) => self.face_motion[index] = key,
            Err(index) => self.face_motion.insert(index, key),
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

    pub(crate) fn valid(&self) -> bool {
        self.track != 0
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
            && self
                .face_keys
                .iter()
                .all(|key| key.frame < 10000 && key.variant < VARIANTS)
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
        FacePart::None | FacePart::Pupil => None,
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
            let Some([min, max]) = piece.pupil_area[slot] else {
                let limits = limits(slot);
                return egui::vec2(
                    motion[0] * limits[usize::from(motion[0] >= 0.)],
                    motion[1] * limits[2 + usize::from(motion[1] >= 0.)],
                );
            };
            let area = egui::Rect::from_two_pos(crate::vector::pos(min), crate::vector::pos(max));
            area.center() - bounds.center() + area.size() * egui::vec2(motion[0], motion[1]) * 0.5
        };
        let next = (from + 1) % 8;
        let gaze = pupil_offset(from) * (1. - blend) + pupil_offset(next) * blend;
        let brow = |p: egui::Pos2, slot: usize| {
            if let [Some(a), Some(b), Some(c)] = piece.brow_points[slot] {
                let lerp = |x0: f32, x1: f32, v0: f32, v1: f32| {
                    let t = ((p.x - x0) / (x1 - x0).max(0.01)).clamp(0., 1.);
                    v0 * (1. - t) + v1 * t
                };
                return bounds.width()
                    * 0.35
                    * if p.x <= b[0] {
                        lerp(a[0], b[0], motion[0], motion[1])
                    } else {
                        lerp(b[0], c[0], motion[1], motion[2])
                    };
            }
            let x = ((p.x - bounds.center().x) / (bounds.width() / 2.).max(1.)).clamp(-1., 1.);
            bounds.width()
                * (motion[1] * 0.35 + motion[0] * x * 0.25 - motion[2] * (1. - x * x) * 0.3)
        };
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

pub(crate) fn rigged_eye_artwork(doc: &crate::Document, track: u64, frame: usize) -> Vec<Mark> {
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
            .filter(|p| p.eye_parent == Some(track))
        else {
            continue;
        };
        let Some(inverse) = rig::inverse(piece.matrix) else {
            continue;
        };
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
                let Some([a, b]) = piece.pupil_area[slot] else {
                    return egui::Vec2::ZERO;
                };
                let area = egui::Rect::from_two_pos(crate::vector::pos(a), crate::vector::pos(b));
                let bounds = layer.frames[slot]
                    .iter()
                    .fold(egui::Rect::NOTHING, |bounds, mark| {
                        bounds.union(crate::vector::control_bounds(mark))
                    });
                if !bounds.is_finite() {
                    return egui::Vec2::ZERO;
                }
                area.center() - bounds.center() + area.size() * egui::vec2(x, y) * 0.5
            };
            if let Some((a, b, t)) = drawn_transition(turn, piece.shape_transitions, drawn)
                && crate::animation::shape_pair(&layer.frames[a], &layer.frames[b])
            {
                let delta = offset(a) * (1. - t) + offset(b) * t;
                output.extend(
                    layer.frames[a]
                        .iter()
                        .zip(layer.frames[b].iter())
                        .map(|(a, b)| {
                            let mark = crate::animation::morph(a, b, t);
                            crate::features::affine(
                                &crate::vector::transform(&mark, |p| p + delta),
                                inverse,
                            )
                        }),
                );
            } else {
                let delta = offset(nearest);
                output.extend(layer.frames[nearest].iter().map(|mark| {
                    crate::features::affine(&crate::vector::transform(mark, |p| p + delta), inverse)
                }));
            }
            continue;
        }
        if layer.frames.len() < 32 {
            continue;
        }
        let Some(view) = (0..VIEWS.len())
            .filter(|&slot| (1..=3).any(|state| !layer.frames[state * 8 + slot].is_empty()))
            .min_by_key(|&slot| {
                let distance = slot.abs_diff(view);
                distance.min(8 - distance)
            })
        else {
            continue;
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
            if enabled && crate::animation::shape_pair(a, b) {
                return a
                    .iter()
                    .zip(b.iter())
                    .map(|(a, b)| crate::animation::morph(a, b, t))
                    .collect();
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
            if crate::animation::shape_pair(&a, &b) {
                output.extend(a.iter().zip(b.iter()).map(|(a, b)| {
                    crate::features::affine(&crate::animation::morph(a, b, blend), inverse)
                }));
                continue;
            }
        }
        output.extend(
            drawing(view)
                .iter()
                .map(|mark| crate::features::affine(mark, inverse)),
        );
    }
    output
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

impl Editor {
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
        if let Some([a, b]) = draft.or(piece.pupil_area[slot]) {
            let rect = egui::Rect::from_two_pos(
                view.screen(crate::vector::pos(a)),
                view.screen(crate::vector::pos(b)),
            );
            ui.ctx()
                .layer_painter(egui::LayerId::new(
                    egui::Order::Middle,
                    ui.scope_id().with("pupil-area-guide"),
                ))
                .with_clip_rect(area)
                .rect_stroke(
                    rect,
                    3.,
                    egui::Stroke::new(2., egui::Color32::from_rgb(105, 220, 151)),
                    egui::StrokeKind::Inside,
                );
            if released && draft.is_some() && rect.width() >= 3. && rect.height() >= 3. {
                self.checkpoint();
                self.doc.layers[self.layer]
                    .options
                    .character_piece
                    .as_mut()
                    .unwrap()
                    .pupil_area[slot] = draft;
                self.recover();
            }
        }
        if released {
            ui.data_mut(|d| d.remove::<egui::Pos2>(id));
        }
        hit.is_some() || start.is_some()
    }
    pub(crate) fn face_reference(&self) -> bool {
        self.in_character_symbol()
            && self.doc.layers[self.layer]
                .options
                .character_piece
                .as_ref()
                .is_some_and(|piece| piece.face == FacePart::Eye)
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
                            let mut axes = piece.motion(frame);
                            let mut changed = false;
                            ui.label(&layer.name);
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(115., 48.), egui::Sense::hover());
                            let points = std::array::from_fn::<_, 3, _>(|i| {
                                egui::pos2(
                                    rect.left() + 14. + i as f32 * 43.,
                                    rect.center().y + axes[i] * 16.,
                                )
                            });
                            ui.painter().add(egui::Shape::line(
                                points.to_vec(),
                                egui::Stroke::new(2., crate::themes::muted()),
                            ));
                            for (axis, point) in points.into_iter().enumerate() {
                                ui.painter()
                                    .circle_filled(point, 5., crate::themes::accent());
                                let response = ui
                                    .interact(
                                        egui::Rect::from_min_max(
                                            egui::pos2(point.x - 11., rect.top()),
                                            egui::pos2(point.x + 11., rect.bottom()),
                                        ),
                                        ui.scope_id().with(("brow-control", axis)),
                                        egui::Sense::click_and_drag(),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::ResizeVertical);
                                if let Some(pointer) = response.interact_pointer_pos()
                                    && (response.clicked() || response.dragged())
                                {
                                    axes[axis] =
                                        ((pointer.y - rect.center().y) / 16.).clamp(-1., 1.);
                                    changed = true;
                                }
                            }
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
                            if ui
                                .add(
                                    egui::Slider::new(&mut axes[0], 0. ..=1.)
                                        .text(tr("Eye closure")),
                                )
                                .changed()
                            {
                                motion_edits.push((row, axes));
                            }
                            return;
                        }
                        if piece.face == FacePart::Eye {
                            let mut closure = piece.variant(frame).min(2);
                            if ui
                                .add(
                                    egui::Slider::new(&mut closure, 0..=2)
                                        .text(tr("Eyelid closure")),
                                )
                                .changed()
                            {
                                edits.push((row, Some(closure)));
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
        if !self.doc.extras.symbols.iter().any(|symbol| {
            symbol.id == id
                && symbol.layers.iter().any(|layer| {
                    layer
                        .options
                        .character_piece
                        .as_ref()
                        .is_some_and(|piece| piece.face == FacePart::Eye)
                })
        }) {
            return;
        }
        egui::Window::new(tr("Face Rig"))
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
                    .show(ui, |ui| self.scene_eye_controls(ui, id));
            });
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
        for (row, layer) in self.doc.extras.symbols[index].layers.iter().enumerate() {
            let Some(piece) = layer
                .options
                .character_piece
                .as_ref()
                .filter(|piece| matches!(piece.face, FacePart::Eye | FacePart::Pupil))
            else {
                continue;
            };
            ui.push_id(row, |ui| {
                if piece.face == FacePart::Pupil {
                    ui.horizontal(|ui| {
                        character_row_icon(ui, Icon::Focus, &layer.name);
                        let axes = piece.motion(frame);
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(55., 55.),
                            egui::Sense::click_and_drag(),
                        );
                        let radius = 22.;
                        ui.painter().circle_stroke(
                            rect.center(),
                            radius,
                            egui::Stroke::new(1., crate::themes::muted()),
                        );
                        ui.painter().circle_filled(
                            rect.center() + egui::vec2(axes[0], axes[1]) * radius,
                            5.,
                            egui::Color32::from_rgb(105, 220, 151),
                        );
                        if let Some(pointer) = response.interact_pointer_pos()
                            && (response.clicked() || response.dragged())
                        {
                            let delta = (pointer - rect.center()) / radius;
                            let delta = delta / delta.length().max(1.);
                            edit = Some((row, [delta.x, delta.y, 0.]));
                            began = response.drag_started() || !response.dragged();
                        }
                    });
                    return;
                }
                character_row_icon(ui, Icon::Eye, &layer.name);
                let mut value = if piece.eye_parent.is_some() {
                    piece.motion(frame)[0]
                } else {
                    piece.variant(frame).min(2) as f32 * 0.5
                };
                ui.spacing_mut().slider_width = (ui.available_width() - 48.).max(65.);
                let response = ui.add(
                    egui::Slider::new(&mut value, 0. ..=1.)
                        .custom_formatter(|value, _| format!("{:.0}%", value * 100.)),
                );
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
            let piece = self.doc.extras.symbols[index].layers[row]
                .options
                .character_piece
                .as_mut()
                .unwrap();
            if piece.eye_parent.is_some() || piece.face == FacePart::Pupil {
                piece.set_motion(frame, value);
            } else {
                piece.set_variant(frame, (value[0] * 2.).round() as usize);
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
                if button(ui, Icon::Eye, "Eye", "", false, true).clicked() {
                    self.add_rigged_piece(FacePart::Eye);
                    ui.close();
                }
                if button(ui, Icon::Focus, "Pupil", "", false, true).clicked() {
                    self.add_rigged_piece(FacePart::Pupil);
                    ui.close();
                }
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
                    "Drag the pupil travel area in this view",
                    enabled,
                    true,
                )
                .clicked()
                {
                    ui.data_mut(|d| d.insert_temp(id, !enabled));
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
        let mut copy_eye = None;
        let mut rename = None;
        let mut reorder = None;
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
                        for label in VIEWS {
                            ui.strong(tr(label));
                        }
                        ui.end_row();
                        for (index, layer) in self.doc.layers.iter().enumerate() {
                            let eye = layer.options.character_piece.as_ref().filter(|p| p.face == FacePart::Eye && layer.frames.len() >= 32);
                            let fold = ui.scope_id().with(("eye-fold", index));
                            let mut collapsed = ui.data(|data| data.get_temp::<bool>(fold)).unwrap_or(false);
                            ui.horizontal(|ui| {
                                if eye.is_some() && ui.small_button(if collapsed { "▸" } else { "▾" }).clicked() {
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
                                        Some(FacePart::Pupil) => (Icon::Focus, egui::Color32::from_rgb(105, 220, 151)),
                                        _ => (Icon::Bones, egui::Color32::from_rgb(117, 181, 255)),
                                    };
                                    let response = ui.horizontal(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(15.), egui::Sense::hover());
                                        crate::design::draw(ui.painter(), rect, icon, color);
                                        ui.add(egui::Label::new(egui::RichText::new(&layer.name).color(color)).sense(egui::Sense::click()))
                                    }).inner;
                                    if response.double_clicked() { rename = Some((index, layer.name.clone())); }
                                    response.context_menu(|ui| {
                                        if button(ui, Icon::StepBack, "Move up", "", false, index > 0).clicked() {
                                            reorder = Some((index, index - 1)); ui.close();
                                        }
                                        if button(ui, Icon::StepFront, "Move down", "", false, index + 1 < self.doc.layers.len()).clicked() {
                                            reorder = Some((index, index + 1)); ui.close();
                                        }
                                    });
                                }
                            });
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
                                            if button(ui, Icon::Node, "Vertical Shape Tween", "", linked, true).clicked() {
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
        if let Some((from, to)) = reorder {
            self.finish_pending_pen();
            self.checkpoint();
            let joints = self.doc.layers[0]
                .options
                .character_piece
                .as_mut()
                .map(|p| std::mem::take(&mut p.joints));
            self.doc.layers.swap(from, to);
            if let Some(joints) = joints
                && let Some(piece) = self.doc.layers[0].options.character_piece.as_mut()
            {
                piece.joints = joints;
            }
            self.layer = if self.layer == from {
                to
            } else if self.layer == to {
                from
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
        if let Some((layer, frame, source)) = copy_eye {
            self.finish_pending_pen();
            self.checkpoint();
            let frames = &mut self.doc.layers[layer].frames;
            frames[frame] = frames[source].clone();
            selection = Some((layer, frame));
            self.recover();
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

    fn add_rigged_piece(&mut self, kind: FacePart) {
        if self.doc.layers.len() >= 100 {
            return;
        }
        let parent = &self.feature.symbol_stack.last().unwrap().0.extras.rig;
        let bound = |p: &&Piece| {
            p.eye_parent.is_none() && parent.bindings.iter().any(|b| b.target == p.track)
        };
        let head = parent
            .character_turns
            .head
            .root
            .map(|root| parent.pose_branch(root, None))
            .unwrap_or_default();
        let on_head = |p: &&Piece| {
            parent
                .bones
                .iter()
                .any(|bone| head.contains(&bone.id) && bone.targets.contains(&p.track))
        };
        let current = self.doc.layers[self.layer]
            .options
            .character_piece
            .as_ref()
            .filter(bound);
        let attachment = self.doc.layers[self.layer]
            .options
            .character_piece
            .as_ref()
            .and_then(|piece| piece.eye_parent);
        let pieces = || {
            self.doc
                .layers
                .iter()
                .filter_map(|layer| layer.options.character_piece.as_ref())
                .filter(bound)
        };
        let Some(mut piece) = pieces()
            .find(|p| Some(p.track) == attachment)
            .or_else(|| {
                current
                    .filter(on_head)
                    .or_else(|| pieces().find(on_head))
                    .or(current)
                    .or_else(|| pieces().next())
            })
            .cloned()
        else {
            return;
        };
        self.finish_pending_pen();
        self.checkpoint();
        piece.eye_parent = Some(piece.track);
        piece.renamed_bone = None;
        piece.track = crate::features::id();
        piece.face = kind;
        piece.face_keys = Default::default();
        piece.face_motion = Default::default();
        piece.vertical_tweens = Default::default();
        piece.shape_transitions = 0;
        piece.pupil_area = Default::default();
        piece.hidden = [false; 8];
        for layer in &mut self.doc.layers {
            layer.frames.resize(FACE_FRAMES, Default::default());
            layer.options.keys.resize(FACE_FRAMES, true);
        }
        self.doc.layers.push(Layer {
            name: tr(if kind == FacePart::Eye {
                "Eye"
            } else {
                "Pupil"
            })
            .into(),
            visible: true,
            frames: vec![Default::default(); FACE_FRAMES].into(),
            options: LayerOptions {
                keys: vec![true; FACE_FRAMES].into(),
                character_piece: Some(piece),
                ..Default::default()
            },
        });
        self.layer = self.doc.layers.len() - 1;
        self.frame = if kind == FacePart::Eye { 8 } else { 0 } + self.frame % 8;
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
                            renamed_bone: None,
                            pupil_area: Default::default(),
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
        if !editing && path.is_none() {
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
                    } else if let Some(path) = path
                        && button(ui, Icon::Head, "Edit Character Symbol", "", false, true)
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
                    self.activate_tool(crate::canvas::Tool::Brush);
                }
                if button(
                    ui,
                    Icon::Bones,
                    "Edit Bones",
                    "",
                    self.rigging.edit_character_bones,
                    true,
                )
                .clicked()
                {
                    self.rigging.edit_character_bones = false;
                    self.rigging.edit_character_bones = true;
                }
                ui.separator();
                let copy = button(ui, Icon::Copy, "Copy from View", "", false, true);
                egui::Popup::menu(&copy).show(|ui| {
                    for (source, name) in VIEWS.into_iter().enumerate().filter(|(i, _)| *i != frame)
                    {
                        if ui.button(tr(name)).clicked() {
                            self.finish_pending_pen();
                            self.checkpoint();
                            let frames = &mut self.doc.layers[layer].frames;
                            frames[art_frame] = frames[art_frame / 8 * 8 + source].clone();
                            if let Some(settings) =
                                self.doc.layers[0].options.character_piece.as_mut()
                            {
                                settings.joints[frame] = settings.joints[source].clone();
                            }
                            self.recover();
                            ui.close();
                        }
                    }
                });
            });
        });
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
            renamed_bone: None,
            pupil_area: Default::default(),
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
        layer.frames[26] = vec![Mark {
            points: vec![[0., 0.], [10., 0.]].into(),
            ..Default::default()
        }]
        .into();
        assert_eq!(rigged_eye_artwork(&eye_doc, 9, 5)[0].points[1], [10., 10.]);
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
        assert_eq!(rigged_eye_artwork(&eye_doc, 9, 5)[0].points[1], [20., 15.]);
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
