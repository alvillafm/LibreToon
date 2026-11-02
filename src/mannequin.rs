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
    pub depth_angles: [f32; 16],
    pub twists: [f32; 16],
    pub joint_rotation: [[f32; 3]; 16],
    pub rotation: [f32; 3],
    pub perspective: f32,
}
impl Default for Mannequin {
    fn default() -> Self {
        use std::f32::consts::{FRAC_PI_2 as H, PI};
        Self {
            origin: [480., 300.],
            depth_angles: [0.; 16],
            twists: [0.; 16],
            joint_rotation: [[0.; 3]; 16],
            rotation: [0.; 3],
            perspective: 24.,
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
        self.joint_rotation
            .iter()
            .flatten()
            .all(|v| v.is_finite() && v.abs() < 10000.)
            && self
                .depth_angles
                .iter()
                .chain(&self.twists)
                .chain(&self.rotation)
                .all(|v| v.is_finite() && v.abs() < 10000.)
            && self.perspective.is_finite()
            && (12. ..=100.).contains(&self.perspective)
            && (3..=8).contains(&self.heads)
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
    pub fn skeleton3d(&self) -> ([[f32; 3]; 16], [[[f32; 3]; 3]; 16]) {
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
        let mut points = [[0.; 3]; 16];
        let mut frames = [IDENTITY3; 16];
        for i in 0..16 {
            let (sa, ca) = self.angles[i].sin_cos();
            let (sd, cd) = self.depth_angles[i].sin_cos();
            let (st, ct) = self.twists[i].sin_cos();
            let x = [ca * cd, sa * cd, sd];
            let y = [-sa, ca, 0.];
            let z = [-ca * sd, -sa * sd, cd];
            let local = [x, add(mul(y, ct), mul(z, st)), add(mul(y, -st), mul(z, ct))];
            let parent = if i == 0 {
                IDENTITY3
            } else {
                frames[PARENTS[i]]
            };
            frames[i] = local.map(|v| basis(parent, v));
            if i > 0 {
                points[i] = add(
                    points[PARENTS[i]],
                    mul(frames[i][0], lengths[i] * self.unit),
                );
            }
            let incoming = frames[i];
            frames[i] = IDENTITY3.map(|v| basis(incoming, rotate3(v, self.joint_rotation[i])));
        }
        for p in &mut points {
            *p = rotate3(*p, self.rotation);
        }
        for f in &mut frames {
            for axis in f {
                *axis = rotate3(*axis, self.rotation);
            }
        }
        (points, frames)
    }
    pub fn project(&self, p: [f32; 3]) -> Pos2 {
        let f = self.perspective / (self.perspective + p[2] / self.unit).max(2.);
        crate::vector::pos(self.origin) + Vec2::new(p[0], p[1]) * f
    }
    pub fn pose(&self) -> ([Pos2; 16], [f32; 16]) {
        let (points, _) = self.skeleton3d();
        let points = points.map(|p| self.project(p));
        let mut angles = [self.angles[0] + self.rotation[2]; 16];
        for i in 1..16 {
            angles[i] = (points[i] - points[PARENTS[i]]).angle();
        }
        (points, angles)
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
                    unit: (self.doc.height as f32 / 12.).clamp(8., 150.),
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
            ui.horizontal_wrapped(|ui| { for heads in [8,7,6,4,3] {
                if crate::design::button(ui, crate::design::Icon::Head, &heads.to_string(),
                    &crate::i18n::message("{heads} cabezas", &[("heads",heads.to_string())]),
                    model.heads == heads, true).clicked() { model.heads=heads; }
            }});
            crate::design::category(ui, "ROTACIÓN 3D");
            ui.horizontal(|ui| {
                crate::design::button(ui, crate::design::Icon::Node, "", "Posar articulaciones",
                    !self.drawing.reference_rotate, true).clicked().then(|| self.drawing.reference_rotate=false);
                crate::design::button(ui, crate::design::Icon::Rotate, "", "Girar maniquí en 3D",
                    self.drawing.reference_rotate, true).clicked().then(|| self.drawing.reference_rotate=true);
            });
            for (axis,label) in [(0,"X"),(1,"Y"),(2,"Z")] {
                let mut degrees = model.rotation[axis].to_degrees();
                if ui.add(egui::Slider::new(&mut degrees,-180. ..=180.).text(label)).changed() { model.rotation[axis]=degrees.to_radians(); }
            }
            ui.add(egui::Slider::new(&mut model.perspective,12. ..=100.).text(crate::i18n::tr("Perspectiva")));
            crate::design::category(ui,"ARTICULACIÓN");
            egui::ComboBox::from_id_salt("mannequin-joint").selected_text(crate::i18n::tr(joint_name(self.drawing.reference_joint)))
                .show_ui(ui,|ui|{for i in 0..16 {ui.selectable_value(&mut self.drawing.reference_joint,i,crate::i18n::tr(joint_name(i)));}});
            let j=self.drawing.reference_joint;
            ui.horizontal(|ui|{for (axis,label) in ["X","Y","Z"].into_iter().enumerate() {
                let value=&mut model.joint_rotation[j][axis];
                let mut degrees=value.to_degrees();
                if ui.add(egui::DragValue::new(&mut degrees).prefix(label).suffix("°")).changed(){*value=degrees.to_radians();}
            }});
            ui.add(egui::Slider::new(&mut model.breadth,0.4..=2.).text(crate::i18n::tr("Complexión")));
            ui.add(egui::Slider::new(&mut model.unit,8. ..=150.).text(crate::i18n::tr("Escala")));
            crate::design::category(ui,"POSE");
            ui.small(crate::i18n::tr("Selecciona una articulación y arrastra X/Y/Z. Mayús: precisión · Ctrl: pasos de 15° · Esc: cancelar."));
            if crate::design::button(ui,crate::design::Icon::Rotate,"Restablecer pose","Conservar posición y proporciones",false,true).clicked(){model.angles=Mannequin::default().angles; model.depth_angles=[0.;16];model.twists=[0.;16];model.joint_rotation=[[0.;3];16];model.rotation=[0.;3];}
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
            self.drawing.reference_gizmo = None;
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
        if self.mannequin_gizmo_input(ui, area, view, screen) {
            return true;
        }
        if ui.input(|i| {
            i.key_down(egui::Key::Space) || i.pointer.button_down(egui::PointerButton::Middle)
        }) {
            return false;
        }
        let orbit_pressed = ui.input(|i| {
            i.pointer.secondary_pressed()
                || (self.drawing.reference_rotate && i.pointer.primary_pressed())
        });
        if orbit_pressed
            && area.contains(screen)
            && ui.ctx().layer_id_at(screen) == Some(ui.layer_id())
        {
            if let Some(l) = self
                .mannequin_layer()
                .filter(|l| !self.doc.layers[*l].options.locked && self.doc.layers[*l].visible)
            {
                self.checkpoint();
                self.drawing.reference_orbit = Some((
                    l,
                    screen,
                    self.doc.layers[l]
                        .options
                        .mannequin
                        .as_ref()
                        .unwrap()
                        .rotation,
                ));
                self.drawing.reference_drag = None;
            }
        }
        if let Some((l, start, rotation)) = self.drawing.reference_orbit {
            if let Some(model) = self
                .doc
                .layers
                .get_mut(l)
                .and_then(|l| l.options.mannequin.as_mut())
            {
                let delta = screen - start;
                model.rotation[0] = rotation[0] - delta.y * 0.01;
                model.rotation[1] = rotation[1] + delta.x * 0.01;
            }
            if !ui.input(|i| i.pointer.primary_down() || i.pointer.secondary_down()) {
                self.drawing.reference_orbit = None;
                self.recover();
            }
            return true;
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
                        self.drawing.reference_joint = joint;
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
                    // Fit the projected joint without changing any 3D segment length.
                    let mut step = 0.35;
                    for _ in 0..10 {
                        for axis in 0..2 {
                            let old = if axis == 0 {
                                model.angles[joint]
                            } else {
                                model.depth_angles[joint]
                            };
                            let mut best = old;
                            let mut error = model.pose().0[joint].distance_sq(world);
                            for delta in [-step, step] {
                                if axis == 0 {
                                    model.angles[joint] = old + delta;
                                } else {
                                    model.depth_angles[joint] = old + delta;
                                }
                                let candidate = model.pose().0[joint].distance_sq(world);
                                if candidate < error {
                                    error = candidate;
                                    best = old + delta;
                                }
                            }
                            if axis == 0 {
                                model.angles[joint] = best;
                            } else {
                                model.depth_angles[joint] = best;
                            }
                        }
                        step *= 0.65;
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
        let layer = &self.doc.layers[index];
        let Some(model) = layer.options.mannequin.as_ref().filter(|_| layer.visible) else {
            return;
        };
        let (points, frames) = model.skeleton3d();
        let u = model.unit;
        let mut triangles = vec![];
        let mut ellipsoid = |center: [f32; 3], axes: [[f32; 3]; 3], radii: [f32; 3]| {
            let mut vertices = vec![];
            for lat in 0..=10 {
                let a = std::f32::consts::PI * lat as f32 / 10.;
                for lon in 0..=16 {
                    let b = std::f32::consts::TAU * lon as f32 / 16.;
                    let n = [a.sin() * b.cos(), a.cos(), a.sin() * b.sin()];
                    let pos = add(
                        center,
                        basis(axes, [n[0] * radii[0], n[1] * radii[1], n[2] * radii[2]]),
                    );
                    let normal = norm(basis(
                        axes,
                        [n[0] / radii[0], n[1] / radii[1], n[2] / radii[2]],
                    ));
                    let light = dot(normal, norm([-0.4, -0.6, -1.])).max(0.);
                    let shade = (105. + light * 145.) as u8;
                    vertices.push((
                        pos,
                        Color32::from_rgba_unmultiplied(
                            shade,
                            shade,
                            (shade as u16 + 5).min(255) as u8,
                            (255. * layer.options.opacity) as u8,
                        ),
                    ));
                }
            }
            for lat in 0..10 {
                for lon in 0..16 {
                    let a = lat * 17 + lon;
                    for ids in [[a, a + 1, a + 17], [a + 1, a + 18, a + 17]] {
                        let v = ids.map(|i| vertices[i]);
                        triangles.push(((v[0].0[2] + v[1].0[2] + v[2].0[2]) / 3., v));
                    }
                }
            }
        };
        for i in [2, 3, 5, 6, 8, 9, 11, 12, 14, 15] {
            let a = points[PARENTS[i]];
            let b = points[i];
            let d = sub(b, a);
            let x = norm(d);
            let y = norm(cross([0., 0., 1.], x));
            let z = norm(cross(x, y));
            let width = u * if i >= 11 {
                if i == 11 || i == 14 { 0.43 } else { 0.31 }
            } else if i == 5 || i == 8 {
                0.35
            } else {
                0.25
            };
            ellipsoid(
                mul(add(a, b), 0.5),
                [x, y, z],
                [dot(d, d).sqrt() * 0.5 + u * 0.08, width * 0.5, width * 0.55],
            );
        }
        for i in [6, 9] {
            ellipsoid(
                add(points[i], mul(frames[i][0], u * 0.18)),
                frames[i],
                [u * 0.30, u * 0.18, u * 0.12],
            );
        }
        for i in [12, 15] {
            ellipsoid(
                add(points[i], basis(frames[0], [0., u * 0.10, -u * 0.20])),
                frames[0],
                [u * 0.20, u * 0.14, u * 0.40],
            );
        }
        ellipsoid(
            points[0],
            frames[0],
            [u * 0.62 * model.breadth, u * 0.42, u * 0.4],
        );
        ellipsoid(
            add(mul(points[0], 0.4), mul(points[1], 0.6)),
            frames[1],
            [
                dot(sub(points[1], points[0]), sub(points[1], points[0])).sqrt() * 0.58,
                u * 0.7 * model.breadth,
                u * 0.43,
            ],
        );
        ellipsoid(points[3], frames[3], [u * 0.52, u * 0.4, u * 0.39]);
        for i in [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] {
            ellipsoid(points[i], frames[i], [u * 0.18, u * 0.18, u * 0.18]);
        }
        triangles.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut mesh = egui::Mesh::default();
        for (_, v) in triangles {
            let base = mesh.vertices.len() as u32;
            for (position, color) in v {
                mesh.colored_vertex(view.screen(model.project(position)), color);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        p.add(egui::Shape::mesh(mesh));
        // Facial landmarks rotate with the head and disappear behind it.
        if frames[3][2][2] > 0.1 {
            for side in [-1., 1.] {
                let eye = add(
                    points[3],
                    basis(frames[3], [u * 0.10, u * 0.14 * side, -u * 0.35]),
                );
                p.circle_filled(
                    view.screen(model.project(eye)),
                    (u * view.scale * 0.035).max(1.),
                    Color32::from_gray(40),
                );
            }
            let a = add(
                points[3],
                basis(frames[3], [-u * 0.14, -u * 0.10, -u * 0.36]),
            );
            let b = add(
                points[3],
                basis(frames[3], [-u * 0.14, u * 0.10, -u * 0.36]),
            );
            p.line_segment(
                [view.screen(model.project(a)), view.screen(model.project(b))],
                Stroke::new(1., Color32::from_gray(40)),
            );
        }
        if self.tool == Tool::Mannequin && !layer.options.locked {
            for (i, point) in points.iter().enumerate() {
                p.circle_stroke(
                    view.screen(model.project(*point)),
                    if i == self.drawing.reference_joint {
                        6.
                    } else {
                        4.
                    },
                    Stroke::new(1.3, COLOR),
                );
            }
        }
    }
}
type V3 = [f32; 3];
const IDENTITY3: [V3; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: V3, b: f32) -> V3 {
    a.map(|v| v * b)
}
fn dot(a: V3, b: V3) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn norm(a: V3) -> V3 {
    let n = dot(a, a).sqrt();
    if n < 1e-6 {
        [1., 0., 0.]
    } else {
        mul(a, 1. / n)
    }
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn basis(m: [V3; 3], v: V3) -> V3 {
    add(add(mul(m[0], v[0]), mul(m[1], v[1])), mul(m[2], v[2]))
}
fn rotate3(mut v: V3, r: V3) -> V3 {
    let (s, c) = r[0].sin_cos();
    v = [v[0], c * v[1] - s * v[2], s * v[1] + c * v[2]];
    let (s, c) = r[1].sin_cos();
    v = [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]];
    let (s, c) = r[2].sin_cos();
    [c * v[0] - s * v[1], s * v[0] + c * v[1], v[2]]
}
fn joint_name(i: usize) -> &'static str {
    [
        "Cadera",
        "Torso",
        "Cuello",
        "Cabeza",
        "Hombro izquierdo",
        "Codo izquierdo",
        "Mano izquierda",
        "Hombro derecho",
        "Codo derecho",
        "Mano derecha",
        "Cadera izquierda",
        "Rodilla izquierda",
        "Pie izquierdo",
        "Cadera derecha",
        "Rodilla derecha",
        "Pie derecho",
    ][i.min(15)]
}

#[derive(Clone)]
pub struct GizmoGeometry {
    pub center: Pos2,
    pub rings: [[Pos2; 65]; 3],
}
impl GizmoGeometry {
    fn hit(&self, p: Pos2) -> Option<(usize, usize)> {
        for axis in 0..3 {
            if p.distance(self.rings[axis][8]) < 11. {
                return Some((axis, 8));
            }
        }
        if p.distance(self.center) < 14. {
            return None;
        }
        (0..3)
            .flat_map(|axis| {
                (0..64).map(move |n| {
                    let a = self.rings[axis][n];
                    let b = self.rings[axis][n + 1];
                    let d = b - a;
                    let t = ((p - a).dot(d) / d.length_sq().max(1e-6)).clamp(0., 1.);
                    (axis, n, p.distance(a + d * t))
                })
            })
            .filter(|(_, _, d)| *d < 7.)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(axis, n, _)| (axis, n))
    }
    fn phase(&self, axis: usize, p: Pos2) -> Option<f32> {
        let a = (self.rings[axis][0] - self.rings[axis][32]) * 0.5;
        let b = (self.rings[axis][16] - self.rings[axis][48]) * 0.5;
        let det = a.x * b.y - a.y * b.x;
        if det.abs() < a.length() * b.length() * 0.2 || a.length() < 3. || b.length() < 3. {
            return None;
        }
        let d = p - self.center;
        Some(((a.x * d.y - a.y * d.x) / det).atan2((d.x * b.y - d.y * b.x) / det))
    }
}
#[derive(Clone)]
pub struct GizmoDrag {
    layer: usize,
    joint: usize,
    axis: usize,
    geometry: GizmoGeometry,
    last_pointer: Pos2,
    last_phase: Option<f32>,
    tangent: Vec2,
    amount: f32,
    original: f32,
}
impl Mannequin {
    pub fn rotation_gizmo(&self, view: View, joint: usize) -> GizmoGeometry {
        let (points, frames) = self.skeleton3d();
        let joint = joint.min(15);
        let center = view.screen(self.project(points[joint]));
        let perspective =
            self.perspective / (self.perspective + points[joint][2] / self.unit).max(2.);
        let radius = 48. / (view.scale * perspective).max(0.001);
        let local = self.joint_rotation[joint];
        let incoming = IDENTITY3.map(|v| basis(frames[joint], unrotate3(v, local)));
        let axis_frames = [
            frames[joint],
            IDENTITY3.map(|v| basis(incoming, rotate3(v, [0., 0., local[2]]))),
            incoming,
        ];
        let rings = std::array::from_fn(|axis| {
            std::array::from_fn(|n| {
                let phase = n as f32 * std::f32::consts::TAU / 64.;
                let a = axis_frames[axis][(axis + 1) % 3];
                let b = axis_frames[axis][(axis + 2) % 3];
                view.screen(self.project(add(
                    points[joint],
                    mul(add(mul(a, phase.cos()), mul(b, phase.sin())), radius),
                )))
            })
        });
        GizmoGeometry { center, rings }
    }
}
impl Editor {
    fn mannequin_gizmo_input(
        &mut self,
        ui: &egui::Ui,
        area: Rect,
        view: View,
        screen: Pos2,
    ) -> bool {
        if let Some(mut drag) = self.drawing.reference_gizmo.take() {
            let editable =
                self.doc.layers.get(drag.layer).is_some_and(|l| {
                    l.visible && !l.options.locked && l.options.mannequin.is_some()
                });
            if !editable {
                return true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.history(false);
                return true;
            }
            let phase = drag.geometry.phase(drag.axis, screen);
            let delta = if let (Some(now), Some(previous)) = (phase, drag.last_phase) {
                if screen.distance(drag.geometry.center) > 4. {
                    crate::animation::shortest(now - previous)
                } else {
                    0.
                }
            } else {
                (screen - drag.last_pointer).dot(drag.tangent) / drag.tangent.length_sq().max(64.)
            };
            drag.amount += delta
                * if ui.input(|i| i.modifiers.shift) {
                    0.2
                } else {
                    1.
                };
            drag.last_pointer = screen;
            drag.last_phase = phase;
            let mut angle = drag.original + drag.amount;
            if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                let step = 15f32.to_radians();
                angle = (angle / step).round() * step;
            }
            self.doc.layers[drag.layer]
                .options
                .mannequin
                .as_mut()
                .unwrap()
                .joint_rotation[drag.joint][drag.axis] = crate::animation::shortest(angle);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if ui.input(|i| i.pointer.primary_down()) {
                self.drawing.reference_gizmo = Some(drag);
            } else {
                self.recover();
            }
            ui.ctx().request_repaint();
            return true;
        }
        if self.drawing.reference_rotate
            || self.drawing.reference_drag.is_some()
            || self.drawing.reference_orbit.is_some()
            || ui.input(|i| {
                i.key_down(egui::Key::Space) || i.pointer.button_down(egui::PointerButton::Middle)
            })
        {
            return false;
        }
        let Some(layer) = self
            .mannequin_layer()
            .filter(|l| self.doc.layers[*l].visible && !self.doc.layers[*l].options.locked)
        else {
            return false;
        };
        if !area.contains(screen) || ui.ctx().layer_id_at(screen) != Some(ui.layer_id()) {
            return false;
        }
        let joint = self.drawing.reference_joint.min(15);
        let model = self.doc.layers[layer].options.mannequin.as_ref().unwrap();
        if model
            .pose()
            .0
            .iter()
            .enumerate()
            .any(|(i, p)| i != joint && view.screen(*p).distance(screen) < 9.)
        {
            return false;
        }
        let geometry = model.rotation_gizmo(view, joint);
        let Some((axis, n)) = geometry.hit(screen) else {
            return false;
        };
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        if ui.input(|i| i.pointer.primary_pressed()) {
            let original = model.joint_rotation[joint][axis];
            let phase = geometry.phase(axis, screen);
            let tangent = (geometry.rings[axis][(n + 1) % 64]
                - geometry.rings[axis][(n + 63) % 64])
                / (2. * std::f32::consts::TAU / 64.);
            self.checkpoint();
            self.drawing.reference_drag = None;
            self.drawing.reference_gizmo = Some(GizmoDrag {
                layer,
                joint,
                axis,
                geometry,
                last_pointer: screen,
                last_phase: phase,
                tangent,
                amount: 0.,
                original,
            });
            self.layer = layer;
        }
        true
    }
    pub fn paint_mannequin_gizmo(&self, p: &egui::Painter, view: View) {
        if self.tool != Tool::Mannequin || self.playing || self.drawing.reference_rotate {
            return;
        }
        let Some(layer) = self
            .mannequin_layer()
            .filter(|l| self.doc.layers[*l].visible && !self.doc.layers[*l].options.locked)
        else {
            return;
        };
        let model = self.doc.layers[layer].options.mannequin.as_ref().unwrap();
        let joint = self.drawing.reference_joint.min(15);
        let drag = self
            .drawing
            .reference_gizmo
            .as_ref()
            .filter(|d| d.layer == layer && d.joint == joint);
        let geometry =
            drag.map_or_else(|| model.rotation_gizmo(view, joint), |d| d.geometry.clone());
        let hover = p
            .ctx()
            .input(|i| i.pointer.hover_pos())
            .and_then(|pos| geometry.hit(pos))
            .map(|h| h.0);
        if p.ctx().input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|q| p.clip_rect().contains(q))
        }) && (hover.is_some() || drag.is_some())
        {
            p.ctx().set_cursor_icon(if drag.is_some() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        let colors = [
            Color32::from_rgb(240, 90, 85),
            Color32::from_rgb(70, 195, 105),
            Color32::from_rgb(75, 145, 255),
        ];
        for axis in 0..3 {
            let active = drag.is_some_and(|d| d.axis == axis);
            let width = if active {
                3.5
            } else if hover == Some(axis) {
                3.
            } else {
                2.
            };
            p.add(egui::Shape::line(
                geometry.rings[axis].to_vec(),
                Stroke::new(width + 2., Color32::from_black_alpha(150)),
            ));
            p.add(egui::Shape::line(
                geometry.rings[axis].to_vec(),
                Stroke::new(width, colors[axis]),
            ));
            let badge = geometry.rings[axis][8];
            p.circle_filled(badge, 9., colors[axis]);
            p.text(
                badge,
                egui::Align2::CENTER_CENTER,
                ["X", "Y", "Z"][axis],
                egui::FontId::proportional(11.),
                Color32::BLACK,
            );
        }
        p.circle_stroke(geometry.center, 8., Stroke::new(1.5, Color32::WHITE));
        if let Some(d) = drag {
            p.text(
                d.last_pointer + Vec2::new(14., -18.),
                egui::Align2::LEFT_BOTTOM,
                format!(
                    "{} · {:.1}°",
                    ["X", "Y", "Z"][d.axis],
                    model.joint_rotation[joint][d.axis].to_degrees()
                ),
                egui::FontId::proportional(12.),
                colors[d.axis],
            );
        }
    }
}

fn unrotate3(v: V3, r: V3) -> V3 {
    let v = rotate3(v, [0., 0., -r[2]]);
    let v = rotate3(v, [0., -r[1], 0.]);
    rotate3(v, [-r[0], 0., 0.])
}

#[cfg(test)]
mod gizmo_tests {
    use super::*;
    fn setup() -> (Editor, egui::Context, View) {
        let ctx = egui::Context::default();
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        e.doc = crate::fresh_document();
        e.recovery = std::env::temp_dir().join(format!(
            "gizmo-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        e.add_mannequin(Pos2::new(300., 350.));
        e.doc.layers[e.layer]
            .options
            .mannequin
            .as_mut()
            .unwrap()
            .unit = 30.;
        e.tool = Tool::Mannequin;
        e.drawing.reference_joint = 4;
        (
            e,
            ctx,
            View {
                center: Pos2::ZERO,
                stage_center: Pos2::ZERO,
                angle: 0.,
                scale: 1.,
            },
        )
    }
    fn tick(ctx: &egui::Context, e: &mut Editor, view: View, events: Vec<egui::Event>) {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(700.))),
                events,
                ..Default::default()
            },
            |ui| {
                e.mannequin_input(ui, ui.max_rect(), view);
                e.paint_mannequin_gizmo(ui.painter(), view);
            },
        );
        out.textures_delta.clear();
    }
    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn handle(model: &Mannequin, view: View, axis: usize) -> (Pos2, usize, GizmoGeometry) {
        let g = model.rotation_gizmo(view, 4);
        let joints = model.pose().0;
        let n = (0..64)
            .find(|&n| {
                let p = g.rings[axis][n];
                g.hit(p).is_some_and(|(a, _)| a == axis)
                    && joints.iter().all(|q| view.screen(*q).distance(p) > 12.)
            })
            .expect("a reachable rotation ring");
        (g.rings[axis][n], n, g)
    }
    #[test]
    fn each_axis_rotates_at_the_joint_preserves_lengths_and_undoes_as_one_gesture() {
        for axis in 0..3 {
            let (mut e, ctx, view) = setup();
            e.doc.layers[e.layer]
                .options
                .mannequin
                .as_mut()
                .unwrap()
                .rotation = [0.3, 0.5, 0.2];
            let before = e.doc.layers[e.layer].options.mannequin.clone().unwrap();
            let points = before.skeleton3d().0;
            let (p, n, g) = handle(&before, view, axis);
            let q = g.rings[axis][(n + 5) % 64];
            tick(&ctx, &mut e, view, vec![]);
            tick(&ctx, &mut e, view, vec![egui::Event::PointerMoved(p)]);
            tick(&ctx, &mut e, view, vec![button(p, true)]);
            assert!(e.drawing.reference_gizmo.is_some());
            tick(&ctx, &mut e, view, vec![egui::Event::PointerMoved(q)]);
            tick(&ctx, &mut e, view, vec![button(q, false)]);
            let after = e.doc.layers[e.layer].options.mannequin.as_ref().unwrap();
            assert!(after.joint_rotation[4][axis].abs() > 0.05);
            for other in 0..3 {
                if other != axis {
                    assert_eq!(after.joint_rotation[4][other], 0.);
                }
            }
            let posed = after.skeleton3d().0;
            assert!(
                dot(sub(points[4], posed[4]), sub(points[4], posed[4])) < 1e-5,
                "pivot moved"
            );
            for i in 1..16 {
                let length =
                    |p: [V3; 16]| dot(sub(p[i], p[PARENTS[i]]), sub(p[i], p[PARENTS[i]])).sqrt();
                assert!((length(points) - length(posed)).abs() < 0.001);
            }
            e.history(false);
            assert_eq!(
                e.doc.layers[e.layer]
                    .options
                    .mannequin
                    .as_ref()
                    .unwrap()
                    .joint_rotation,
                before.joint_rotation
            );
            assert!(e.doc.validate().is_ok());
        }
    }
    #[test]
    fn edge_on_rings_still_drag_and_escape_restores_the_pose() {
        let (mut e, ctx, view) = setup();
        let before = e.doc.layers[e.layer].options.mannequin.clone().unwrap();
        let (p, n, g) = handle(&before, view, 0);
        let tangent = g.rings[0][(n + 1) % 64] - g.rings[0][(n + 63) % 64];
        let q = p + tangent.normalized() * 20.;
        tick(&ctx, &mut e, view, vec![]);
        tick(
            &ctx,
            &mut e,
            view,
            vec![egui::Event::PointerMoved(p), button(p, true)],
        );
        tick(&ctx, &mut e, view, vec![egui::Event::PointerMoved(q)]);
        assert!(
            e.doc.layers[e.layer]
                .options
                .mannequin
                .as_ref()
                .unwrap()
                .joint_rotation[4][0]
                .abs()
                > 0.01
        );
        tick(
            &ctx,
            &mut e,
            view,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(e.drawing.reference_gizmo.is_none());
        assert_eq!(
            e.doc.layers[e.layer]
                .options
                .mannequin
                .as_ref()
                .unwrap()
                .joint_rotation,
            before.joint_rotation
        );
        tick(&ctx, &mut e, view, vec![button(q, false)]);
        e.doc.layers[e.layer].options.locked = true;
        tick(
            &ctx,
            &mut e,
            view,
            vec![egui::Event::PointerMoved(p), button(p, true)],
        );
        assert!(e.drawing.reference_gizmo.is_none());
    }
}
