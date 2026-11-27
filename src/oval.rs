//! Editable elliptical sectors and rings; ordinary path edits remain authoritative.
use crate::{
    Editor, Mark,
    document::Node,
    vector::{pos, xy},
};
use eframe::egui::{self, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Primitive {
    pub frame: [[f32; 2]; 3], // center and the ends of the two radius vectors
    pub params: [f32; 3],     // start/end degrees clockwise, inner radius percent
}
impl Primitive {
    pub(crate) fn valid(&self) -> bool {
        self.frame
            .iter()
            .flatten()
            .all(|v| v.is_finite() && v.abs() <= 1e7)
            && valid_params(self.params)
            && (pos(self.frame[1]) - pos(self.frame[0])).length_sq() > 1e-12
            && (pos(self.frame[2]) - pos(self.frame[0])).length_sq() > 1e-12
    }
    fn point(&self, t: f32, r: f32) -> Pos2 {
        pos(self.frame[0])
            + (pos(self.frame[1]) - pos(self.frame[0])) * (t.cos() * r)
            + (pos(self.frame[2]) - pos(self.frame[0])) * (t.sin() * r)
    }
    fn tangent(&self, t: f32, r: f32) -> Vec2 {
        (pos(self.frame[1]) - pos(self.frame[0])) * (-t.sin() * r)
            + (pos(self.frame[2]) - pos(self.frame[0])) * (t.cos() * r)
    }
    fn arc(&self, start: f32, sweep: f32, r: f32, full: bool) -> Vec<Node> {
        let count = (sweep.abs() / std::f32::consts::FRAC_PI_2).ceil().max(1.) as usize;
        let delta = sweep / count as f32;
        let mut nodes = vec![Node::corner(xy(self.point(start, r)))];
        for i in 0..count {
            let a = start + i as f32 * delta;
            let b = a + delta;
            let k = 4. / 3. * (delta / 4.).tan();
            nodes.last_mut().unwrap().out_handle = xy(self.point(a, r) + self.tangent(a, r) * k);
            let mut next = Node::corner(xy(self.point(b, r)));
            next.in_handle = xy(self.point(b, r) - self.tangent(b, r) * k);
            nodes.push(next);
        }
        if full {
            let last = nodes.pop().unwrap();
            nodes[0].in_handle = last.in_handle;
        }
        nodes
    }
    fn render(&self, style: &Mark) -> Mark {
        let mut mark = style.clone();
        let start = self.params[0].to_radians();
        let raw = (self.params[1] - self.params[0]).rem_euclid(360.);
        let full = raw < 0.0001;
        let sweep = if full {
            std::f32::consts::TAU
        } else {
            raw.to_radians()
        };
        let inner = self.params[2] / 100.;
        let mut nodes = self.arc(start, sweep, 1., full);
        mark.holes = Default::default();
        if full && inner > 0. {
            let mut hole = Mark {
                nodes: self.arc(start, -sweep, inner, true).into(),
                closed: true,
                ..Default::default()
            };
            hole.stroke = false;
            mark.holes.push(
                crate::vector::flatten(&hole, 0.02)
                    .into_iter()
                    .map(xy)
                    .collect(),
            );
        } else if !full {
            if inner > 0. {
                nodes.extend(self.arc(start + sweep, -sweep, inner, false));
            } else {
                nodes.push(Node::corner(self.frame[0]));
            }
        }
        mark.nodes = nodes.into();
        mark.points = Default::default();
        mark.rectangle = false;
        mark.closed = true;
        mark.style.rounded_rectangle = None;
        mark.style.primitive = true;
        mark.style.linked_outline_fill |= mark.stroke && mark.fill.is_some();
        mark.style.oval = Some(Box::new(self.clone()));
        mark
    }
    fn matches(&self, mark: &Mark) -> bool {
        if !self.valid() || !mark.closed {
            return false;
        }
        let expected = self.render(mark);
        if mark.nodes.len() != expected.nodes.len() || mark.holes.len() != expected.holes.len() {
            return false;
        }
        let same = mark.nodes.iter().zip(expected.nodes.iter()).all(|(a, b)| {
            [a.position, a.in_handle, a.out_handle]
                .into_iter()
                .zip([b.position, b.in_handle, b.out_handle])
                .all(|(a, b)| pos(a).distance(pos(b)) < 0.01)
        });
        if !same {
            return false;
        }
        if mark.holes.is_empty() {
            return true;
        }
        let x = pos(self.frame[1]) - pos(self.frame[0]);
        let y = pos(self.frame[2]) - pos(self.frame[0]);
        let det = x.x * y.y - x.y * y.x;
        if det.abs() < 1e-10 {
            return false;
        }
        mark.holes[0].iter().all(|p| {
            let d = pos(*p) - pos(self.frame[0]);
            let u = (d.x * y.y - d.y * y.x) / det;
            let v = (x.x * d.y - x.y * d.x) / det;
            (u.hypot(v) - self.params[2] / 100.).abs() < 0.001
        })
    }
    fn from_mark(mark: &Mark) -> Option<Self> {
        if mark.style.instance.is_some()
            || mark.style.text.is_some()
            || mark.style.media_image.is_some()
        {
            return None;
        }
        if let Some(p) = &mark.style.oval {
            return p.matches(mark).then(|| p.as_ref().clone());
        }
        if mark.nodes.len() != 4 || !mark.closed || !mark.holes.is_empty() {
            return None;
        }
        let center = pos(mark.nodes[0].position).lerp(pos(mark.nodes[2].position), 0.5);
        let p = Self {
            frame: [xy(center), mark.nodes[0].position, mark.nodes[1].position],
            params: [0., 360., 0.],
        };
        p.matches(mark).then_some(p)
    }
}
fn valid_params(p: [f32; 3]) -> bool {
    p.iter().all(|v| v.is_finite())
        && (0. ..=360.).contains(&p[0])
        && (0. ..=360.).contains(&p[1])
        && (0. ..=99.9).contains(&p[2])
}
pub(crate) fn create(a: Pos2, b: Pos2, params: [f32; 3], style: &Mark) -> Mark {
    let r = Rect::from_two_pos(a, b);
    let c = r.center();
    Primitive {
        frame: [
            xy(c),
            xy(c + Vec2::new(r.width().max(1e-5) / 2., 0.)),
            xy(c + Vec2::new(0., r.height().max(1e-5) / 2.)),
        ],
        params,
    }
    .render(style)
}
fn controls(ui: &mut egui::Ui, params: &mut [f32; 3]) -> bool {
    crate::design::category(ui, "Oval primitive");
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for (icon, label, value) in [
            (crate::design::Icon::Ellipse, "Full oval", [0., 360., 0.]),
            (crate::design::Icon::CapRound, "Half circle", [0., 180., 0.]),
            (crate::design::Icon::Focus, "Ring", [0., 360., 50.]),
        ] {
            if crate::design::button(ui, icon, label, "", *params == value, true).clicked() {
                *params = value;
                changed = true;
            }
        }
    });
    egui::Grid::new("oval-parameters")
        .num_columns(2)
        .show(ui, |ui| {
            for (i, label) in ["Start angle", "End angle", "Inner radius"]
                .into_iter()
                .enumerate()
            {
                ui.label(crate::i18n::tr(label));
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut params[i])
                            .range(0. ..=if i == 2 { 99.9 } else { 360. })
                            .speed(0.5)
                            .suffix(if i == 2 { " %" } else { "°" }),
                    )
                    .changed();
                ui.end_row();
            }
        });
    changed
}
fn selected(e: &Editor) -> Vec<(usize, usize, Primitive, Mark)> {
    let mut evaluated = std::collections::BTreeMap::new();
    e.canvas
        .selected
        .iter()
        .filter_map(|&(l, m)| {
            let layer = e.doc.layers.get(l)?;
            if layer.options.locked || !layer.visible {
                return None;
            }
            let raw = layer.frames.get(e.frame)?.get(m)?;
            if raw.style.oval.is_none() && raw.nodes.len() != 4 {
                return None;
            }
            let marks = evaluated
                .entry(l)
                .or_insert_with(|| crate::animation::marks(layer, e.frame));
            let mark = marks.get(m)?.clone();
            Some((l, m, Primitive::from_mark(&mark)?, mark))
        })
        .collect()
}
impl Editor {
    pub(crate) fn oval_creation_ui(&mut self, ui: &mut egui::Ui) {
        controls(ui, &mut self.drawing.oval);
    }
    pub(crate) fn oval_properties_ui(&mut self, ui: &mut egui::Ui) {
        if self.playing || !self.canvas.partial.is_empty() {
            return;
        }
        if let Some((_, _, p, _)) = selected(self).first() {
            let mut params = p.params;
            if controls(ui, &mut params) {
                self.apply_oval_params(params);
            }
        }
    }
    pub(crate) fn apply_oval_params(&mut self, params: [f32; 3]) -> bool {
        if self.playing || !self.canvas.partial.is_empty() || !valid_params(params) {
            return false;
        }
        let updates: Vec<_> = selected(self)
            .into_iter()
            .filter_map(|(l, m, mut p, mark)| {
                if p.params == params {
                    return None;
                }
                p.params = params;
                Some((l, m, p.render(&mark)))
            })
            .collect();
        let Some((l, m, _)) = updates.first() else {
            return false;
        };
        self.begin_property_edit(("oval-parameters", *l, *m, self.frame));
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
    fn style() -> Mark {
        Mark {
            fill: Some([230, 70, 120, 255]),
            width: 3.,
            ..Default::default()
        }
    }
    #[test]
    fn oval_sectors_rings_and_wrapped_angles_have_expected_fill() {
        let full = create(Pos2::ZERO, Pos2::new(200., 100.), [0., 360., 0.], &style());
        assert!(crate::vector::contains_fill(&full, Pos2::new(100., 50.)));
        let half = create(Pos2::ZERO, Pos2::new(200., 100.), [0., 180., 0.], &style());
        assert!(crate::vector::contains_fill(&half, Pos2::new(100., 70.)));
        assert!(!crate::vector::contains_fill(&half, Pos2::new(100., 30.)));
        let ring = create(Pos2::ZERO, Pos2::new(200., 100.), [0., 360., 50.], &style());
        assert!(!crate::vector::contains_fill(&ring, Pos2::new(100., 50.)));
        assert!(crate::vector::contains_fill(&ring, Pos2::new(180., 50.)));
        assert_eq!(ring.holes.len(), 1);
        let arc = create(
            Pos2::ZERO,
            Pos2::new(200., 100.),
            [300., 60., 50.],
            &style(),
        );
        assert!(crate::vector::contains_fill(&arc, Pos2::new(180., 50.)));
        assert!(!crate::vector::contains_fill(&arc, Pos2::new(20., 50.)));
        assert!(Primitive::from_mark(&arc).is_some());
    }
    #[test]
    fn oval_parameters_survive_native_data_and_affine_transforms() {
        let mark = create(Pos2::ZERO, Pos2::new(200., 100.), [0., 360., 60.], &style());
        let moved = crate::vector::transform(&mark, |p| {
            Pos2::new(p.x * 2. + p.y * 0.3 + 50., p.y * 3. + 20.)
        });
        let bytes = serde_json::to_vec(&moved).unwrap();
        let restored: Mark = serde_json::from_slice(&bytes).unwrap();
        assert!(Primitive::from_mark(&restored).is_some());
        assert_eq!(restored.fill, mark.fill);
        let mut edited = restored;
        edited.nodes[0].out_handle[0] += 30.;
        assert!(
            Primitive::from_mark(&edited).is_none(),
            "manual edits must not be overwritten"
        );
    }
    #[test]
    fn legacy_ellipses_are_recognized_but_rectangles_are_not() {
        let mark = crate::editing::shape(
            crate::canvas::Tool::Ellipse,
            Pos2::ZERO,
            Pos2::new(200., 100.),
            4,
            false,
            0.5,
            &style(),
        );
        assert!(Primitive::from_mark(&mark).is_some());
        let mut other = mark;
        for n in &mut other.nodes {
            n.in_handle = n.position;
            n.out_handle = n.position;
        }
        assert!(Primitive::from_mark(&other).is_none());
    }
    #[test]
    fn oval_inspector_edits_group_undo_and_preserve_both_channels() {
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        e.doc = crate::Document::default();
        e.layer = 0;
        e.frame = 0;
        e.recovery = std::env::temp_dir().join(format!("oval-edit-{}.json", crate::features::id()));
        e.doc.layers[0].frames[0] = vec![create(
            Pos2::ZERO,
            Pos2::new(200., 100.),
            [0., 360., 0.],
            &style(),
        )]
        .into();
        e.canvas.selected = vec![(0, 0)];
        let history = e.undo.len();
        assert!(e.apply_oval_params([0., 360., 50.]));
        assert!(e.apply_oval_params([0., 360., 60.]));
        assert_eq!(e.undo.len(), history + 1);
        assert!(e.doc.layers[0].frames[0][0].stroke);
        assert_eq!(e.doc.layers[0].frames[0][0].fill, style().fill);
        e.history(false);
        assert!(e.doc.layers[0].frames[0][0].holes.is_empty());
        e.history(true);
        assert_eq!(e.doc.layers[0].frames[0][0].holes.len(), 1);
        assert!(!e.apply_oval_params([0., 360., 100.]));
    }
    #[test]
    #[ignore = "headless oval primitive UI visual QA"]
    fn oval_visual_qa() {
        let mut doc = crate::fresh_document();
        doc.layers.truncate(1);
        doc.layers[0].frames[0] = vec![create(
            Pos2::new(200., 120.),
            Pos2::new(700., 430.),
            [300., 220., 55.],
            &style(),
        )]
        .into();
        crate::manual_visuals::screenshot_setup(
            doc,
            0,
            crate::canvas::Tool::Select,
            false,
            std::path::Path::new("target/qa/oval-primitive.png"),
            |_, e| {
                e.canvas.selected = vec![(0, 0)];
            },
        );
    }
}
