use crate::{
    Editor,
    design::{ACCENT, Icon},
    document::Mark,
};
use eframe::egui::{self, Color32, CursorIcon, PointerButton, Pos2, Rect, Sense, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Brush,
    Rectangle,
    Select,
    Transform,
    RotateView,
}
impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Self::Brush => "Pincel",
            Self::Rectangle => "Rectángulo",
            Self::Select => "Selección",
            Self::Transform => "Transformación libre",
            Self::RotateView => "Rotar vista",
        }
    }
    pub fn icon(self) -> Icon {
        match self {
            Self::Brush => Icon::Brush,
            Self::Rectangle => Icon::Rectangle,
            Self::Select => Icon::Select,
            Self::Transform => Icon::Transform,
            Self::RotateView => Icon::Rotate,
        }
    }
}
#[derive(Clone, Copy)]
pub struct View {
    pub center: Pos2,
    pub scale: f32,
    pub angle: f32,
}
pub fn rotate(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y)
}
impl View {
    pub fn screen(self, p: Pos2) -> Pos2 {
        self.center + rotate(p - Pos2::new(480., 270.), self.angle) * self.scale
    }
    pub fn world(self, p: Pos2) -> Pos2 {
        Pos2::new(480., 270.) + rotate((p - self.center) / self.scale, -self.angle)
    }
}
#[derive(Clone, Copy, Debug)]
enum Operation {
    Move,
    Scale(usize),
    Rotate,
    Pivot,
}
struct TransformDrag {
    operation: Operation,
    start: Pos2,
    pivot: Pos2,
    bounds: SelectionBox,
    originals: Vec<((usize, usize), Mark)>,
    changed: bool,
}
enum Drag {
    Pan(Pos2),
    ViewRotate { start: f32, angle: f32 },
    Draw { raw: Vec<[f32; 2]> },
    Marquee { start: Pos2, end: Pos2, add: bool },
    Transform(TransformDrag),
}
pub struct Canvas {
    pub zoom: f32,
    pub pan: Vec2,
    pub angle: f32,
    pub selected: Vec<(usize, usize)>,
    pub pivot: Option<Pos2>,
    orientation: f32,
    frame: usize,
    drag: Option<Drag>,
}
impl Default for Canvas {
    fn default() -> Self {
        Self {
            zoom: 1.,
            pan: Vec2::ZERO,
            angle: 0.,
            selected: vec![],
            pivot: None,
            orientation: 0.,
            frame: 0,
            drag: None,
        }
    }
}
impl Canvas {
    fn view(&self, area: Rect) -> View {
        View {
            center: area.center() + self.pan,
            scale: fit_scale(area) * self.zoom,
            angle: self.angle,
        }
    }
    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.pivot = None;
        self.orientation = 0.;
        self.drag = None;
    }
    pub fn fit(&mut self) {
        self.zoom = 1.;
        self.pan = Vec2::ZERO;
        self.angle = 0.;
    }
    fn bounds(&self, e: &Editor) -> Option<SelectionBox> {
        let mut r = Rect::NOTHING;
        for &(l, m) in &self.selected {
            for p in vertices(&e.doc.layers[l].frames[e.frame][m]) {
                r.extend_with(Pos2::ZERO + rotate(p.to_vec2(), -self.orientation));
            }
        }
        if r.is_finite() {
            if r.width() < 1. {
                r = r.expand2(Vec2::new(0.5, 0.));
            }
            if r.height() < 1. {
                r = r.expand2(Vec2::new(0., 0.5));
            }
            Some(SelectionBox {
                rect: r,
                angle: self.orientation,
            })
        } else {
            None
        }
    }
}
pub fn vertices(m: &Mark) -> Vec<Pos2> {
    if m.rectangle && m.points.len() > 1 {
        let a = m.points[0];
        let b = *m.points.last().unwrap();
        vec![
            Pos2::new(a[0], a[1]),
            Pos2::new(b[0], a[1]),
            Pos2::new(b[0], b[1]),
            Pos2::new(a[0], b[1]),
            Pos2::new(a[0], a[1]),
        ]
    } else {
        m.points.iter().map(|p| Pos2::new(p[0], p[1])).collect()
    }
}
fn handles(r: Rect) -> [Pos2; 8] {
    [
        r.left_top(),
        r.center_top(),
        r.right_top(),
        r.right_center(),
        r.right_bottom(),
        r.center_bottom(),
        r.left_bottom(),
        r.left_center(),
    ]
}
fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0. {
        ((p - a).dot(ab) / ab.length_sq()).clamp(0., 1.)
    } else {
        0.
    };
    p.distance(a + ab * t)
}
fn hit(m: &Mark, p: Pos2, tolerance: f32) -> bool {
    let pts = vertices(m);
    let t = tolerance + m.width / 2.;
    pts.first().is_some_and(|q| q.distance(p) <= t)
        || pts
            .windows(2)
            .any(|w| distance_to_segment(p, w[0], w[1]) <= t)
}
pub fn transformed(m: &Mark, pivot: Pos2, scale: Vec2, angle: f32, translation: Vec2) -> Mark {
    let mut out = m.clone();
    out.rectangle = false;
    out.points = vertices(m)
        .into_iter()
        .map(|p| {
            let v = p - pivot;
            let p = pivot + rotate(Vec2::new(v.x * scale.x, v.y * scale.y), angle) + translation;
            [p.x.clamp(-1e7, 1e7), p.y.clamp(-1e7, 1e7)]
        })
        .collect();
    out
}
fn safe_ratio(a: f32, b: f32) -> f32 {
    if b.abs() < 0.0001 {
        1.
    } else {
        let s = a / b;
        if s.abs() < 0.01 {
            if s < 0. { -0.01 } else { 0.01 }
        } else {
            s.clamp(-1000., 1000.)
        }
    }
}
fn paint_mark(p: &egui::Painter, m: &Mark, v: View, ghost: bool, selected: bool) {
    let points: Vec<Pos2> = vertices(m).into_iter().map(|p| v.screen(p)).collect();
    let c = if ghost {
        Color32::from_rgba_unmultiplied(110, 145, 220, 65)
    } else {
        Color32::from_rgba_unmultiplied(m.color[0], m.color[1], m.color[2], m.color[3])
    };
    let width = m.width * v.scale;
    if points.len() == 1 {
        p.circle_filled(points[0], width / 2., c);
    } else if points.len() > 1 {
        p.add(egui::Shape::line(points.clone(), Stroke::new(width, c)));
        if points[0].distance(*points.last().unwrap()) > 0.1 {
            p.circle_filled(points[0], width / 2., c);
            p.circle_filled(*points.last().unwrap(), width / 2., c);
        }
    }
    // A stippled selection on the actual ink, instead of displaying sampled nodes.
    if selected {
        let mut carry = 0.;
        let spacing = 3.5;
        let mut count = 0;
        for pair in points.windows(2) {
            let delta = pair[1] - pair[0];
            let length = delta.length();
            if length < 0.001 {
                continue;
            }
            let direction = delta / length;
            let normal = Vec2::new(-direction.y, direction.x);
            let mut distance = carry;
            while distance < length && count < 20000 {
                let center = pair[0] + direction * distance;
                let rows = (width / 3.).floor().clamp(0., 12.) as i32;
                for row in -rows..=rows {
                    let offset = row as f32 * 1.5;
                    if offset.abs() < width / 2. - 0.4 || row == 0 {
                        p.circle_filled(
                            center + normal * offset,
                            0.65,
                            Color32::from_white_alpha(185),
                        );
                    }
                }
                distance += spacing;
                count += 1;
            }
            carry = distance - length;
        }
    }
}
impl Editor {
    pub fn canvas_ui(&mut self, ui: &mut egui::Ui) {
        let mut state = std::mem::take(&mut self.canvas);
        if state.frame != self.frame || self.playing {
            state.clear_selection();
            state.frame = self.frame;
        }
        state.selected.retain(|&(l, m)| {
            self.doc
                .layers
                .get(l)
                .is_some_and(|l| l.visible && m < l.frames[self.frame].len())
        });
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let area = response.rect;
        let fit = fit_scale(area);
        let mut view = state.view(area);
        let ctx = ui.ctx().clone();
        let wants_keyboard = ctx.egui_wants_keyboard_input();
        let (
            pointer,
            pressed,
            middle_pressed,
            primary_down,
            middle_down,
            released,
            space,
            shift,
            scroll,
            escape,
        ) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.button_pressed(PointerButton::Primary),
                i.pointer.button_pressed(PointerButton::Middle),
                i.pointer.button_down(PointerButton::Primary),
                i.pointer.button_down(PointerButton::Middle),
                i.pointer.button_released(PointerButton::Primary),
                i.key_down(egui::Key::Space) && !wants_keyboard,
                i.modifiers.shift,
                i.smooth_scroll_delta().y,
                i.key_pressed(egui::Key::Escape),
            )
        });
        let over = pointer.is_some_and(|p| area.contains(p));
        if over && scroll != 0. && state.drag.is_none() {
            let p = pointer.unwrap();
            let before = view.world(p);
            state.zoom = (state.zoom * (scroll * 0.0025).exp()).clamp(0.08, 24.);
            view.scale = fit * state.zoom;
            state.pan += p - view.screen(before);
            view.center = area.center() + state.pan;
        }
        if escape {
            if let Some(Drag::Transform(d)) = state.drag.take() {
                for ((l, m), original) in d.originals {
                    self.doc.layers[l].frames[self.frame][m] = original;
                }
                state.pivot = Some(d.pivot);
                state.orientation = d.bounds.angle;
            } else {
                state.clear_selection();
            }
            self.draft = None;
        }
        if !ctx.egui_wants_keyboard_input() && state.drag.is_none() {
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A))
                && matches!(self.tool, Tool::Select | Tool::Transform)
            {
                state.selected.clear();
                for (l, layer) in self
                    .doc
                    .layers
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.visible)
                {
                    for m in 0..layer.frames[self.frame].len() {
                        state.selected.push((l, m));
                    }
                }
                state.pivot = state.bounds(self).map(|b| b.center());
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) && !state.selected.is_empty() {
                self.checkpoint();
                let mut ids = state.selected.clone();
                ids.sort_unstable();
                for (l, m) in ids.into_iter().rev() {
                    self.doc.layers[l].frames[self.frame].remove(m);
                }
                state.clear_selection();
                self.recover();
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::D))
                && !state.selected.is_empty()
            {
                self.checkpoint();
                let mut selected = vec![];
                for &(l, m) in &state.selected {
                    let mark = transformed(
                        &self.doc.layers[l].frames[self.frame][m],
                        Pos2::ZERO,
                        Vec2::splat(1.),
                        0.,
                        Vec2::splat(15.),
                    );
                    let index = self.doc.layers[l].frames[self.frame].len();
                    self.doc.layers[l].frames[self.frame].push(mark);
                    selected.push((l, index));
                }
                state.selected = selected;
                state.pivot = state.pivot.map(|p| p + Vec2::splat(15.));
                self.recover();
            }
        }
        if over && state.drag.is_none() && (pressed || middle_pressed) {
            let screen = pointer.unwrap();
            let world = view.world(screen);
            if middle_pressed || (pressed && space) {
                state.drag = Some(Drag::Pan(screen));
            } else if self.tool == Tool::RotateView {
                let delta = screen - view.center;
                state.drag = Some(Drag::ViewRotate {
                    start: delta.y.atan2(delta.x),
                    angle: state.angle,
                });
            } else if !self.playing
                && matches!(self.tool, Tool::Brush | Tool::Rectangle)
                && self.doc.layers[self.layer].visible
                && Rect::from_min_max(Pos2::ZERO, Pos2::new(960., 540.)).contains(world)
            {
                self.draft = Some(Mark {
                    points: vec![[world.x, world.y]],
                    width: self.width,
                    color: self.color.to_array(),
                    rectangle: self.tool == Tool::Rectangle,
                });
                state.clear_selection();
                state.drag = Some(Drag::Draw {
                    raw: vec![[world.x, world.y]],
                });
            } else if !self.playing && matches!(self.tool, Tool::Select | Tool::Transform) {
                let mut op = None;
                if let Some(bounds) = state.bounds(self) {
                    let pivot = state.pivot.unwrap_or(bounds.center());
                    state.pivot = Some(pivot);
                    if self.tool == Tool::Transform {
                        if screen.distance(view.screen(pivot)) < 10. {
                            op = Some(Operation::Pivot);
                        }
                        let top = view.screen(bounds.center_top());
                        let rotation = top + rotate(Vec2::new(0., -30.), view.angle + bounds.angle);
                        if op.is_none() && screen.distance(rotation) < 10. {
                            op = Some(Operation::Rotate);
                        }
                        if op.is_none() {
                            for (h, p) in bounds.handles().into_iter().enumerate() {
                                if screen.distance(view.screen(p)) < 8. {
                                    op = Some(Operation::Scale(h));
                                    break;
                                }
                            }
                        }
                    }
                    if op.is_none() && !shift && bounds.contains(world) {
                        op = Some(Operation::Move);
                    }
                }
                if op.is_none() {
                    let mut found = None;
                    'layers: for (l, layer) in self
                        .doc
                        .layers
                        .iter()
                        .enumerate()
                        .rev()
                        .filter(|(_, l)| l.visible)
                    {
                        for (m, mark) in layer.frames[self.frame].iter().enumerate().rev() {
                            if hit(mark, world, 6. / view.scale) {
                                found = Some((l, m));
                                break 'layers;
                            }
                        }
                    }
                    if let Some(id) = found {
                        if shift && state.selected.contains(&id) {
                            state.selected.retain(|x| *x != id);
                            state.pivot = state.bounds(self).map(|b| b.center());
                        } else {
                            if !shift && !state.selected.contains(&id) {
                                state.selected.clear();
                            }
                            if !state.selected.contains(&id) {
                                state.selected.push(id);
                            }
                            self.layer = id.0;
                            state.pivot = state.bounds(self).map(|b| b.center());
                            op = Some(Operation::Move);
                        }
                    } else {
                        if !shift {
                            state.clear_selection();
                        }
                        state.drag = Some(Drag::Marquee {
                            start: world,
                            end: world,
                            add: shift,
                        });
                    }
                }
                if let (Some(operation), Some(bounds)) = (op, state.bounds(self)) {
                    let pivot = state.pivot.unwrap_or(bounds.center());
                    let originals = state
                        .selected
                        .iter()
                        .map(|&(l, m)| ((l, m), self.doc.layers[l].frames[self.frame][m].clone()))
                        .collect();
                    state.drag = Some(Drag::Transform(TransformDrag {
                        operation,
                        start: world,
                        pivot,
                        bounds,
                        originals,
                        changed: false,
                    }));
                }
            }
        }
        if let Some(screen) = pointer {
            let world = view.world(screen);
            match state.drag.as_mut() {
                Some(Drag::Pan(last)) => {
                    if primary_down || middle_down {
                        state.pan += screen - *last;
                        *last = screen;
                        view.center = area.center() + state.pan;
                    } else {
                        state.drag = None;
                    }
                }
                Some(Drag::ViewRotate { start, angle }) => {
                    if primary_down {
                        let d = screen - view.center;
                        state.angle = *angle + d.y.atan2(d.x) - *start;
                        if shift {
                            state.angle = (state.angle / (std::f32::consts::PI / 12.)).round()
                                * (std::f32::consts::PI / 12.);
                        }
                        view.angle = state.angle;
                    } else {
                        state.drag = None;
                    }
                }
                Some(Drag::Draw { raw }) => {
                    if let Some(m) = &mut self.draft {
                        let p = [world.x.clamp(0., 960.), world.y.clamp(0., 540.)];
                        if m.rectangle {
                            if m.points.len() == 1 {
                                m.points.push(p);
                            } else {
                                m.points[1] = p;
                            }
                        } else if raw.last().is_none_or(|last| {
                            (Pos2::new(last[0], last[1]) - Pos2::new(p[0], p[1])).length() > 0.4
                        }) {
                            raw.push(p);
                            m.points = crate::brush::smooth(raw, self.smoothing);
                        }
                    }
                    if released || !primary_down {
                        if let Some(m) = self.draft.take() {
                            self.checkpoint();
                            self.doc.layers[self.layer].frames[self.frame].push(m);
                            self.recover();
                        }
                        state.drag = None;
                    }
                }
                Some(Drag::Marquee { start, end, add }) => {
                    *end = world;
                    if released || !primary_down {
                        let rect = Rect::from_two_pos(*start, *end);
                        if !*add {
                            state.selected.clear();
                        }
                        for (l, layer) in self
                            .doc
                            .layers
                            .iter()
                            .enumerate()
                            .filter(|(_, l)| l.visible)
                        {
                            for (m, mark) in layer.frames[self.frame].iter().enumerate() {
                                if vertices(mark).iter().all(|p| rect.contains(*p))
                                    && !state.selected.contains(&(l, m))
                                {
                                    state.selected.push((l, m));
                                }
                            }
                        }
                        state.pivot = state.bounds(self).map(|b| b.center());
                        state.drag = None;
                    }
                }
                Some(Drag::Transform(d)) => {
                    if d.changed || world.distance(d.start) > 0.01 {
                        if !d.changed && !matches!(d.operation, Operation::Pivot) {
                            self.checkpoint();
                        }
                        d.changed = true;
                        match d.operation {
                            Operation::Pivot => {
                                state.pivot = Some(world);
                            }
                            operation => {
                                let mut scale = Vec2::splat(1.);
                                let mut angle = 0.;
                                let mut translation = Vec2::ZERO;
                                match operation {
                                    Operation::Move => {
                                        translation = world - d.start;
                                        state.pivot = Some(d.pivot + translation);
                                    }
                                    Operation::Rotate => {
                                        let a = d.start - d.pivot;
                                        let b = world - d.pivot;
                                        angle = b.y.atan2(b.x) - a.y.atan2(a.x);
                                        if shift {
                                            angle = (angle / (std::f32::consts::PI / 12.)).round()
                                                * (std::f32::consts::PI / 12.);
                                        }
                                    }
                                    Operation::Scale(h) => {
                                        let handle = d.bounds.handles()[h];
                                        let initial = rotate(handle - d.pivot, -d.bounds.angle);
                                        let target =
                                            initial + rotate(world - d.start, -d.bounds.angle);
                                        if h % 2 == 0 || h == 3 || h == 7 {
                                            scale.x = safe_ratio(target.x, initial.x);
                                        }
                                        if h % 2 == 0 || h == 1 || h == 5 {
                                            scale.y = safe_ratio(target.y, initial.y);
                                        }
                                        if shift {
                                            let factor =
                                                if (scale.x - 1.).abs() > (scale.y - 1.).abs() {
                                                    scale.x
                                                } else {
                                                    scale.y
                                                };
                                            scale = Vec2::splat(factor);
                                        }
                                    }
                                    _ => {}
                                }
                                state.orientation = d.bounds.angle + angle;
                                for ((l, m), original) in &d.originals {
                                    self.doc.layers[*l].frames[self.frame][*m] =
                                        transformed_in_basis(
                                            original,
                                            d.pivot,
                                            scale,
                                            angle,
                                            translation,
                                            d.bounds.angle,
                                        );
                                }
                            }
                        }
                    }
                    if released || !primary_down {
                        if d.changed {
                            self.recover();
                        }
                        state.drag = None;
                    }
                }
                None => {}
            }
        }
        if over {
            ctx.set_cursor_icon(if matches!(state.drag, Some(Drag::Pan(_))) {
                CursorIcon::Grabbing
            } else if space {
                CursorIcon::Grab
            } else if matches!(self.tool, Tool::Select | Tool::Transform) {
                CursorIcon::Default
            } else {
                CursorIcon::Crosshair
            });
        }
        let corners = [
            Pos2::ZERO,
            Pos2::new(960., 0.),
            Pos2::new(960., 540.),
            Pos2::new(0., 540.),
        ];
        let screen: Vec<Pos2> = corners.into_iter().map(|p| view.screen(p)).collect();
        painter.add(egui::Shape::convex_polygon(
            screen.clone(),
            Color32::WHITE,
            Stroke::new(1., Color32::from_gray(90)),
        ));
        // Viewport clipping allows transformed artwork outside the stage to remain editable.
        for layer in self.doc.layers.iter().filter(|l| l.visible) {
            if self.onion && !self.playing && self.frame > 0 {
                for m in &layer.frames[self.frame - 1] {
                    paint_mark(&painter, m, view, true, false);
                }
            }
        }
        for (l, layer) in self
            .doc
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.visible)
        {
            for (m, mark) in layer.frames[self.frame].iter().enumerate() {
                paint_mark(
                    &painter,
                    mark,
                    view,
                    false,
                    state.selected.contains(&(l, m)),
                );
            }
        }
        if let Some(mark) = &self.draft {
            paint_mark(&painter, mark, view, false, false);
        }
        if let Some(bounds) = state.bounds(self) {
            if matches!(self.tool, Tool::Select | Tool::Transform) {
                let mut points: Vec<Pos2> = [
                    bounds.left_top(),
                    bounds.right_top(),
                    bounds.right_bottom(),
                    bounds.left_bottom(),
                    bounds.left_top(),
                ]
                .into_iter()
                .map(|p| view.screen(p))
                .collect();
                painter.add(egui::Shape::line(
                    std::mem::take(&mut points),
                    Stroke::new(1., ACCENT),
                ));
                if self.tool == Tool::Transform {
                    for p in bounds.handles() {
                        painter.rect_filled(
                            Rect::from_center_size(view.screen(p), Vec2::splat(7.)),
                            0.,
                            Color32::from_rgb(32, 36, 43),
                        );
                        painter.rect_stroke(
                            Rect::from_center_size(view.screen(p), Vec2::splat(7.)),
                            0.,
                            Stroke::new(1., ACCENT),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let top = view.screen(bounds.center_top());
                    let rot = top + rotate(Vec2::new(0., -30.), view.angle + bounds.angle);
                    painter.line_segment([top, rot], Stroke::new(1., ACCENT));
                    painter.circle_filled(rot, 5., Color32::from_rgb(32, 36, 43));
                    painter.circle_stroke(rot, 5., Stroke::new(1.5, ACCENT));
                    let pivot = view.screen(state.pivot.unwrap_or(bounds.center()));
                    painter.circle_filled(pivot, 5., Color32::from_gray(28));
                    painter.circle_stroke(pivot, 5., Stroke::new(1.5, ACCENT));
                    painter.line_segment(
                        [pivot - Vec2::new(8., 0.), pivot + Vec2::new(8., 0.)],
                        Stroke::new(1., ACCENT),
                    );
                    painter.line_segment(
                        [pivot - Vec2::new(0., 8.), pivot + Vec2::new(0., 8.)],
                        Stroke::new(1., ACCENT),
                    );
                }
            }
        }
        if let Some(Drag::Marquee { start, end, .. }) = &state.drag {
            let r = Rect::from_two_pos(*start, *end);
            let mut pts: Vec<_> = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ]
            .into_iter()
            .map(|p| view.screen(p))
            .collect();
            painter.add(egui::Shape::line(
                std::mem::take(&mut pts),
                Stroke::new(1., ACCENT),
            ));
        }
        self.canvas = state;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn mark() -> Mark {
        Mark {
            points: vec![[10., 20.], [50., 60.]],
            color: [0, 0, 0, 255],
            width: 4.,
            rectangle: false,
        }
    }
    #[test]
    fn view_inverse_with_rotation_pan_zoom() {
        let v = View {
            center: Pos2::new(143., -22.),
            scale: 2.7,
            angle: 1.3,
        };
        let p = Pos2::new(-32., 750.);
        assert!(v.world(v.screen(p)).distance(p) < 0.001);
    }
    #[test]
    fn pivot_stays_fixed_under_scale_and_rotation() {
        let mut m = mark();
        m.points[0] = [10., 20.];
        let out = transformed(&m, Pos2::new(10., 20.), Vec2::new(2., 3.), 1.2, Vec2::ZERO);
        assert_eq!(out.points[0], [10., 20.]);
    }
    #[test]
    fn rotated_rectangle_preserves_four_edges() {
        let mut m = mark();
        m.rectangle = true;
        let out = transformed(
            &m,
            Pos2::ZERO,
            Vec2::splat(1.),
            std::f32::consts::FRAC_PI_2,
            Vec2::ZERO,
        );
        assert!(!out.rectangle);
        assert_eq!(out.points.len(), 5);
        assert!((out.points[1][0] + 20.).abs() < 0.001);
        assert!((out.points[1][1] - 50.).abs() < 0.001);
    }
    #[test]
    fn stroke_hit_tests_geometry_not_bounds() {
        assert!(hit(&mark(), Pos2::new(30., 40.), 2.));
        assert!(!hit(&mark(), Pos2::new(12., 58.), 2.));
    }
}

