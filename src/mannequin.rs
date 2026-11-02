//! One articulated reference layer. Sixteen joints, fixed-cost forward
//! kinematics and a small number of vector shapes; never part of export ink.
use crate::{
    Editor,
    canvas::{Tool, View},
    document::Layer,
    features::LayerOptions,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};
const PARENTS: [usize; 16] = [0, 0, 1, 2, 1, 4, 5, 1, 7, 8, 0, 10, 11, 0, 13, 14];
pub const COLOR: Color32 = Color32::from_rgb(169, 133, 225);
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mannequin {
    pub origin: [f32; 2],
    pub unit: f32,
    pub heads: u8,
    pub breadth: f32,
    pub angles: [f32; 16],
}
impl Default for Mannequin {
    fn default() -> Self {
        use std::f32::consts::{FRAC_PI_2 as H, PI};
        Self {
            origin: [480., 300.],
            unit: 38.,
            heads: 8,
            breadth: 1.,
            angles: [
                0., -H, 0., 0., -H, -H, -0.15, H, H, 0.15, PI, -H, 0.08, 0., H, -0.08,
            ],
        }
    }
}
impl Mannequin {
    pub fn valid(&self) -> bool {
        (3..=8).contains(&self.heads)
            && self.unit.is_finite()
            && (2. ..=1000.).contains(&self.unit)
            && self.breadth.is_finite()
            && (0.4..=2.).contains(&self.breadth)
            && self.origin.iter().all(|v| v.is_finite() && v.abs() <= 1e7)
            && self
                .angles
                .iter()
                .all(|v| v.is_finite() && v.abs() < 10000.)
    }
    pub fn pose(&self) -> ([Pos2; 16], [f32; 16]) {
        let torso = (self.heads as f32 - 1.) * 0.39;
        let leg = (self.heads as f32 - 1.) * 0.56 * 0.5;
        let lengths = [
            0.,
            torso,
            0.22,
            0.62,
            0.65 * self.breadth,
            torso * 0.65,
            torso * 0.6,
            0.65 * self.breadth,
            torso * 0.65,
            torso * 0.6,
            0.38 * self.breadth,
            leg,
            leg,
            0.38 * self.breadth,
            leg,
            leg,
        ];
        let mut points = [crate::vector::pos(self.origin); 16];
        let mut directions = [self.angles[0]; 16];
        for i in 1..16 {
            let parent = PARENTS[i];
            directions[i] = directions[parent] + self.angles[i];
            points[i] = points[parent] + Vec2::angled(directions[i]) * lengths[i] * self.unit;
        }
        (points, directions)
    }
}
impl Editor {
    pub fn mannequin_layer(&self) -> Option<usize> {
        self.doc
            .layers
            .iter()
            .position(|l| l.options.mannequin.is_some())
    }
    pub fn add_mannequin(&mut self, p: Pos2) {
        if self.mannequin_layer().is_some() || self.doc.layers.len() >= 100 {
            return;
        }
        self.finish_pending_pen();
        self.checkpoint();
        let count = self.doc.layers[0].frames.len();
        let mut keys = vec![false; count];
        keys[0] = true;
        self.doc.layers.push(Layer {
            name: "Maniquí · referencia".into(),
            visible: true,
            frames: vec![Default::default(); count].into(),
            options: LayerOptions {
                keys: keys.into(),
                mannequin: Some(Mannequin {
                    origin: crate::vector::xy(p),
                    ..Default::default()
                }),
                ..Default::default()
            },
        });
        self.layer = self.doc.layers.len() - 1;
        self.recover();
    }
    pub fn mannequin_properties(&mut self, ui: &mut egui::Ui) {
        let Some(l) = self.mannequin_layer() else {
            crate::design::category(ui, "REFERENCIA");
            ui.small(crate::i18n::tr(
                "Haz clic en el lienzo para colocar un maniquí articulado.",
            ));
            if crate::design::button(
                ui,
                crate::design::Icon::Mannequin,
                "Colocar maniquí",
                "Una referencia por escena",
                false,
                true,
            )
            .clicked()
            {
                self.add_mannequin(Pos2::new(
                    self.doc.width as f32 / 2.,
                    self.doc.height as f32 / 2.,
                ));
            }
            return;
        };
        let mut model = self.doc.layers[l].options.mannequin.clone().unwrap();
        let original = model.clone();
        ui.colored_label(
            COLOR,
            crate::i18n::tr("Referencia de dibujo · no se exporta"),
        );
        ui.add_enabled_ui(!self.doc.layers[l].options.locked,|ui|{
            crate::design::category(ui,"PROPORCIONES");
            ui.horizontal_wrapped(|ui|{for heads in [8,7,6,4,3]{ui.selectable_value(&mut model.heads,heads,crate::i18n::message("{heads} cabezas",&[("heads",heads.to_string())]));}});
            ui.add(egui::Slider::new(&mut model.breadth,0.4..=2.).text(crate::i18n::tr("Complexión")));
            ui.add(egui::Slider::new(&mut model.unit,8. ..=150.).text(crate::i18n::tr("Escala")));
            crate::design::category(ui,"POSE");
            ui.small(crate::i18n::tr("Arrastra las articulaciones para posar. El centro de la cadera mueve todo el maniquí."));
            if crate::design::button(ui,crate::design::Icon::Rotate,"Restablecer pose","Conservar posición y proporciones",false,true).clicked(){model.angles=Mannequin::default().angles;}
        });
        if model != original {
            self.checkpoint();
            self.doc.layers[l].options.mannequin = Some(model);
            self.recover();
        }
        crate::design::category(ui, "CAPA DE REFERENCIA");
        if crate::design::button(
            ui,
            crate::design::Icon::Trash,
            "Retirar maniquí",
            "Eliminar la referencia",
            false,
            !self.doc.layers[l].options.locked,
        )
        .clicked()
        {
            self.checkpoint();
            self.doc.layers.remove(l);
            self.layer = self.layer.min(self.doc.layers.len() - 1);
            self.drawing.reference_drag = None;
            self.recover();
        }
    }
    pub fn mannequin_input(&mut self, ui: &egui::Ui, area: Rect, view: View) -> bool {
        if self.tool != Tool::Mannequin || self.playing {
            return false;
        }
        let Some(screen) = ui.input(|i| i.pointer.interact_pos()) else {
            return true;
        };
        if ui.input(|i| {
            i.key_down(egui::Key::Space) || i.pointer.button_down(egui::PointerButton::Middle)
        }) {
            return false;
        }
        if ui.input(|i| i.pointer.primary_pressed())
            && area.contains(screen)
            && ui.ctx().layer_id_at(screen) == Some(ui.layer_id())
        {
            if let Some(l) = self.mannequin_layer() {
                if self.doc.layers[l].visible && !self.doc.layers[l].options.locked {
                    let (points, _) = self.doc.layers[l]
                        .options
                        .mannequin
                        .as_ref()
                        .unwrap()
                        .pose();
                    let hit = points.iter().enumerate().min_by(|(_, a), (_, b)| {
                        view.screen(**a)
                            .distance_sq(screen)
                            .total_cmp(&view.screen(**b).distance_sq(screen))
                    });
                    if let Some((joint, p)) =
                        hit.filter(|(_, p)| view.screen(**p).distance(screen) < 16.)
                    {
                        let _ = p;
                        self.checkpoint();
                        self.layer = l;
                        self.drawing.reference_drag = Some((l, joint));
                    }
                }
            } else {
                self.add_mannequin(view.world(screen));
            }
        }
        if let Some((l, joint)) = self.drawing.reference_drag {
            if let Some(model) = self
                .doc
                .layers
                .get_mut(l)
                .and_then(|l| l.options.mannequin.as_mut())
            {
                let world = view.world(screen);
                if joint == 0 {
                    model.origin = crate::vector::xy(world);
                } else {
                    let (points, angles) = model.pose();
                    let parent = PARENTS[joint];
                    let d = world - points[parent];
                    if d.length_sq() > 0.001 {
                        model.angles[joint] =
                            crate::animation::shortest(d.angle() - angles[parent]);
                    }
                }
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                self.drawing.reference_drag = None;
                self.recover();
            }
        }
        true
    }
    pub fn paint_mannequin_layer(&self, p: &egui::Painter, view: View, index: usize) {
        for layer in self.doc.layers.iter().skip(index).take(1) {
            let Some(model) = layer.options.mannequin.as_ref().filter(|_| layer.visible) else {
                continue;
            };
            let (points, angles) = model.pose();
            let points = points.map(|p| view.screen(p));
            let unit = model.unit * view.scale;
            let opacity = layer.options.opacity;
            let white = Color32::from_white_alpha((opacity * 255.) as u8);
            let black = Color32::from_black_alpha((opacity * 255.) as u8);
            let shadow = Color32::from_rgba_unmultiplied(105, 108, 116, (opacity * 255.) as u8);
            // Limbs are capsules with a hard shadow strip and outlined joints.
            for i in [5, 6, 8, 9, 11, 12, 14, 15] {
                let a = points[PARENTS[i]];
                let b = points[i];
                let w = unit
                    * if i >= 11 {
                        if i == 11 || i == 14 { 0.43 } else { 0.31 }
                    } else if i == 5 || i == 8 {
                        0.35
                    } else {
                        0.25
                    };
                p.line_segment([a, b], Stroke::new(w + 2., black));
                p.line_segment([a, b], Stroke::new(w, white));
                let side = crate::canvas::rotate((b - a).normalized(), std::f32::consts::FRAC_PI_2)
                    * w
                    * 0.21;
                p.line_segment([a + side, b + side], Stroke::new(w * 0.38, shadow));
            }
            for (center, rx, ry, angle) in [
                (
                    points[0],
                    unit * 0.62 * model.breadth,
                    unit * 0.42,
                    angles[0],
                ),
                (
                    points[0].lerp(points[1], 0.6),
                    unit * 0.7 * model.breadth,
                    points[0].distance(points[1]) * 0.58,
                    angles[1] + std::f32::consts::FRAC_PI_2,
                ),
                (
                    points[3],
                    unit * 0.40,
                    unit * 0.52,
                    angles[3] + std::f32::consts::FRAC_PI_2,
                ),
                (
                    points[6] + Vec2::angled(angles[6]) * unit * 0.18,
                    unit * 0.19,
                    unit * 0.31,
                    angles[6] + std::f32::consts::FRAC_PI_2,
                ),
                (
                    points[9] + Vec2::angled(angles[9]) * unit * 0.18,
                    unit * 0.19,
                    unit * 0.31,
                    angles[9] + std::f32::consts::FRAC_PI_2,
                ),
                (
                    points[12]
                        + crate::canvas::rotate(egui::vec2(unit * 0.22, unit * 0.10), angles[0]),
                    unit * 0.39,
                    unit * 0.18,
                    angles[0],
                ),
                (
                    points[15]
                        + crate::canvas::rotate(egui::vec2(unit * 0.22, unit * 0.10), angles[0]),
                    unit * 0.39,
                    unit * 0.18,
                    angles[0],
                ),
            ] {
                let ring: Vec<_> = (0..32)
                    .map(|n| {
                        let a = n as f32 * std::f32::consts::TAU / 32.;
                        center
                            + crate::canvas::rotate(egui::vec2(a.cos() * rx, a.sin() * ry), angle)
                    })
                    .collect();
                p.add(egui::Shape::convex_polygon(
                    ring,
                    white,
                    Stroke::new(1.5, black),
                ));
                let shade: Vec<_> = (0..=16)
                    .map(|n| {
                        let a =
                            -std::f32::consts::FRAC_PI_2 + n as f32 * std::f32::consts::PI / 16.;
                        center
                            + crate::canvas::rotate(egui::vec2(a.cos() * rx, a.sin() * ry), angle)
                    })
                    .collect();
                p.add(egui::Shape::convex_polygon(shade, shadow, Stroke::NONE));
            }
            p.line_segment([points[1], points[2]], Stroke::new(unit * 0.23, black));
            for i in [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] {
                p.circle(points[i], unit * 0.15, white, Stroke::new(1.5, black));
            }
            if self.tool == Tool::Mannequin && !layer.options.locked {
                for point in points {
                    p.circle_stroke(point, 5., Stroke::new(1.5, COLOR));
                }
                p.circle_filled(points[0], 4., COLOR);
            }
        }
    }
}
