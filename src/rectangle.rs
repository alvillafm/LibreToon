//! Parametric rectangles keep fill and outline on one editable boundary.
use crate::{
    Editor, Mark,
    document::Node,
    vector::{pos, xy},
};
use eframe::egui::{self, Pos2, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Primitive {
    pub size: [f32; 2],
    pub frame: [[f32; 2]; 3],
    pub radii: [f32; 4], // clockwise: top left, top right, bottom right, bottom left
}
impl Primitive {
    pub(crate) fn valid(&self) -> bool {
        self.size
            .iter()
            .all(|v| v.is_finite() && *v > 0. && *v <= 1e7)
            && self
                .frame
                .iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() <= 1e7)
            && self
                .radii
                .iter()
                .all(|v| v.is_finite() && *v >= 0. && *v <= 1e7)
    }
    fn map(&self, p: Pos2) -> Pos2 {
        pos(self.frame[0])
            + (pos(self.frame[1]) - pos(self.frame[0])) * (p.x / self.size[0])
            + (pos(self.frame[2]) - pos(self.frame[0])) * (p.y / self.size[1])
    }
    fn render(&self, style: &Mark) -> Mark {
        let [w, h] = self.size;
        let mut radii = self.radii;
        // CSS-style proportional limiting prevents neighboring arcs from overlapping.
        let mut factor = 1.0_f32;
        for (edge, sum) in [
            (w, radii[0] + radii[1]),
            (h, radii[1] + radii[2]),
            (w, radii[2] + radii[3]),
            (h, radii[3] + radii[0]),
        ] {
            if sum > 0. {
                factor = factor.min(edge / sum);
            }
        }
        for r in &mut radii {
            *r *= factor;
        }
        if radii.iter().all(|r| *r == 0.) {
            let mut mark = style.clone();
            mark.nodes = Default::default();
            mark.holes = Default::default();
            mark.closed = true;
            let origin = self.map(Pos2::ZERO);
            let x = self.map(Pos2::new(w, 0.));
            let y = self.map(Pos2::new(0., h));
            let end = self.map(Pos2::new(w, h));
            mark.rectangle = (x.y - origin.y).abs() < 1e-5 && (y.x - origin.x).abs() < 1e-5;
            mark.points = if mark.rectangle {
                vec![xy(origin), xy(end)]
            } else {
                vec![xy(origin), xy(x), xy(end), xy(y)]
            }
            .into();
            mark.style.linked_outline_fill |= mark.stroke && mark.fill.is_some();
            mark.style.primitive = true;
            mark.style.rounded_rectangle = Some(Box::new(self.clone()));
            return mark;
        }
        let mut nodes = Vec::with_capacity(8);
        for ((corner, before, after, outgoing, incoming), radius) in [
            (Pos2::new(0., 0.), Vec2::Y, Vec2::X, -Vec2::Y, -Vec2::X),
            (Pos2::new(w, 0.), -Vec2::X, Vec2::Y, Vec2::X, -Vec2::Y),
            (Pos2::new(w, h), -Vec2::Y, -Vec2::X, Vec2::Y, Vec2::X),
            (Pos2::new(0., h), Vec2::X, -Vec2::Y, -Vec2::X, Vec2::Y),
        ]
        .into_iter()
        .zip(radii)
        {
            let before = corner + before * radius;
            let after = corner + after * radius;
            let outgoing = outgoing * (radius * 0.5522848);
            let incoming = incoming * (radius * 0.5522848);
            let mut a = Node::corner(xy(self.map(before)));
            if before.distance(after) < 1e-5 {
                nodes.push(a);
            } else {
                a.out_handle = xy(self.map(before + outgoing));
                let mut b = Node::corner(xy(self.map(after)));
                b.in_handle = xy(self.map(after + incoming));
                nodes.extend([a, b]);
            }
        }
        let mut mark = style.clone();
        mark.points = Default::default();
        mark.nodes = nodes.into();
        mark.holes = Default::default();
        mark.rectangle = false;
        mark.closed = true;
        mark.style.linked_outline_fill |= mark.stroke && mark.fill.is_some();
        mark.style.primitive = true;
        let mut primitive = self.clone();
        primitive.radii = radii;
        mark.style.rounded_rectangle = Some(Box::new(primitive));
        mark
    }
    fn matches(&self, mark: &Mark) -> bool {
        if !self.valid() || !mark.closed && !mark.rectangle || !mark.holes.is_empty() {
            return false;
        }
        let expected = self.render(mark);
        let a = crate::vector::cubics(mark);
        let b = crate::vector::cubics(&expected);
        a.len() == b.len()
            && a.iter()
                .zip(b.iter())
                .all(|(a, b)| a.0.iter().zip(b.0).all(|(a, b)| a.distance(b) < 0.01))
    }
    fn from_mark(mark: &Mark) -> Option<Self> {
        if mark.style.instance.is_some()
            || mark.style.media_image.is_some()
            || mark.style.text.is_some()
        {
            return None;
        }
        if let Some(primitive) = &mark.style.rounded_rectangle {
            return primitive.matches(mark).then(|| primitive.as_ref().clone());
        }
        if mark.rectangle
            && mark.points.len() == 2
            && mark.nodes.is_empty()
            && mark.holes.is_empty()
        {
            let r = egui::Rect::from_two_pos(pos(mark.points[0]), pos(mark.points[1]));
            let result = Self {
                size: [r.width(), r.height()],
                frame: [xy(r.min), xy(r.right_top()), xy(r.left_bottom())],
                radii: [0.; 4],
            };
            return result.valid().then_some(result);
        }
        None
    }
}
pub(crate) fn create(a: Pos2, b: Pos2, radii: [f32; 4], style: &Mark) -> Mark {
    let r = egui::Rect::from_two_pos(a, b);
    let primitive = Primitive {
        size: [r.width().max(1e-5), r.height().max(1e-5)],
        frame: [xy(r.min), xy(r.right_top()), xy(r.left_bottom())],
        radii: radii.map(|v| if v.is_finite() { v.clamp(0., 1e7) } else { 0. }),
    };
    primitive.render(style)
}
fn controls(ui: &mut egui::Ui, radii: &mut [f32; 4], linked: &mut bool, max: f32) -> bool {
    use crate::{
        design::{self, Icon},
        i18n::tr,
    };
    design::category(ui, "Rectangle corners");
    let mut changed = false;
    ui.horizontal(|ui| {
        design::draw(
            ui.painter(),
            egui::Rect::from_min_size(ui.cursor().min, Vec2::splat(18.)),
            Icon::Rectangle,
            crate::themes::accent(),
        );
        ui.add_space(24.);
        if ui.checkbox(linked, tr("Link corners")).changed() && *linked {
            *radii = [radii[0]; 4];
            changed = true;
        }
        if design::button(ui, Icon::Undo, "", "Reset corner radii", false, true).clicked() {
            *radii = [0.; 4];
            changed = true;
        }
    });
    if *linked {
        if ui
            .add(
                egui::Slider::new(&mut radii[0], 0. ..=max)
                    .text(tr("Corner radius"))
                    .suffix(" px"),
            )
            .changed()
        {
            *radii = [radii[0]; 4];
            changed = true;
        }
    } else {
        egui::Grid::new("corner-radii")
            .num_columns(2)
            .show(ui, |ui| {
                for (radius, label) in
                    radii
                        .iter_mut()
                        .zip(["Top left", "Top right", "Bottom right", "Bottom left"])
                {
                    ui.label(tr(label));
                    changed |= ui
                        .add(
                            egui::DragValue::new(radius)
                                .range(0. ..=max)
                                .speed(0.5)
                                .suffix(" px"),
                        )
                        .changed();
                    ui.end_row();
                }
            });
    }
    changed
}
type Selection = Vec<(usize, usize, Primitive, Mark)>;
fn selected_rectangles(editor: &Editor) -> Selection {
    let mut evaluated = std::collections::BTreeMap::new();
    editor
        .canvas
        .selected
        .iter()
        .filter_map(|&(l, m)| {
            let layer = editor.doc.layers.get(l)?;
            if layer.options.locked || !layer.visible {
                return None;
            }
            let source = layer.frames.get(editor.frame)?.get(m)?;
            if !source.rectangle && source.style.rounded_rectangle.is_none() {
                return None;
            }
            let marks = evaluated
                .entry(l)
                .or_insert_with(|| crate::animation::marks(layer, editor.frame));
            let mark = marks.get(m)?.clone();
            Some((l, m, Primitive::from_mark(&mark)?, mark))
        })
        .collect()
}
impl Editor {
    pub(crate) fn rectangle_creation_ui(&mut self, ui: &mut egui::Ui) {
        let mut radii = self
            .drawing
            .corner_radii
            .unwrap_or([self.drawing.radius; 4]);
        let mut linked = self.drawing.corner_radii.is_none();
        controls(ui, &mut radii, &mut linked, 500.);
        self.drawing.radius = radii[0];
        self.drawing.corner_radii = (!linked).then_some(radii);
    }
    pub(crate) fn rectangle_properties_ui(&mut self, ui: &mut egui::Ui) {
        if self.playing || !self.canvas.partial.is_empty() {
            return;
        }
        let updates = selected_rectangles(self);
        let Some((_, _, primitive, _)) = updates.first() else {
            return;
        };
        let mut radii = primitive.radii;
        let id = ui.scope_id().with("rectangle-corners-linked");
        let mut linked = ui
            .data(|data| data.get_temp::<bool>(id))
            .unwrap_or(radii.iter().all(|r| *r == radii[0]));
        let changed = controls(
            ui,
            &mut radii,
            &mut linked,
            primitive.size[0].max(primitive.size[1]).max(500.),
        );
        ui.data_mut(|data| data.insert_temp(id, linked));
        if changed {
            self.apply_rectangle_radii(radii);
        }
    }
    pub(crate) fn apply_rectangle_radii(&mut self, radii: [f32; 4]) -> bool {
        if self.playing
            || !self.canvas.partial.is_empty()
            || radii
                .iter()
                .any(|r| !r.is_finite() || !(0. ..=1e7).contains(r))
        {
            return false;
        }
        let updates: Vec<_> = selected_rectangles(self)
            .into_iter()
            .filter_map(|(l, m, mut primitive, mark)| {
                if primitive.radii == radii {
                    return None;
                }
                primitive.radii = radii;
                Some((l, m, primitive.render(&mark)))
            })
            .collect();
        let Some((l, m, _)) = updates.first() else {
            return false;
        };
        self.begin_property_edit(("rectangle-corners", *l, *m, self.frame));
        let selected = self.canvas.selected.clone();
        self.materialize_motion(&selected);
        for (l, m, mark) in updates {
            self.doc.layers[l].frames[self.frame][m] = mark;
        }
        self.canvas.selected = selected;
        self.canvas.node_selection = None;
        self.recover();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shape() -> Mark {
        create(
            Pos2::new(20., 30.),
            Pos2::new(220., 130.),
            [0.; 4],
            &Mark {
                fill: Some([220, 70, 100, 255]),
                width: 24.,
                ..Default::default()
            },
        )
    }
    #[test]
    fn independent_radii_clamp_without_crossing_and_preserve_style() {
        let m = create(
            Pos2::new(220., 130.),
            Pos2::new(20., 30.),
            [200., 100., 0., 50.],
            &shape(),
        );
        assert_eq!(m.nodes.len(), 7);
        assert_eq!(m.fill, shape().fill);
        assert_eq!(m.width, 24.);
        assert!(m.stroke && m.closed);
        assert!(Primitive::from_mark(&m).is_some());
        for point in crate::vector::flatten(&m, 0.05) {
            assert!(point.x >= 19.99 && point.x <= 220.01 && point.y >= 29.99 && point.y <= 130.01);
        }
        let zero = create(Pos2::ZERO, Pos2::new(100., 60.), [0.; 4], &m);
        assert!(zero.rectangle && zero.nodes.is_empty());
        assert_eq!(crate::vector::cubics(&zero).len(), 4);
    }
    #[test]
    fn primitive_survives_affine_transforms_and_manual_nodes_detach_safely() {
        let m = shape();
        let transformed = crate::vector::transform(&m, |p| {
            Pos2::new(300., 250.)
                + egui::emath::Rot2::from_angle(0.7) * (p.to_vec2() * Vec2::new(1.5, 0.8))
        });
        let mut primitive =
            Primitive::from_mark(&transformed).expect("editable after rotation and scale");
        primitive.radii = [20.; 4];
        let rounded = primitive.render(&transformed);
        assert_eq!(rounded.fill, m.fill);
        assert!(Primitive::from_mark(&rounded).is_some());
        let mut modified = rounded;
        modified.nodes[0].position[0] += 10.;
        assert!(
            Primitive::from_mark(&modified).is_none(),
            "node edits must not be overwritten by stale primitive metadata"
        );
    }
    #[test]
    fn radius_edit_moves_fill_and_stroke_together_roundtrips_and_undoes_once() {
        let ctx = egui::Context::default();
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx));
        e.doc = crate::fresh_document();
        e.recovery =
            std::env::temp_dir().join(format!("rectangle-{}.ltoon", crate::features::id()));
        e.doc.layers[0].frames[0] = vec![shape()].into();
        e.canvas.selected = vec![(0, 0)];
        let before = e.doc.layers[0].frames[0][0].clone();
        let undo = e.undo.len();
        assert!(e.apply_rectangle_radii([10.; 4]));
        assert!(e.apply_rectangle_radii([25.; 4]));
        assert_eq!(e.undo.len(), undo + 1);
        let mark = &e.doc.layers[0].frames[0][0];
        assert_eq!(mark.fill, before.fill);
        assert_eq!(mark.color, before.color);
        assert!(mark.stroke && mark.closed && mark.nodes.len() == 8);
        let data = crate::native_project::encode(&e.doc).unwrap();
        let restored = crate::native_project::decode_if_native(&data)
            .unwrap()
            .unwrap();
        assert_eq!(
            Primitive::from_mark(&restored.layers[0].frames[0][0])
                .unwrap()
                .radii,
            [25.; 4]
        );
        e.property_edit = None;
        e.history(false);
        assert!(e.doc.layers[0].frames[0][0] == before);
    }
    #[test]
    fn old_rectangles_are_editable_but_arbitrary_paths_are_not_inferred() {
        let mark = Mark {
            rectangle: true,
            points: vec![[0., 0.], [100., 60.]].into(),
            fill: Some([0, 0, 0, 255]),
            ..Default::default()
        };
        assert_eq!(Primitive::from_mark(&mark).unwrap().radii, [0.; 4]);
        let mut invalid = shape().style.rounded_rectangle.unwrap();
        invalid.radii[1] = f32::NAN;
        assert!(!invalid.valid());
    }
    #[test]
    #[ignore = "headless rounded rectangle UI visual QA"]
    fn rounded_rectangle_visual_qa() {
        let mut doc = crate::fresh_document();
        doc.layers.truncate(1);
        doc.layers[0].frames[0] = vec![create(
            Pos2::new(200., 200.),
            Pos2::new(650., 480.),
            [70., 20., 90., 0.],
            &shape(),
        )]
        .into();
        crate::manual_visuals::screenshot_setup(
            doc,
            0,
            crate::canvas::Tool::Select,
            false,
            std::path::Path::new("target/qa/rounded-rectangle.png"),
            |_, e| {
                e.canvas.selected = vec![(0, 0)];
            },
        );
    }
}