#[cfg(test)]
mod gesture_tests {
    use super::*;
    fn event(pos: Pos2, button: PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn tick(ctx: &egui::Context, e: &mut Editor, events: Vec<egui::Event>) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| {
                e.canvas_ui(ui);
            },
        );
        output.textures_delta.clear();
    }
    #[test]
    fn middle_and_space_drag_pan_without_drawing() {
        for middle in [true, false] {
            let ctx = egui::Context::default();
            let cc = eframe::CreationContext::_new_kittest(ctx.clone());
            let mut e = Editor::new(&cc);
            e.doc = crate::Document::default();
            let p = Pos2::new(500., 350.);
            let end = p + Vec2::new(70., 45.);
            let button = if middle {
                PointerButton::Middle
            } else {
                PointerButton::Primary
            };
            tick(&ctx, &mut e, vec![]);
            let mut events = vec![egui::Event::PointerMoved(p)];
            if !middle {
                events.push(egui::Event::Key {
                    key: egui::Key::Space,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            events.push(event(p, button, true));
            tick(&ctx, &mut e, events);
            tick(&ctx, &mut e, vec![egui::Event::PointerMoved(end)]);
            tick(&ctx, &mut e, vec![event(end, button, false)]);
            assert!(
                (e.canvas.pan - Vec2::new(70., 45.)).length() < 0.01,
                "middle={middle}, pan={:?}",
                e.canvas.pan
            );
            assert!(e.doc.layers[0].frames[0].is_empty());
            assert!(e.draft.is_none());
        }
    }
    #[test]
    fn transformed_offstage_art_survives_save_and_export() {
        let mut doc = crate::Document::default();
        let mark = Mark {
            points: vec![[0., 0.], [40., 30.]],
            color: [20, 30, 40, 255],
            width: 3.,
            rectangle: true,
        };
        doc.layers[0].frames[0].push(transformed(
            &mark,
            Pos2::ZERO,
            Vec2::new(2., 1.),
            0.5,
            Vec2::new(-200., 800.),
        ));
        let restored: crate::Document =
            serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert!(restored.validate().is_ok());
        assert!(restored.svg(0).contains("polyline"));
        assert_eq!(restored.layers[0].frames[0][0].points.len(), 5);
    }
}

fn fit_scale(area: Rect) -> f32 {
    ((area.width() - 60.) / 960.)
        .min((area.height() - 60.) / 540.)
        .max(0.01)
}
#[derive(Clone, Copy)]
struct SelectionBox {
    rect: Rect,
    angle: f32,
}
impl SelectionBox {
    fn world(self, p: Pos2) -> Pos2 {
        Pos2::ZERO + rotate(p.to_vec2(), self.angle)
    }
    fn center(self) -> Pos2 {
        self.world(self.rect.center())
    }
    fn center_top(self) -> Pos2 {
        self.world(self.rect.center_top())
    }
    fn left_top(self) -> Pos2 {
        self.world(self.rect.left_top())
    }
    fn right_top(self) -> Pos2 {
        self.world(self.rect.right_top())
    }
    fn left_bottom(self) -> Pos2 {
        self.world(self.rect.left_bottom())
    }
    fn right_bottom(self) -> Pos2 {
        self.world(self.rect.right_bottom())
    }
    fn handles(self) -> [Pos2; 8] {
        handles(self.rect).map(|p| self.world(p))
    }
    fn contains(self, p: Pos2) -> bool {
        self.rect
            .contains(Pos2::ZERO + rotate(p.to_vec2(), -self.angle))
    }
}
fn transformed_in_basis(
    m: &Mark,
    pivot: Pos2,
    scale: Vec2,
    angle: f32,
    translation: Vec2,
    basis: f32,
) -> Mark {
    let mut out = m.clone();
    out.rectangle = false;
    out.points = vertices(m)
        .into_iter()
        .map(|p| {
            let local = rotate(p - pivot, -basis);
            let v = rotate(
                Vec2::new(local.x * scale.x, local.y * scale.y),
                basis + angle,
            );
            let p = pivot + v + translation;
            [p.x.clamp(-1e7, 1e7), p.y.clamp(-1e7, 1e7)]
        })
        .collect();
    out
}

#[cfg(test)]
mod rotation_regressions {
    use super::*;
    #[test]
    fn view_rotation_preserves_scale_and_distances() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.));
        let mut c = Canvas::default();
        c.zoom = 2.;
        c.pan = Vec2::new(12., -40.);
        let initial = c.view(area);
        let a = Pos2::new(20., 80.);
        let b = Pos2::new(700., 400.);
        for degrees in (0..360).step_by(7) {
            c.angle = (degrees as f32).to_radians();
            let v = c.view(area);
            assert_eq!(v.scale, initial.scale);
            assert!(
                (v.screen(a).distance(v.screen(b)) - initial.screen(a).distance(initial.screen(b)))
                    .abs()
                    < 0.001
            );
        }
    }
    #[test]
    fn fit_resets_position_zoom_and_rotation() {
        let mut c = Canvas::default();
        c.zoom = 9.;
        c.pan = Vec2::new(-230., 960.);
        c.angle = 2.3;
        c.fit();
        assert_eq!(c.zoom, 1.);
        assert_eq!(c.pan, Vec2::ZERO);
        assert_eq!(c.angle, 0.);
        let area = Rect::from_min_size(Pos2::new(40., 30.), Vec2::new(1000., 700.));
        assert_eq!(c.view(area).screen(Pos2::new(480., 270.)), area.center());
    }
    #[test]
    fn rotation_and_oriented_frame_are_rigid() {
        let mark = Mark {
            points: vec![
                [20., 30.],
                [180., 30.],
                [180., 100.],
                [20., 100.],
                [20., 30.],
            ],
            color: [0, 0, 0, 255],
            width: 6.,
            rectangle: false,
        };
        let pivot = Pos2::new(-80., 190.);
        for degrees in (0..360).step_by(11) {
            let angle = (degrees as f32).to_radians();
            let result =
                transformed_in_basis(&mark, pivot, Vec2::splat(1.), angle, Vec2::ZERO, 0.4);
            for (i, a) in result.points.iter().enumerate() {
                let a = Pos2::new(a[0], a[1]);
                let origin = Pos2::new(mark.points[i][0], mark.points[i][1]);
                assert!((a.distance(pivot) - origin.distance(pivot)).abs() < 0.001);
            }
            assert_eq!(result.width, mark.width);
            let mut rect = Rect::NOTHING;
            for p in &result.points {
                rect.extend_with(Pos2::ZERO + rotate(Vec2::new(p[0], p[1]), -angle));
            }
            assert!((rect.width() - 160.).abs() < 0.001);
            assert!((rect.height() - 70.).abs() < 0.001);
        }
    }
}
