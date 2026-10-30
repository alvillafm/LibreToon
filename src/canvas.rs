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
    VectorBrush,
    FillBrush,
    ShadeBrush,
    LightBrush,
    Ruler,
    Symmetry,
    Perspective,
    Camera,
    Mirror,
    Pen,
    Node,
    Bucket,
    Width,
    Rectangle,
    Select,
    Transform,
    RotateView,
    Lasso,
    PolygonSelect,
    Wand,
    Object,
    Eraser,
    Eyedropper,
    Line,
    Ellipse,
    Polygon,
    Pan,
    Zoom,
}
impl Tool {
    pub fn name(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::Brush => "Pincel Estándar",
            Self::VectorBrush => "Pincel Vectorial",
            Self::FillBrush => "Pincel de relleno",
            Self::ShadeBrush => "Pincel de sombreado",
            Self::LightBrush => "Pincel de iluminación",
            Self::Ruler => "Regla",
            Self::Symmetry => "Simetría",
            Self::Perspective => "Perspectiva",
            Self::Camera => "Cámara",
            Self::Mirror => "Espejo",
            Self::Pen => "Pluma",
            Self::Node => "Editar nodos",
            Self::Bucket => "Bote de pintura",
            Self::Width => "Anchura del contorno",
            Self::Rectangle => "Rectángulo",
            Self::Select => "Selección",
            Self::Transform => "Transformación libre",
            Self::RotateView => "Rotar vista",
            Self::Lasso => "Lazo",
            Self::PolygonSelect => "Selección poligonal",
            Self::Wand => "Varita mágica",
            Self::Object => "Selección de objeto",
            Self::Eraser => "Goma de borrar",
            Self::Eyedropper => "Cuentagotas",
            Self::Line => "Línea",
            Self::Ellipse => "Círculo / elipse",
            Self::Polygon => "Polígono / estrella",
            Self::Pan => "Mano",
            Self::Zoom => "Lupa",
        })
    }
    pub fn icon(self) -> Icon {
        match self {
            Self::Brush => Icon::Brush,
            Self::VectorBrush => Icon::VectorBrush,
            Self::FillBrush => Icon::FillBrush,
            Self::ShadeBrush => Icon::ShadeBrush,
            Self::LightBrush => Icon::LightBrush,
            Self::Ruler => Icon::Ruler,
            Self::Symmetry => Icon::Symmetry,
            Self::Perspective => Icon::Perspective,
            Self::Camera => Icon::Camera,
            Self::Mirror => Icon::Mirror,
            Self::Pen => Icon::Pen,
            Self::Node => Icon::Node,
            Self::Bucket => Icon::Bucket,
            Self::Width => Icon::Width,
            Self::Rectangle => Icon::Rectangle,
            Self::Select => Icon::Select,
            Self::Transform => Icon::Transform,
            Self::RotateView => Icon::Rotate,
            Self::Lasso => Icon::Lasso,
            Self::PolygonSelect => Icon::PolygonSelect,
            Self::Wand => Icon::Wand,
            Self::Object => Icon::Object,
            Self::Eraser => Icon::Eraser,
            Self::Eyedropper => Icon::Eyedropper,
            Self::Line => Icon::Line,
            Self::Ellipse => Icon::Ellipse,
            Self::Polygon => Icon::Polygon,
            Self::Pan => Icon::Pan,
            Self::Zoom => Icon::Zoom,
        }
    }
}
#[derive(Clone, Copy)]
pub struct View {
    pub center: Pos2,
    pub scale: f32,
    pub angle: f32,
    pub stage_center: Pos2,
}
pub fn rotate(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y)
}
impl View {
    pub fn screen(self, p: Pos2) -> Pos2 {
        self.center + rotate(p - self.stage_center, self.angle) * self.scale
    }
    pub fn world(self, p: Pos2) -> Pos2 {
        self.stage_center + rotate((p - self.center) / self.scale, -self.angle)
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
    last_input: Option<(Pos2, bool)>,
}
enum Drag {
    Area {
        points: Vec<Pos2>,
        add: bool,
    },
    Erase {
        last: Pos2,
        changed: bool,
    },
    Pan(Pos2),
    ViewRotate {
        start: f32,
        angle: f32,
    },
    Draw {
        smoother: crate::brush::StrokeSmoother,
        tool: Tool,
    },
    Marquee {
        start: Pos2,
        end: Pos2,
        add: bool,
    },
    Transform(TransformDrag),
    PenNode {
        index: usize,
        start: Pos2,
    },
    Node(NodeDrag),
    Width(WidthDrag),
}
pub struct Canvas {
    pub zoom: f32,
    pub actual_scale: f32,
    pub pan: Vec2,
    pub angle: f32,
    pub selected: Vec<(usize, usize)>,
    pub pivot: Option<Pos2>,
    orientation: f32,
    frame: usize,
    drag: Option<Drag>,
    pen: Option<PenDraft>,
    node_selection: Option<(usize, usize, usize)>,
    cache: crate::paint::RenderCache,
    stage_size: Vec2,
    polygon_selection: Vec<Pos2>,
    polygon_add: bool,
}
impl Default for Canvas {
    fn default() -> Self {
        Self {
            zoom: 1.,
            actual_scale: 1.,
            pan: Vec2::ZERO,
            angle: 0.,
            selected: vec![],
            pivot: None,
            orientation: 0.,
            frame: 0,
            drag: None,
            pen: None,
            node_selection: None,
            cache: Default::default(),
            stage_size: Vec2::new(960., 540.),
            polygon_selection: vec![],
            polygon_add: false,
        }
    }
}
impl Canvas {
    fn view(&self, area: Rect) -> View {
        View {
            center: area.center() + self.pan,
            scale: fit_scale(area, self.stage_size) * self.zoom,
            angle: self.angle,
            stage_center: Pos2::ZERO + self.stage_size * 0.5,
        }
    }
    pub fn clear_selection(&mut self) {
        self.polygon_selection.clear();
        self.selected.clear();
        self.node_selection = None;
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
        let mut evaluated = std::collections::HashMap::new();
        for &(l, m) in &self.selected {
            let marks = evaluated
                .entry(l)
                .or_insert_with(|| crate::animation::marks(&e.doc.layers[l], e.frame));
            for p in vertices(&marks[m]) {
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
    crate::vector::flatten(m, 0.1)
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
pub(crate) fn stroke_hit(m: &Mark, p: Pos2, tolerance: f32) -> bool {
    if !m.stroke {
        return false;
    }
    let points = vertices(m);
    let (lengths, total) = crate::width::lengths(&points);
    points.first().is_some_and(|q| {
        q.distance(p) <= tolerance + m.width * crate::width::sample(&m.width_profile, 0.) * 0.5
    }) || points.windows(2).enumerate().any(|(i, w)| {
        let d = w[1] - w[0];
        let t = ((p - w[0]).dot(d) / d.length_sq().max(1e-12)).clamp(0., 1.);
        let fraction = (lengths[i] + d.length() * t) / total.max(1e-6);
        p.distance(w[0] + d * t)
            <= tolerance + m.width * crate::width::sample(&m.width_profile, fraction) * 0.5
    }) || m.holes.iter().any(|r| {
        r.windows(2).any(|w| {
            distance_to_segment(p, crate::vector::pos(w[0]), crate::vector::pos(w[1]))
                <= tolerance + m.width * 0.5
        })
    })
}
pub(crate) fn axis_distance(m: &Mark, p: Pos2) -> f32 {
    vertices(m)
        .windows(2)
        .map(|w| distance_to_segment(p, w[0], w[1]))
        .fold(f32::INFINITY, f32::min)
}
fn hit(m: &Mark, p: Pos2, tolerance: f32) -> bool {
    stroke_hit(m, p, tolerance) || crate::vector::contains_fill(m, p)
}
pub fn transformed(m: &Mark, pivot: Pos2, scale: Vec2, angle: f32, translation: Vec2) -> Mark {
    crate::vector::transform(m, |p| {
        let v = p - pivot;
        let p = pivot + rotate(Vec2::new(v.x * scale.x, v.y * scale.y), angle) + translation;
        Pos2::new(p.x.clamp(-1e7, 1e7), p.y.clamp(-1e7, 1e7))
    })
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
fn paint_mark(
    p: &egui::Painter,
    m: &Mark,
    v: View,
    ghost: bool,
    selected: bool,
    cache: Option<&mut crate::paint::RenderCache>,
) -> bool {
    crate::paint::draw(p, m, v, ghost, selected, cache)
}
impl Editor {
    pub fn canvas_ui(&mut self, ui: &mut egui::Ui) {
        let mut state = std::mem::take(&mut self.canvas);
        state.stage_size = Vec2::new(self.doc.width as f32, self.doc.height as f32);
        if state.pen.as_ref().is_some_and(|p| {
            self.tool != Tool::Pen || p.frame != self.frame || p.layer != self.layer || self.playing
        }) {
            let pen = state.pen.take().unwrap();
            state.drag = None;
            self.commit_pen(pen);
        }
        if self.tool != Tool::PolygonSelect {
            state.polygon_selection.clear();
        }
        state.cache.begin_frame();
        if state.frame != self.frame || self.playing {
            self.draft = None;
            state.clear_selection();
            state.frame = self.frame;
        }
        state.selected.retain(|&(l, m)| {
            self.doc
                .layers
                .get(l)
                .is_some_and(|l| l.visible && !l.options.locked && m < l.frames[self.frame].len())
        });
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let ruler_area = response.rect;
        let area = if self.studio.rulers {
            Rect::from_min_max(ruler_area.min + Vec2::splat(22.), ruler_area.max)
        } else {
            ruler_area
        };
        let ruler_painter = painter.clone();
        let painter = painter.with_clip_rect(area);
        let fit = fit_scale(area, state.stage_size);
        let stage_view = state.view(area);
        let mut view = self.camera_view(stage_view);
        let ctx = ui.ctx().clone();
        let wants_keyboard = ctx.egui_wants_keyboard_input()
            || self.animation.scenes_open
            || self.animation.picker.is_some()
            || self.studio.preferences_open
            || self.studio.export.is_some();
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
        let over = response.contains_pointer();
        let press_position = ctx.input(|i| {
            i.events.iter().rev().find_map(|event| match event {
                egui::Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    ..
                } => Some(*pos),
                _ => None,
            })
        });
        let visual_handled = self.visual_canvas_input(ui, area, view, &state.selected);
        let guide_handled = visual_handled
            | self.rulers_ui(ui, &ruler_painter, ruler_area, view)
            | self.advanced_canvas_input(ui, area, view);
        let mut convert = false;
        response.context_menu(|ui| {
            if crate::design::button(
                ui,
                Icon::Symbol,
                "Convertir a símbolo · F8",
                "Convertir a símbolo",
                false,
                !state.selected.is_empty(),
            )
            .clicked()
            {
                convert = true;
                ui.close();
            }
        });
        if convert {
            self.canvas = state;
            self.convert_symbol();
            return;
        }
        if response.double_clicked()
            && matches!(self.tool, Tool::Select | Tool::Object | Tool::Transform)
        {
            if let Some(screen) = pointer {
                let world = view.world(screen);
                let symbol = self
                    .doc
                    .layers
                    .iter()
                    .rev()
                    .filter(|l| l.visible && !l.options.locked)
                    .flat_map(|l| l.frames[self.frame].iter().rev())
                    .find_map(|m| {
                        m.style
                            .instance
                            .as_ref()
                            .filter(|_| crate::vector::contains_fill(m, world))
                            .map(|i| i.symbol)
                    });
                if let Some(id) = symbol {
                    self.canvas = state;
                    self.enter_symbol(id);
                    return;
                }
            }
        }
        let alt = ctx.input(|i| i.modifiers.alt);
        let command = ctx.input(|i| i.modifiers.command);
        let mut handled_delete = false;
        if !wants_keyboard && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(pen) = state.pen.take() {
                state.drag = None;
                self.commit_pen(pen);
            }
        }
        if self.tool == Tool::Node
            && !wants_keyboard
            && ctx.input(|i| i.key_pressed(egui::Key::Delete))
        {
            if let Some((l, m, index)) = state.node_selection.take() {
                let mut mark = editable_nodes(&self.doc.layers[l].frames[self.frame][m]);
                self.checkpoint();
                handled_delete = true;
                state.drag = None;
                if mark.nodes.len() <= 2 {
                    self.doc.layers[l].frames[self.frame].remove(m);
                    state.clear_selection();
                } else {
                    mark.nodes.remove(index);
                    self.doc.layers[l].frames[self.frame][m] = mark;
                }
                self.recover();
            }
        }
        if over && scroll != 0. && state.drag.is_none() {
            let p = pointer.unwrap();
            let before = view.world(p);
            state.zoom = (state.zoom * (scroll * 0.0025).exp()).clamp(0.08, 24.);
            view.scale = fit * state.zoom;
            state.pan += p - view.screen(before);
            view.center = area.center() + state.pan;
        }
        if escape {
            self.cancel_guide_drag();
            let canceled = state.drag.take();
            if let Some(Drag::Width(d)) = &canceled {
                self.doc.layers[d.id.0].frames[self.frame][d.id.1] = d.original.clone();
            }
            if let Some(Drag::Node(d)) = &canceled {
                self.doc.layers[d.id.0].frames[self.frame][d.id.1] = d.original.clone();
            }
            if let Some(Drag::Transform(d)) = canceled {
                for ((l, m), original) in d.originals {
                    self.doc.layers[l].frames[self.frame][m] = original;
                }
                state.pivot = Some(d.pivot);
                state.orientation = d.bounds.angle;
            } else {
                state.clear_selection();
            }
            self.draft = None;
            state.pen = None;
        }
        if !ctx.egui_wants_keyboard_input() && state.drag.is_none() {
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A))
                && matches!(self.tool, Tool::Select | Tool::Transform | Tool::Object)
            {
                state.selected.clear();
                for (l, layer) in self
                    .doc
                    .layers
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.visible && !l.options.locked)
                {
                    for m in 0..layer.frames[self.frame].len() {
                        state.selected.push((l, m));
                    }
                }
                state.pivot = state.bounds(self).map(|b| b.center());
            }
            if !handled_delete
                && ctx.input(|i| i.key_pressed(egui::Key::Delete))
                && !state.selected.is_empty()
            {
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
        if !guide_handled
            && (!matches!(
                self.tool,
                Tool::Ruler | Tool::Symmetry | Tool::Mirror | Tool::Perspective | Tool::Camera
            ) || space
                || middle_pressed)
            && (press_position
                .or(pointer)
                .is_some_and(|p| area.contains(p) && ctx.layer_id_at(p) == Some(ui.layer_id())))
            && state.drag.is_none()
            && (pressed || middle_pressed)
        {
            let screen = press_position.or(pointer).unwrap();
            let world = view.world(screen);
            if middle_pressed || (pressed && (space || self.tool == Tool::Pan)) {
                state.drag = Some(Drag::Pan(screen));
            } else if self.tool == Tool::Zoom {
                let old = view.world(screen);
                state.zoom =
                    (state.zoom * if self.zoom_out ^ alt { 0.8 } else { 1.25 }).clamp(0.05, 32.);
                view = self.camera_view(state.view(area));
                state.pan += screen - view.screen(old);
                view = self.camera_view(state.view(area));
            } else if !self.playing && self.tool == Tool::Eyedropper {
                self.sample_apply_at(&state.selected, world, 3. / view.scale);
            } else if !self.playing && self.tool == Tool::Wand {
                self.wand_select(&mut state, world, shift);
            } else if !self.playing && self.tool == Tool::Lasso {
                state.drag = Some(Drag::Area {
                    points: vec![world],
                    add: shift,
                });
            } else if !self.playing && self.tool == Tool::PolygonSelect {
                if state.polygon_selection.is_empty() {
                    state.polygon_add = shift;
                }
                if state.polygon_selection.len() >= 3
                    && view.screen(state.polygon_selection[0]).distance(screen) < 8.
                {
                    let points = std::mem::take(&mut state.polygon_selection);
                    let add = state.polygon_add;
                    self.cut_area_selection(&mut state, &points, add);
                } else {
                    state.polygon_selection.push(world);
                }
            } else if !self.playing
                && self.tool == Tool::Eraser
                && self.doc.layers[self.layer].visible
                && !self.doc.layers[self.layer].options.locked
            {
                state.clear_selection();
                state.drag = Some(Drag::Erase {
                    last: world,
                    changed: false,
                });
            } else if !self.playing
                && self.tool == Tool::Select
                && selected_node_hit(self, &state, world, view)
            {
                self.start_node_drag(&mut state, world, view);
            } else if self.tool == Tool::RotateView {
                let delta = screen - view.center;
                state.drag = Some(Drag::ViewRotate {
                    start: delta.y.atan2(delta.x),
                    angle: state.angle,
                });
            } else if !self.playing
                && self.tool == Tool::Pen
                && self.doc.layers[self.layer].visible
                && !self.doc.layers[self.layer].options.locked
            {
                self.begin_pen(&mut state, world, view);
            } else if !self.playing
                && self.tool == Tool::Bucket
                && self.doc.layers[self.layer].visible
                && !self.doc.layers[self.layer].options.locked
            {
                self.bucket_at(&mut state, world);
            } else if !self.playing && self.tool == Tool::Width {
                self.start_width_drag(&mut state, world, view);
            } else if !self.playing && self.tool == Tool::Node {
                self.start_node_drag(&mut state, world, view);
            } else if !self.playing
                && matches!(
                    self.tool,
                    Tool::Brush
                        | Tool::FillBrush
                        | Tool::ShadeBrush
                        | Tool::LightBrush
                        | Tool::VectorBrush
                        | Tool::Rectangle
                        | Tool::Line
                        | Tool::Ellipse
                        | Tool::Polygon
                )
                && self.doc.layers[self.layer].visible
                && !self.doc.layers[self.layer].options.locked
                && match self.tool {
                    Tool::Brush | Tool::FillBrush | Tool::ShadeBrush | Tool::LightBrush => true,
                    Tool::VectorBrush | Tool::Line => self.stroke_enabled,
                    _ => self.fill_enabled || self.stroke_enabled,
                }
            {
                self.draft = Some(Mark {
                    points: vec![[world.x, world.y]].into(),
                    width: self.width,
                    width_profile: if matches!(
                        self.tool,
                        Tool::VectorBrush | Tool::Line | Tool::Ellipse | Tool::Polygon
                    ) {
                        self.width_profile.clone()
                    } else {
                        Default::default()
                    },
                    color: if matches!(
                        self.tool,
                        Tool::Brush | Tool::FillBrush | Tool::ShadeBrush | Tool::LightBrush
                    ) {
                        self.fill_color.to_array()
                    } else {
                        self.color.to_array()
                    },
                    rectangle: self.tool == Tool::Rectangle,
                    closed: self.tool == Tool::Rectangle,
                    stroke: if matches!(self.tool, Tool::Rectangle | Tool::Ellipse | Tool::Polygon)
                    {
                        self.stroke_enabled
                    } else {
                        true
                    },
                    fill: if matches!(self.tool, Tool::Rectangle | Tool::Ellipse | Tool::Polygon)
                        && self.fill_enabled
                    {
                        Some(self.fill_color.to_array())
                    } else {
                        None
                    },
                    ..Mark::default()
                });
                state.clear_selection();
                state.drag = Some(Drag::Draw {
                    smoother: crate::brush::StrokeSmoother::new([world.x, world.y], self.smoothing),
                    tool: self.tool,
                });
            } else if !self.playing
                && matches!(self.tool, Tool::Select | Tool::Transform | Tool::Object)
            {
                let force_marquee = command && self.tool == Tool::Select;
                let mut op = None;
                if let Some(bounds) = state.bounds(self).filter(|_| !force_marquee) {
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
                    if op.is_none()
                        && !shift
                        && bounds.contains(world)
                        && (self.tool == Tool::Transform
                            || state.selected.iter().any(|&(l, m)| {
                                hit(
                                    &crate::animation::marks(&self.doc.layers[l], self.frame)[m],
                                    world,
                                    4. / view.scale,
                                )
                            }))
                    {
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
                        .filter(|(_, l)| l.visible && !l.options.locked)
                    {
                        for (m, mark) in crate::animation::marks(layer, self.frame)
                            .iter()
                            .enumerate()
                            .rev()
                        {
                            if !force_marquee && hit(mark, world, 6. / view.scale) {
                                found = Some((l, m));
                                break 'layers;
                            }
                        }
                    }
                    if let Some(mut id) = found {
                        if self.tool == Tool::Object {
                            self.select_connected_object(&mut state, id, shift);
                        }
                        if self.tool == Tool::Select && !state.selected.contains(&id) {
                            let wants_stroke = stroke_hit(
                                &self.doc.layers[id.0].frames[self.frame][id.1],
                                world,
                                4. / view.scale,
                            );
                            id = self.separate_component(&mut state, id, wants_stroke);
                            if wants_stroke {
                                id = self.split_clicked_crossing(&mut state, id, world);
                            } else {
                                id = self.split_fill_face(&mut state, id, world);
                            }
                        }
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
                        .map(|&(l, m)| {
                            (
                                (l, m),
                                crate::animation::marks(&self.doc.layers[l], self.frame)[m].clone(),
                            )
                        })
                        .collect();
                    state.drag = Some(Drag::Transform(TransformDrag {
                        operation,
                        start: world,
                        pivot,
                        bounds,
                        originals,
                        changed: false,
                        last_input: None,
                    }));
                }
            }
        }
        if let Some(screen) = pointer {
            let world = view.world(screen);
            match state.drag.as_mut() {
                Some(Drag::Erase { last, changed }) => {
                    let samples = ctx.input(|i| {
                        i.events
                            .iter()
                            .filter_map(|e| match e {
                                egui::Event::PointerMoved(p) => Some(view.world(*p)),
                                egui::Event::PointerButton {
                                    pos,
                                    button: PointerButton::Primary,
                                    pressed: false,
                                    ..
                                } => Some(view.world(*pos)),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                    });
                    for point in samples.into_iter().chain(std::iter::once(world)) {
                        let before = (!*changed).then(|| self.doc.clone());
                        let did_change = self.erase_segment(*last, point);
                        if did_change && !*changed {
                            self.undo.push(before.unwrap());
                            if self.undo.len() > 60 {
                                self.undo.remove(0);
                            }
                            self.redo.clear();
                        }
                        *changed |= did_change;
                        *last = point;
                    }
                    if released || !primary_down {
                        if *changed {
                            self.recover();
                        }
                        state.drag = None;
                    }
                }
                Some(Drag::Area { points, add }) => {
                    let samples = ctx.input(|i| {
                        i.events
                            .iter()
                            .filter_map(|e| {
                                if let egui::Event::PointerMoved(p) = e {
                                    Some(view.world(*p))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                    });
                    for p in samples {
                        if points
                            .last()
                            .is_none_or(|q| q.distance(p) * view.scale > 2.)
                        {
                            points.push(p);
                        }
                    }
                    if released || !primary_down {
                        points.push(world);
                        let points = points.clone();
                        let add = *add;
                        self.cut_area_selection(&mut state, &points, add);
                        state.drag = None;
                    }
                }
                Some(Drag::Pan(last)) => {
                    if primary_down || middle_down || released {
                        state.pan += screen - *last;
                        *last = screen;
                        view.center = area.center() + state.pan;
                        if released {
                            state.drag = None;
                        }
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
                Some(Drag::PenNode { index, start }) => {
                    if let Some(pen) = &mut state.pen {
                        if primary_down
                            && (!pen.closing || world.distance(*start) > 1. / view.scale)
                        {
                            let node = &mut pen.mark.nodes[*index];
                            let anchor = crate::vector::pos(node.position);
                            let mut delta = world - anchor;
                            if shift {
                                let angle = (delta.y.atan2(delta.x) / (std::f32::consts::PI / 4.))
                                    .round()
                                    * (std::f32::consts::PI / 4.);
                                delta = Vec2::angled(angle) * delta.length();
                            }
                            node.out_handle = crate::vector::xy(anchor + delta);
                            node.in_handle = if alt {
                                node.position
                            } else {
                                crate::vector::xy(anchor - delta)
                            };
                        }
                    }
                    if released || !primary_down {
                        state.drag = None;
                        if state.pen.as_ref().is_some_and(|p| p.closing) {
                            let pen = state.pen.take().unwrap();
                            self.commit_pen(pen);
                        }
                    }
                }
                Some(Drag::Width(d)) => {
                    if d.changed || world.distance(d.start) > 1.5 / view.scale {
                        if !d.changed {
                            self.checkpoint();
                            d.changed = true;
                        }
                        let mut mark = d.original.clone();
                        let half = (world - d.anchor).dot(d.normal).abs();
                        self.profile_point = Some(crate::width::edit_stop(
                            &mut mark.width_profile,
                            d.t,
                            (half * 2. / mark.width).clamp(0., 4.),
                        ));
                        self.width_profile = mark.width_profile.clone();
                        self.width_preset = crate::width::Preset::Custom;
                        self.doc.layers[d.id.0].frames[self.frame][d.id.1] = mark;
                    }
                    if released || !primary_down {
                        if d.changed {
                            self.recover();
                        }
                        state.drag = None;
                    }
                }
                Some(Drag::Node(d)) => {
                    if d.changed || world.distance(d.start) > 0.001 {
                        if !d.changed {
                            self.checkpoint();
                            d.changed = true;
                        }
                        let mut mark = editable_nodes(&d.original);
                        let node = &mut mark.nodes[d.index];
                        let delta = world - d.start;
                        let anchor = crate::vector::pos(node.position);
                        if d.handle == 0 {
                            node.position = crate::vector::xy(anchor + delta);
                            node.in_handle =
                                crate::vector::xy(crate::vector::pos(node.in_handle) + delta);
                            node.out_handle =
                                crate::vector::xy(crate::vector::pos(node.out_handle) + delta);
                        } else {
                            let value = crate::vector::pos(if d.handle == 1 {
                                node.in_handle
                            } else {
                                node.out_handle
                            }) + delta;
                            if d.handle == 1 {
                                node.in_handle = crate::vector::xy(value);
                                if !alt {
                                    node.out_handle = crate::vector::xy(anchor - (value - anchor));
                                }
                            } else {
                                node.out_handle = crate::vector::xy(value);
                                if !alt {
                                    node.in_handle = crate::vector::xy(anchor - (value - anchor));
                                }
                            }
                        }
                        self.doc.layers[d.id.0].frames[self.frame][d.id.1] = mark;
                    }
                    if released || !primary_down {
                        if d.changed {
                            self.recover();
                        }
                        state.drag = None;
                    }
                }
                Some(Drag::Draw { smoother, tool }) => {
                    if let Some(m) = &mut self.draft {
                        if matches!(*tool, Tool::Line | Tool::Ellipse | Tool::Polygon) {
                            let a = smoother.last();
                            let mut b = world;
                            if shift {
                                let delta = b - a;
                                if *tool == Tool::Line {
                                    let angle = (delta.angle() / std::f32::consts::FRAC_PI_4)
                                        .round()
                                        * std::f32::consts::FRAC_PI_4;
                                    b = a + Vec2::angled(angle) * delta.length();
                                } else {
                                    let size = delta.x.abs().max(delta.y.abs());
                                    b = a + Vec2::new(
                                        delta.x.signum() * size,
                                        delta.y.signum() * size,
                                    );
                                }
                            }
                            *m = crate::editing::shape(
                                *tool,
                                a,
                                b,
                                self.polygon_sides,
                                self.polygon_star,
                                self.star_ratio,
                                m,
                            );
                        } else if m.rectangle {
                            let p = crate::vector::xy(world);
                            if m.points.len() == 1 {
                                m.points.push(p);
                            } else {
                                m.points[1] = p;
                            }
                        } else {
                            let samples = ctx.input(|i| {
                                let first = if pressed {
                                    i.events
                                        .iter()
                                        .rposition(|e| {
                                            matches!(
                                                e,
                                                egui::Event::PointerButton {
                                                    button: PointerButton::Primary,
                                                    pressed: true,
                                                    ..
                                                }
                                            )
                                        })
                                        .map_or(0, |n| n + 1)
                                } else {
                                    0
                                };
                                i.events[first..]
                                    .iter()
                                    .filter_map(|e| match e {
                                        egui::Event::PointerMoved(p) => Some(*p),
                                        egui::Event::PointerButton {
                                            pos,
                                            button: PointerButton::Primary,
                                            pressed: false,
                                            ..
                                        } => Some(*pos),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                            });
                            for screen in samples {
                                let p = crate::vector::xy(view.world(screen));
                                if smoother.last().distance(crate::vector::pos(p)) > 0.08 {
                                    smoother.push(p, &mut m.points);
                                }
                            }
                        }
                    }
                    if released || !primary_down {
                        if let Some(mut m) = self.draft.take() {
                            let mut parts = vec![];
                            if matches!(
                                *tool,
                                Tool::Rectangle | Tool::Line | Tool::Ellipse | Tool::Polygon
                            ) {
                                parts.push(m);
                            } else {
                                let curves =
                                    crate::vector::fit(&m.points, 0.2 + self.smoothing * 3.8);
                                if !curves.is_empty() {
                                    m = crate::vector::from_cubics(&curves, &m);
                                }
                                if matches!(
                                    *tool,
                                    Tool::Brush
                                        | Tool::FillBrush
                                        | Tool::ShadeBrush
                                        | Tool::LightBrush
                                ) {
                                    parts = crate::vector::brush_fill(&m, m.color);
                                } else {
                                    parts.push(m);
                                }
                            }
                            parts.retain(|m| {
                                !matches!(
                                    *tool,
                                    Tool::Rectangle | Tool::Line | Tool::Ellipse | Tool::Polygon
                                ) || crate::vector::control_bounds(m).size().length() > 0.01
                            });
                            if !parts.is_empty() {
                                self.checkpoint();
                                let parts = self.symmetry_marks(parts);
                                if matches!(*tool, Tool::ShadeBrush | Tool::LightBrush) {
                                    self.tone_paint(parts, *tool == Tool::LightBrush);
                                } else if *tool == Tool::FillBrush {
                                    self.fill_brush(parts);
                                } else {
                                    let parts = self.prepare_style(parts);
                                    self.commit_paint(self.layer, self.frame, parts);
                                }
                                self.recover();
                            }
                        }
                        state.drag = None;
                    } else {
                        ctx.request_repaint();
                    }
                }
                Some(Drag::Marquee { start, end, add }) => {
                    *end = world;
                    if released || !primary_down {
                        let rect = Rect::from_two_pos(*start, *end);
                        let additive = *add;
                        if self.tool == Tool::Object {
                            if !additive {
                                state.clear_selection();
                            }
                            let seeds: Vec<_> = self
                                .doc
                                .layers
                                .iter()
                                .enumerate()
                                .filter(|(_, l)| l.visible && !l.options.locked)
                                .flat_map(|(l, layer)| {
                                    layer.frames[self.frame].iter().enumerate().filter_map(
                                        move |(m, mark)| {
                                            crate::vector::control_bounds(mark)
                                                .intersects(rect)
                                                .then_some((l, m))
                                        },
                                    )
                                })
                                .collect();
                            for seed in seeds {
                                self.select_connected_object(&mut state, seed, true);
                            }
                        } else {
                            self.cut_selection(&mut state, rect, additive);
                        }
                        state.drag = None;
                    }
                }
                Some(Drag::Transform(d)) => {
                    if d.last_input != Some((world, shift))
                        && (d.changed || world.distance(d.start) > 0.01)
                    {
                        d.last_input = Some((world, shift));
                        if !d.changed && !matches!(d.operation, Operation::Pivot) {
                            self.checkpoint();
                            self.autokey_motion(&state.selected);
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
                                    let track =
                                        self.doc.layers[*l].frames[self.frame][*m].style.track;
                                    let mut moved = transformed_in_basis(
                                        original,
                                        d.pivot,
                                        scale,
                                        angle,
                                        translation,
                                        d.bounds.angle,
                                    );
                                    moved.style.track = track;
                                    self.doc.layers[*l].frames[self.frame][*m] = moved;
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
        if self.tool == Tool::PolygonSelect
            && (response.double_clicked()
                || (!wants_keyboard && ctx.input(|i| i.key_pressed(egui::Key::Enter))))
        {
            let points = std::mem::take(&mut state.polygon_selection);
            let add = state.polygon_add;
            self.cut_area_selection(&mut state, &points, add);
        }
        if escape {
            state.polygon_selection.clear();
        }
        if self.tool == Tool::Pen && response.double_clicked() {
            if let Some(mut pen) = state.pen.take() {
                let n = pen.mark.nodes.len();
                if n > 1
                    && view
                        .screen(crate::vector::pos(pen.mark.nodes[n - 1].position))
                        .distance(view.screen(crate::vector::pos(pen.mark.nodes[n - 2].position)))
                        < 4.
                {
                    pen.mark.nodes.pop();
                }
                state.drag = None;
                self.commit_pen(pen);
            }
        }
        if over {
            ctx.set_cursor_icon(if matches!(state.drag, Some(Drag::Pan(_))) {
                CursorIcon::Grabbing
            } else if space || self.tool == Tool::Pan {
                CursorIcon::Grab
            } else if matches!(self.tool, Tool::Select | Tool::Transform | Tool::Object) {
                CursorIcon::Default
            } else if matches!(
                self.tool,
                Tool::Brush
                    | Tool::FillBrush
                    | Tool::ShadeBrush
                    | Tool::LightBrush
                    | Tool::VectorBrush
                    | Tool::Eraser
            ) {
                CursorIcon::None
            } else {
                CursorIcon::Default
            });
        }
        let corners = [
            Pos2::ZERO,
            Pos2::new(state.stage_size.x, 0.),
            Pos2::new(state.stage_size.x, state.stage_size.y),
            Pos2::new(0., state.stage_size.y),
        ];
        let screen: Vec<Pos2> = corners.into_iter().map(|p| stage_view.screen(p)).collect();
        painter.add(egui::Shape::convex_polygon(
            screen.clone(),
            Color32::WHITE,
            Stroke::new(1., Color32::from_gray(90)),
        ));
        if self.onion && !self.playing {
            let count = self.doc.layers[0].frames.len();
            for f in self.frame.saturating_sub(self.studio.onion_before)
                ..(self.frame + self.studio.onion_after + 1).min(count)
            {
                if f == self.frame {
                    continue;
                }
                for (_, _, m) in crate::features::resolve(&self.doc, f, false) {
                    paint_mark(&painter, &m, view, true, false, Some(&mut state.cache));
                }
            }
        }
        for (l, m, mark) in crate::features::resolve(&self.doc, self.frame, false) {
            paint_mark(
                &painter,
                &mark,
                view,
                false,
                state.selected.contains(&(l, m)),
                Some(&mut state.cache),
            );
        }
        if let Some(mark) = &self.draft {
            let previews = self.symmetry_marks(vec![mark.clone()]);
            if matches!(
                self.tool,
                Tool::FillBrush | Tool::ShadeBrush | Tool::LightBrush
            ) {
                let fills: Vec<_> = self.doc.layers[self.layer].frames[self.frame]
                    .iter()
                    .filter(|m| {
                        m.fill.is_some()
                            && m.style.instance.is_none()
                            && match self.tool {
                                Tool::ShadeBrush => m.style.shade < self.feature.toon_levels,
                                Tool::LightBrush => m.style.light < self.feature.toon_levels,
                                _ => true,
                            }
                    })
                    .cloned()
                    .map(|mut m| {
                        m.stroke = false;
                        m
                    })
                    .collect();
                let mask = crate::features::ink(&fills);
                for preview in previews {
                    let color = match self.tool {
                        Tool::ShadeBrush => [0, 0, 0, (self.studio.strength * 180.) as u8],
                        Tool::LightBrush => [255, 255, 255, (self.studio.strength * 180.) as u8],
                        _ => preview.color,
                    };
                    for ink in crate::vector::brush_fill(&preview, color) {
                        for part in crate::features::clip_to(&ink, &mask) {
                            paint_mark(&painter, &part, view, false, false, None);
                        }
                    }
                }
            } else {
                for preview in previews {
                    paint_mark(&painter, &preview, view, false, false, None);
                }
            }
        }
        self.stage_mask(&painter, area, stage_view);
        self.advanced_overlays(&painter, area, view);
        self.visual_canvas_overlays(&painter, view, &state.selected);
        self.paint_guides(&painter, area, view);
        if self.tool == Tool::Width {
            for &(l, m) in &state.selected {
                paint_width_handles(&painter, &self.doc.layers[l].frames[self.frame][m], view);
            }
        }
        if matches!(self.tool, Tool::Node | Tool::Select) {
            for &(l, m) in &state.selected {
                let mark = &self.doc.layers[l].frames[self.frame][m];
                if self.tool == Tool::Select && (!mark.stroke || mark.nodes.is_empty()) {
                    continue;
                }
                paint_nodes(
                    &painter,
                    &self.doc.layers[l].frames[self.frame][m],
                    view,
                    state
                        .node_selection
                        .filter(|(a, b, _)| *a == l && *b == m)
                        .map(|(_, _, n)| n),
                );
            }
        }
        if let Some(pen) = &state.pen {
            let mut preview = pen.mark.clone();
            if !preview.closed {
                preview.fill = None;
            }
            if !preview.stroke {
                preview.stroke = true;
                preview.color = ACCENT.to_array();
            }
            paint_mark(&painter, &preview, view, false, false, None);
            paint_nodes(&painter, &pen.mark, view, None);
            if state.drag.is_none() && over {
                if let (Some(last), Some(pointer)) = (pen.mark.nodes.last(), pointer) {
                    let end = view.world(pointer);
                    let curve = crate::vector::Cubic([
                        crate::vector::pos(last.position),
                        crate::vector::pos(last.out_handle),
                        end,
                        end,
                    ]);
                    let line = crate::vector::from_cubics(
                        &[curve],
                        &Mark {
                            color: ACCENT.to_array(),
                            width: 1. / view.scale,
                            ..Mark::default()
                        },
                    );
                    paint_mark(&painter, &line, view, false, false, None);
                }
            }
        }
        if let Some(bounds) = state.bounds(self) {
            if self.tool == Tool::Transform {
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
        state.actual_scale = view.scale;
        if let Some(Drag::Area { points, .. }) = &state.drag {
            let mut points: Vec<_> = points.iter().map(|p| view.screen(*p)).collect();
            if let Some(first) = points.first().copied() {
                points.push(first);
            }
            painter.add(egui::Shape::line(points, Stroke::new(1., ACCENT)));
        }
        if !state.polygon_selection.is_empty() {
            let mut points: Vec<_> = state
                .polygon_selection
                .iter()
                .map(|p| view.screen(*p))
                .collect();
            if let Some(p) = pointer {
                points.push(p);
            }
            if let Some(first) = points.first().copied() {
                points.push(first);
            }
            painter.add(egui::Shape::line(points, Stroke::new(1., ACCENT)));
        }
        if over
            && !space
            && matches!(
                self.tool,
                Tool::Brush
                    | Tool::FillBrush
                    | Tool::ShadeBrush
                    | Tool::LightBrush
                    | Tool::VectorBrush
                    | Tool::Eraser
            )
        {
            if let Some(pointer) = pointer {
                let radius = (self.width * view.scale * 0.5).max(1.);
                painter.circle_stroke(pointer, radius, Stroke::new(3., Color32::BLACK));
                painter.circle_stroke(pointer, radius, Stroke::new(1., Color32::WHITE));
            }
        }
        self.paint_rulers(&ruler_painter, ruler_area, view);
        self.sync_selected_style(&state.selected);
        self.canvas = state;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn mark() -> Mark {
        Mark {
            points: vec![[10., 20.], [50., 60.]].into(),
            color: [0, 0, 0, 255],
            width: 4.,
            rectangle: false,
            ..Mark::default()
        }
    }
    #[test]
    fn view_inverse_with_rotation_pan_zoom() {
        let v = View {
            center: Pos2::new(143., -22.),
            scale: 2.7,
            angle: 1.3,
            stage_center: Pos2::new(480., 270.),
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
            points: vec![[0., 0.], [40., 30.]].into(),
            color: [20, 30, 40, 255],
            width: 3.,
            rectangle: true,
            ..Mark::default()
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
        assert!(restored.svg(0).contains("<path"));
        assert_eq!(restored.layers[0].frames[0][0].points.len(), 5);
    }
}

fn fit_scale(area: Rect, size: Vec2) -> f32 {
    ((area.width() - 60.) / size.x)
        .min((area.height() - 60.) / size.y)
        .max(0.01)
}

#[cfg(test)]
mod feature_gesture_tests {
    use super::*;
    #[test]
    fn fill_brush_gesture_never_adds_ink_outside_the_fill_and_locked_layers_ignore_it() {
        for locked in [false, true] {
            let ctx = egui::Context::default();
            let cc = eframe::CreationContext::_new_kittest(ctx.clone());
            let mut e = Editor::new(&cc);
            e.doc = crate::Document::default();
            e.recovery = std::env::temp_dir().join(format!(
                "libretoon-fill-gesture-{}-{locked}.json",
                std::process::id()
            ));
            e.doc.layers[0].frames[0].push(Mark {
                points: vec![[0., 0.], [100., 100.]].into(),
                rectangle: true,
                closed: true,
                fill: Some([30, 80, 120, 255]),
                stroke: false,
                ..Default::default()
            });
            e.doc.layers[0].options.locked = locked;
            e.activate_tool(Tool::FillBrush);
            e.fill_color = Color32::from_rgb(230, 30, 60);
            e.width = 20.;
            e.smoothing = 0.;
            let tick = |e: &mut Editor, events: Vec<egui::Event>| {
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                        events,
                        ..Default::default()
                    },
                    |ui| e.canvas_ui(ui),
                );
                out.textures_delta.clear();
            };
            tick(&mut e, vec![]);
            let area = Rect::from_min_max(Pos2::new(8., 8.), Pos2::new(992., 692.));
            let view = e.canvas.view(area);
            let a = view.screen(Pos2::new(50., 50.));
            let b = view.screen(Pos2::new(180., 50.));
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            let before = serde_json::to_value(&e.doc).unwrap();
            tick(&mut e, vec![egui::Event::PointerMoved(a), button(a, true)]);
            tick(&mut e, vec![egui::Event::PointerMoved(b)]);
            tick(&mut e, vec![button(b, false)]);
            assert!(
                e.doc.layers[0].frames[0]
                    .iter()
                    .all(|m| crate::vector::control_bounds(m).right() <= 100.001)
            );
            if locked {
                assert_eq!(before, serde_json::to_value(&e.doc).unwrap());
            } else {
                assert!(
                    e.doc.layers[0].frames[0]
                        .iter()
                        .any(|m| m.fill == Some([230, 30, 60, 255]))
                );
            }
        }
    }
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
    crate::vector::transform(m, |p| {
        let v = rotate(p - pivot, -basis);
        let p =
            pivot + rotate(Vec2::new(v.x * scale.x, v.y * scale.y), basis + angle) + translation;
        Pos2::new(p.x.clamp(-1e7, 1e7), p.y.clamp(-1e7, 1e7))
    })
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
            ]
            .into(),
            color: [0, 0, 0, 255],
            width: 6.,
            rectangle: false,
            ..Mark::default()
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

struct PenDraft {
    mark: Mark,
    layer: usize,
    frame: usize,
    closing: bool,
}
struct NodeDrag {
    id: (usize, usize),
    index: usize,
    handle: u8,
    start: Pos2,
    original: Mark,
    changed: bool,
}

impl Canvas {
    pub fn cancel_drafts(&mut self) {
        self.pen = None;
        self.drag = None;
        self.node_selection = None;
    }
    pub fn undo_pen(&mut self) -> bool {
        if let Some(pen) = &mut self.pen {
            pen.mark.nodes.pop();
            if pen.mark.nodes.is_empty() {
                self.pen = None;
            }
            self.drag = None;
            true
        } else {
            false
        }
    }
}
impl Editor {
    pub fn finish_pending_pen(&mut self) {
        if let Some(pen) = self.canvas.pen.take() {
            self.commit_pen(pen);
        }
    }
    fn commit_pen(&mut self, mut pen: PenDraft) {
        if pen.mark.nodes.len() < 2
            || pen.layer >= self.doc.layers.len()
            || pen.frame >= self.doc.layers[pen.layer].frames.len()
        {
            return;
        }
        if !pen.mark.closed {
            if !pen.mark.stroke && pen.mark.fill.is_some() && pen.mark.nodes.len() >= 2 {
                pen.mark.closed = true;
            } else {
                pen.mark.fill = None;
            }
        }
        if !pen.mark.stroke && pen.mark.fill.is_none() {
            return;
        }
        self.checkpoint();
        let marks = self.prepare_style(self.symmetry_marks(vec![pen.mark]));
        self.commit_paint(pen.layer, pen.frame, marks);
        self.recover();
        self.status = "Trazado Bézier creado".into();
    }
    fn begin_pen(&mut self, state: &mut Canvas, world: Pos2, view: View) {
        if state.pen.is_none() {
            if !self.stroke_enabled && !self.fill_enabled {
                self.status = "Activa relleno o contorno para dibujar".into();
                return;
            }
            state.clear_selection();
            state.pen = Some(PenDraft {
                mark: Mark {
                    color: self.color.to_array(),
                    width: self.width,
                    width_profile: self.width_profile.clone(),
                    stroke: self.stroke_enabled,
                    fill: self.fill_enabled.then(|| self.fill_color.to_array()),
                    ..Mark::default()
                },
                layer: self.layer,
                frame: self.frame,
                closing: false,
            });
        }
        let pen = state.pen.as_mut().unwrap();
        if pen.mark.nodes.len() >= 2
            && view
                .screen(crate::vector::pos(pen.mark.nodes[0].position))
                .distance(view.screen(world))
                < 9.
        {
            pen.mark.closed = true;
            pen.closing = true;
            state.drag = Some(Drag::PenNode {
                index: 0,
                start: world,
            });
        } else {
            let index = pen.mark.nodes.len();
            pen.mark
                .nodes
                .push(crate::document::Node::corner(crate::vector::xy(world)));
            state.drag = Some(Drag::PenNode {
                index,
                start: world,
            });
        }
        self.status="Pluma: clic crea esquina · arrastrar crea curva · primer nodo cierra · Enter termina · Esc cancela".into();
    }
    fn start_node_drag(&mut self, state: &mut Canvas, world: Pos2, view: View) {
        let mut found = None;
        for &(l, m) in &state.selected {
            let mark = editable_nodes(&self.doc.layers[l].frames[self.frame][m]);
            for (index, node) in mark.nodes.iter().enumerate() {
                for (handle, p) in [
                    (1, node.in_handle),
                    (2, node.out_handle),
                    (0, node.position),
                ] {
                    if handle != 0 && p == node.position {
                        continue;
                    }
                    if view
                        .screen(crate::vector::pos(p))
                        .distance(view.screen(world))
                        < 8.
                    {
                        found = Some(((l, m), index, handle));
                    }
                }
            }
        }
        if let Some((id, index, handle)) = found {
            state.node_selection = Some((id.0, id.1, index));
            state.drag = Some(Drag::Node(NodeDrag {
                id,
                index,
                handle,
                start: world,
                original: self.doc.layers[id.0].frames[self.frame][id.1].clone(),
                changed: false,
            }));
            return;
        }
        'layers: for (l, layer) in self
            .doc
            .layers
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, l)| l.visible && !l.options.locked)
        {
            for (m, mark) in crate::animation::marks(layer, self.frame)
                .iter()
                .enumerate()
                .rev()
            {
                if hit(mark, world, 6. / view.scale) {
                    state.clear_selection();
                    state.selected.push((l, m));
                    self.layer = l;
                    let editable = editable_nodes(mark);
                    if let Some(index) = editable.nodes.iter().position(|node| {
                        view.screen(crate::vector::pos(node.position))
                            .distance(view.screen(world))
                            < 8.
                    }) {
                        state.node_selection = Some((l, m, index));
                        state.drag = Some(Drag::Node(NodeDrag {
                            id: (l, m),
                            index,
                            handle: 0,
                            start: world,
                            original: mark.clone(),
                            changed: false,
                        }));
                    }
                    break 'layers;
                }
            }
        }
    }
    fn cut_selection(&mut self, state: &mut Canvas, rect: Rect, add: bool) {
        if rect.width() < 0.001 || rect.height() < 0.001 {
            return;
        }
        let mut edits = vec![];
        let mut selected = vec![];
        let mut changed = false;
        for (l, layer) in self
            .doc
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.visible && !l.options.locked)
        {
            let mut marks = Vec::new();
            let mut layer_changed = false;
            for (m, mark) in layer.frames[self.frame].iter().enumerate() {
                let (outside, inside) = crate::vector::split_rect(mark, rect);
                let partial = !outside.is_empty() && !inside.is_empty();
                let previous = add && state.selected.contains(&(l, m));
                if inside.is_empty() {
                    if previous {
                        selected.push((l, marks.len()));
                    }
                    marks.push(mark.clone());
                } else if outside.is_empty() {
                    selected.push((l, marks.len()));
                    marks.push(mark.clone());
                } else {
                    layer_changed |= partial;
                    changed |= partial;
                    for part in outside {
                        if previous {
                            selected.push((l, marks.len()));
                        }
                        marks.push(part);
                    }
                    for part in inside {
                        selected.push((l, marks.len()));
                        marks.push(part);
                    }
                }
            }
            if layer_changed {
                edits.push((l, marks));
            }
        }
        if changed {
            self.checkpoint();
            for (l, marks) in edits {
                self.doc.layers[l].frames[self.frame] = marks.into();
            }
            self.recover();
            self.status = "Fragmento separado: puedes moverlo, transformarlo o borrarlo".into();
        }
        state.selected = selected;
        state.orientation = 0.;
        state.pivot = state.bounds(self).map(|b| b.center());
    }
    pub(crate) fn separate_component(
        &mut self,
        state: &mut Canvas,
        id: (usize, usize),
        stroke: bool,
    ) -> (usize, usize) {
        let mark = &self.doc.layers[id.0].frames[self.frame][id.1];
        if mark.fill.is_none() || !mark.stroke {
            return id;
        }
        let mut fill = mark.clone();
        fill.stroke = false;
        let mut line = mark.clone();
        line.fill = None;
        self.checkpoint();
        self.doc.layers[id.0].frames[self.frame][id.1] = fill;
        self.doc.layers[id.0].frames[self.frame].insert(id.1 + 1, line);
        for (l, m) in &mut state.selected {
            if *l == id.0 && *m >= id.1 {
                *m += 1;
            }
        }
        self.recover();
        (id.0, id.1 + usize::from(stroke))
    }
    pub fn apply_drawing_style(&mut self) {
        if self.canvas.selected.is_empty() || (!self.fill_enabled && !self.stroke_enabled) {
            return;
        }
        if !self.stroke_enabled
            && self.canvas.selected.iter().any(|&(l, m)| {
                let mark = &self.doc.layers[l].frames[self.frame][m];
                !mark.closed && !mark.rectangle
            })
        {
            self.status = "Cierra primero el trazado para aplicarle solo relleno".into();
            return;
        }
        let ids = self.canvas.selected.clone();
        self.checkpoint();
        for &(l, m) in &ids {
            let mark = &mut self.doc.layers[l].frames[self.frame][m];
            mark.color = self.color.to_array();
            mark.width = self.width;
            mark.width_profile = self.width_profile.clone();
            mark.stroke = self.stroke_enabled;
            mark.fill = if self.fill_enabled && (mark.closed || mark.rectangle) {
                Some(self.fill_color.to_array())
            } else {
                None
            };
        }
        self.canvas.selected = ids;
        self.recover();
    }
}
fn editable_nodes(mark: &Mark) -> Mark {
    if !mark.nodes.is_empty() {
        return mark.clone();
    }
    let mut editable = crate::vector::from_cubics(&crate::vector::cubics(mark), mark);
    editable.holes = mark.holes.clone();
    editable
}
fn paint_nodes(p: &egui::Painter, mark: &Mark, view: View, active: Option<usize>) {
    for (index, node) in editable_nodes(mark).nodes.iter().enumerate() {
        let anchor = view.screen(crate::vector::pos(node.position));
        for h in [node.in_handle, node.out_handle] {
            let handle = view.screen(crate::vector::pos(h));
            if handle.distance(anchor) > 0.5 {
                p.line_segment([anchor, handle], Stroke::new(1., ACCENT));
                p.circle_filled(handle, 3.4, Color32::from_rgb(34, 36, 44));
                p.circle_stroke(handle, 3.4, Stroke::new(1., ACCENT));
            }
        }
        let rect = Rect::from_center_size(
            anchor,
            Vec2::splat(if active == Some(index) { 8. } else { 6. }),
        );
        p.rect_filled(
            rect,
            0.,
            if active == Some(index) {
                ACCENT
            } else {
                Color32::from_rgb(35, 38, 44)
            },
        );
        p.rect_stroke(rect, 0., Stroke::new(1., ACCENT), egui::StrokeKind::Inside);
    }
}

#[cfg(test)]
mod vector_gesture_tests {
    use super::*;
    fn button(p: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn tick(ctx: &egui::Context, e: &mut Editor, events: Vec<egui::Event>) {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| e.canvas_ui(ui),
        );
        out.textures_delta.clear();
    }
    fn run(test: impl FnOnce(&egui::Context, &mut Editor)) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut e = Editor::new(&cc);
        e.doc = crate::Document::default();
        let recovery = std::env::temp_dir().join(format!(
            "libretoon-vector-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        e.recovery = recovery.clone();
        tick(&ctx, &mut e, vec![]);
        test(&ctx, &mut e);
        drop(e);
        let _ = std::fs::remove_file(recovery);
    }
    #[test]
    fn brush_can_start_and_finish_outside_the_white_stage() {
        run(|ctx, e| {
            e.tool = Tool::Brush;
            let a = Pos2::new(300., 20.);
            let b = Pos2::new(600., 30.);
            tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
            tick(ctx, e, vec![egui::Event::PointerMoved(b), button(b, false)]);
            assert!(!e.doc.layers[0].frames[0].is_empty());
            assert!(
                e.doc.layers[0].frames[0]
                    .iter()
                    .all(|m| crate::vector::control_bounds(m).max.y < 0.)
            );
            assert!(e.doc.validate().is_ok());
        });
    }
    #[test]
    fn all_pointer_samples_in_one_frame_survive_vector_brush_completion() {
        run(|ctx, e| {
            e.tool = Tool::VectorBrush;
            let a = Pos2::new(200., 200.);
            let middle = Pos2::new(350., 380.);
            let b = Pos2::new(500., 200.);
            tick(
                ctx,
                e,
                vec![
                    egui::Event::PointerMoved(a),
                    button(a, true),
                    egui::Event::PointerMoved(Pos2::new(270., 300.)),
                    egui::Event::PointerMoved(middle),
                    egui::Event::PointerMoved(Pos2::new(430., 300.)),
                    egui::Event::PointerMoved(b),
                    button(b, false),
                ],
            );
            assert_eq!(e.doc.layers[0].frames[0].len(), 1);
            let mark = &e.doc.layers[0].frames[0][0];
            assert!(!mark.nodes.is_empty());
            assert!(mark.stroke && mark.fill.is_none());
            let bounds = crate::vector::control_bounds(mark);
            assert!(bounds.height() > 130., "{bounds:?}");
        });
    }
    #[test]
    fn pen_click_drag_creates_real_handles_and_enter_finishes_open_path() {
        run(|ctx, e| {
            e.tool = Tool::Pen;
            for (a, b) in [
                (Pos2::new(200., 200.), Pos2::new(240., 160.)),
                (Pos2::new(400., 250.), Pos2::new(440., 290.)),
            ] {
                tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
                tick(ctx, e, vec![egui::Event::PointerMoved(b)]);
                tick(ctx, e, vec![button(b, false)]);
            }
            assert!(e.canvas.pen.is_some());
            tick(ctx, e, vec![key(egui::Key::Enter)]);
            let mark = &e.doc.layers[0].frames[0][0];
            assert_eq!(mark.nodes.len(), 2);
            assert!(!mark.closed);
            assert_ne!(mark.nodes[0].position, mark.nodes[0].out_handle);
            assert!(mark.fill.is_none());
        });
    }
    #[test]
    fn clicking_first_pen_anchor_closes_a_filled_shape() {
        run(|ctx, e| {
            e.tool = Tool::Pen;
            for a in [
                Pos2::new(200., 200.),
                Pos2::new(400., 200.),
                Pos2::new(350., 400.),
                Pos2::new(200., 200.),
            ] {
                tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
                tick(ctx, e, vec![button(a, false)]);
            }
            assert!(e.canvas.pen.is_none());
            let mark = &e.doc.layers[0].frames[0][0];
            assert_eq!(mark.nodes.len(), 3);
            assert!(mark.closed && mark.fill.is_some());
        });
    }
    #[test]
    fn rectangle_modes_keep_fill_and_stroke_independent() {
        for (fill, stroke) in [(true, false), (false, true), (true, true)] {
            run(|ctx, e| {
                e.tool = Tool::Rectangle;
                e.fill_enabled = fill;
                e.stroke_enabled = stroke;
                let a = Pos2::new(150., 150.);
                let b = Pos2::new(400., 400.);
                tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
                tick(ctx, e, vec![egui::Event::PointerMoved(b), button(b, false)]);
                let mark = &e.doc.layers[0].frames[0][0];
                assert_eq!(mark.fill.is_some(), fill);
                assert_eq!(mark.stroke, stroke);
            });
        }
    }
    #[test]
    fn selection_marquee_separates_fill_instead_of_requiring_whole_object() {
        run(|ctx, e| {
            e.tool = Tool::Select;
            e.doc.layers[0].frames[0].push(Mark {
                points: vec![[200., 100.], [500., 400.]].into(),
                rectangle: true,
                closed: true,
                fill: Some([80, 100, 200, 255]),
                stroke: false,
                ..Default::default()
            });
            let view = e
                .canvas
                .view(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.)));
            let a = view.screen(Pos2::new(150., 50.));
            let b = view.screen(Pos2::new(350., 250.));
            tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
            tick(ctx, e, vec![egui::Event::PointerMoved(b), button(b, false)]);
            assert!(e.doc.layers[0].frames[0].len() >= 2);
            assert!(!e.canvas.selected.is_empty());
            let (l, m) = e.canvas.selected[0];
            let selected = &e.doc.layers[l].frames[0][m];
            assert!(crate::vector::control_bounds(selected).max.x < 400.);
        });
    }
}

struct WidthDrag {
    id: (usize, usize),
    t: f32,
    anchor: Pos2,
    normal: Vec2,
    start: Pos2,
    original: Mark,
    changed: bool,
}
impl Editor {
    fn start_width_drag(&mut self, state: &mut Canvas, world: Pos2, view: View) {
        let mut hit = None;
        for (l, layer) in self
            .doc
            .layers
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, l)| l.visible && !l.options.locked)
        {
            for (m, mark) in layer.frames[self.frame]
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, m)| m.stroke)
            {
                if state.selected.contains(&(l, m)) {
                    for stop in &mark.width_profile {
                        if let Some((anchor, normal)) = crate::width::at_length(mark, stop.t) {
                            let radius = mark.width * stop.width * 0.5;
                            if [-1., 1.].iter().any(|side| {
                                view.screen(anchor + normal * radius * *side)
                                    .distance(view.screen(world))
                                    < 9.
                            }) {
                                hit = Some(((l, m), stop.t, anchor, normal));
                                break;
                            }
                        }
                    }
                }
                if hit.is_none() {
                    if let Some(p) = crate::width::nearest(mark, world) {
                        if p.distance
                            <= mark.width * crate::width::sample(&mark.width_profile, p.t) * 0.5
                                + 9. / view.scale
                        {
                            hit = Some(((l, m), p.t, p.point, p.normal));
                        }
                    }
                }
                if hit.is_some() {
                    break;
                }
            }
            if hit.is_some() {
                break;
            }
        }
        if let Some((id, t, anchor, normal)) = hit {
            let original = self.doc.layers[id.0].frames[self.frame][id.1].clone();
            state.clear_selection();
            state.selected.push(id);
            self.layer = id.0;
            self.width = original.width;
            self.width_profile = original.width_profile.clone();
            self.width_preset = crate::width::Preset::Custom;
            state.drag = Some(Drag::Width(WidthDrag {
                id,
                t,
                anchor,
                normal,
                start: world,
                original,
                changed: false,
            }));
        }
    }
    fn bucket_at(&mut self, state: &mut Canvas, world: Pos2) {
        if !self.fill_enabled {
            self.status = "Activa un color de relleno".into();
            return;
        }
        match crate::bucket::fill(
            &self.doc.layers[self.layer].frames[self.frame],
            world,
            self.gap,
            self.fill_color.to_array(),
        ) {
            Ok(shapes) => {
                let shapes = self.prepare_style(shapes);
                // Recolor an existing bucket face when its geometry is unchanged.
                let old = self.doc.layers[self.layer].frames[self.frame]
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, m)| m.bucket && crate::vector::contains_fill(m, world))
                    .map(|(i, _)| i);
                self.checkpoint();
                state.clear_selection();
                if let Some(index) = old.filter(|&i| {
                    let previous = &self.doc.layers[self.layer].frames[self.frame][i];
                    shapes.len() == 1
                        && previous.points == shapes[0].points
                        && previous.holes == shapes[0].holes
                }) {
                    self.doc.layers[self.layer].frames[self.frame][index].fill =
                        Some(self.fill_color.to_array());
                    self.doc.layers[self.layer].frames[self.frame][index].style =
                        shapes[0].style.clone();
                } else {
                    self.doc.layers[self.layer].frames[self.frame].extend(shapes);
                }
                self.recover();
                self.status = "Relleno aplicado".into();
            }
            Err(error) => self.status = error,
        }
    }
}
fn paint_width_handles(p: &egui::Painter, mark: &Mark, view: View) {
    let controls = if mark.width_profile.is_empty() {
        crate::width::Preset::Custom.stops()
    } else {
        mark.width_profile.clone()
    };
    for stop in &controls {
        if let Some((anchor, normal)) = crate::width::at_length(mark, stop.t) {
            let radius = mark.width * stop.width * 0.5;
            let left = view.screen(anchor - normal * radius);
            let right = view.screen(anchor + normal * radius);
            p.line_segment([left, right], Stroke::new(1., ACCENT));
            for handle in [left, right] {
                p.circle_filled(handle, 3.5, Color32::from_rgb(32, 35, 40));
                p.circle_stroke(handle, 3.5, Stroke::new(1., ACCENT));
            }
            p.circle_filled(view.screen(anchor), 2., Color32::WHITE);
        }
    }
}

#[cfg(test)]
mod styling_gesture_tests {
    use super::*;
    fn button(p: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
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
            |ui| e.canvas_ui(ui),
        );
        output.textures_delta.clear();
    }
    fn run(test: impl FnOnce(&egui::Context, &mut Editor)) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut e = Editor::new(&cc);
        e.doc = crate::Document::default();
        let path = std::env::temp_dir().join(format!(
            "libretoon-styling-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        e.recovery = path.clone();
        tick(&ctx, &mut e, vec![]);
        test(&ctx, &mut e);
        drop(e);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn resized_stage_keeps_world_screen_inverse_under_rotation() {
        let mut c = Canvas::default();
        c.stage_size = Vec2::new(3840., 2160.);
        c.zoom = 1.7;
        c.angle = 0.6;
        c.pan = Vec2::new(21., -44.);
        let v = c.view(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 800.)));
        let p = Pos2::new(-500., 3000.);
        assert!(v.world(v.screen(p)).distance(p) < 0.001);
        assert_eq!(v.stage_center, Pos2::new(1920., 1080.));
    }
    #[test]
    fn width_tool_widens_locally_and_undo_restores_uniform_centerline() {
        run(|ctx, e| {
            e.doc.layers[0].frames[0].push(Mark {
                points: vec![[100., 200.], [600., 200.]].into(),
                width: 20.,
                ..Default::default()
            });
            e.tool = Tool::Width;
            let view = e
                .canvas
                .view(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.)));
            let a = view.screen(Pos2::new(350., 200.));
            let b = view.screen(Pos2::new(350., 240.));
            tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
            tick(ctx, e, vec![egui::Event::PointerMoved(b)]);
            tick(ctx, e, vec![button(b, false)]);
            let mark = &e.doc.layers[0].frames[0][0];
            assert!(crate::width::sample(&mark.width_profile, 0.5) > 3.5);
            assert_eq!(mark.points.len(), 2);
            e.history(false);
            assert!(e.doc.layers[0].frames[0][0].width_profile.is_empty());
            e.history(true);
            assert!(crate::width::sample(&e.doc.layers[0].frames[0][0].width_profile, 0.5) > 3.5);
        });
    }
    #[test]
    fn bucket_click_creates_vector_fill_and_recolor_does_not_accumulate_faces() {
        run(|ctx, e| {
            e.doc.layers[0].frames[0].push(Mark {
                points: vec![[100., 100.], [500., 400.]].into(),
                rectangle: true,
                closed: true,
                width: 3.,
                ..Default::default()
            });
            e.tool = Tool::Bucket;
            let view = e
                .canvas
                .view(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.)));
            let point = view.screen(Pos2::new(300., 250.));
            tick(
                ctx,
                e,
                vec![egui::Event::PointerMoved(point), button(point, true)],
            );
            tick(ctx, e, vec![button(point, false)]);
            assert_eq!(e.doc.layers[0].frames[0].len(), 2);
            assert!(e.doc.layers[0].frames[0][1].bucket);
            e.fill_color = Color32::RED;
            tick(ctx, e, vec![button(point, true)]);
            tick(ctx, e, vec![button(point, false)]);
            assert_eq!(e.doc.layers[0].frames[0].len(), 2);
            assert_eq!(
                e.doc.layers[0].frames[0][1].fill,
                Some(Color32::RED.to_array())
            );
        });
    }
}

fn selected_node_hit(e: &Editor, state: &Canvas, world: Pos2, view: View) -> bool {
    state.selected.iter().any(|&(l, m)| {
        let mark = &e.doc.layers[l].frames[e.frame][m];
        mark.stroke
            && mark.nodes.iter().any(|n| {
                [n.position, n.in_handle, n.out_handle].iter().any(|p| {
                    view.screen(crate::vector::pos(*p))
                        .distance(view.screen(world))
                        < 8.
                })
            })
    })
}

#[cfg(test)]
mod new_tool_regressions {
    use super::*;
    use geo::Area;
    fn button(p: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        }
    }
    fn tick(ctx: &egui::Context, e: &mut Editor, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| e.canvas_ui(ui),
        );
        out.textures_delta.clear();
        out
    }
    fn run(test: impl FnOnce(&egui::Context, &mut Editor)) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut e = Editor::new(&cc);
        e.doc = crate::Document::default();
        let path = std::env::temp_dir().join(format!(
            "libretoon-new-tools-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        e.recovery = path.clone();
        tick(&ctx, &mut e, vec![]);
        test(&ctx, &mut e);
        drop(e);
        let _ = std::fs::remove_file(path);
    }
    fn rect(a: [f32; 2], b: [f32; 2], c: [u8; 4]) -> Mark {
        Mark {
            points: vec![a, b].into(),
            rectangle: true,
            closed: true,
            fill: Some(c),
            stroke: false,
            ..Default::default()
        }
    }
    fn view(e: &Editor) -> View {
        e.canvas
            .view(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.)))
    }
    fn click(ctx: &egui::Context, e: &mut Editor, p: Pos2) {
        tick(ctx, e, vec![egui::Event::PointerMoved(p), button(p, true)]);
        tick(ctx, e, vec![button(p, false)]);
    }
    #[test]
    fn camera_drag_moves_art_with_pointer_and_records_local_key() {
        run(|ctx, e| {
            e.doc = crate::fresh_document();
            e.add_camera();
            e.frame = 10;
            e.activate_tool(Tool::Camera);
            tick(ctx, e, vec![]);
            let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.));
            let original = e.camera_view(e.canvas.view(area));
            let stage = e.canvas.view(area);
            let p = Pos2::new(500., 350.);
            let q = p + Vec2::new(60., 20.);
            let world = original.world(p);
            tick(ctx, e, vec![egui::Event::PointerMoved(p), button(p, true)]);
            tick(ctx, e, vec![egui::Event::PointerMoved(q)]);
            tick(ctx, e, vec![button(q, false)]);
            let v = e.camera_view(e.canvas.view(area));
            assert!(
                v.screen(world).distance(q) < 0.01,
                "camera screen {:?}, expected {:?}",
                v.screen(world),
                q
            );
            assert_eq!(v.center, stage.center);
            let c = &e.doc.extras.cameras[0];
            assert_eq!(c.animation.keys.len(), 1);
            assert_eq!(c.animation.keys[0].frame, 10);
            assert_eq!(c.center, [480., 270.]);
            assert!(e.project_document().validate().is_ok());
        });
    }
    #[test]
    fn moving_held_art_creates_motion_key_instead_of_rewriting_first_frame() {
        run(|ctx, e| {
            e.doc = crate::fresh_document();
            let mark = rect([100., 100.], [200., 200.], [90, 60, 220, 255]);
            for f in &mut e.doc.layers[0].frames {
                *f = vec![mark.clone()].into();
            }
            e.frame = 10;
            e.activate_tool(Tool::Transform);
            tick(ctx, e, vec![]);
            e.canvas.selected = vec![(0, 0)];
            let v = view(e);
            let p = v.screen(Pos2::new(130., 140.));
            let q = p + Vec2::new(70., 0.);
            tick(ctx, e, vec![egui::Event::PointerMoved(p), button(p, true)]);
            tick(ctx, e, vec![egui::Event::PointerMoved(q)]);
            tick(ctx, e, vec![button(q, false)]);
            assert_eq!(e.doc.layers[0].frames[0][0].points, mark.points);
            assert!(e.doc.layers[0].key(10));
            assert_eq!(e.doc.layers[0].options.tweens.len(), 1);
            let mid =
                crate::vector::control_bounds(&crate::animation::marks(&e.doc.layers[0], 5)[0])
                    .center();
            let end = crate::vector::control_bounds(&e.doc.layers[0].frames[10][0]).center();
            assert!((mid.x - (150. + (end.x - 150.) / 2.)).abs() < 0.01);
        });
    }
    #[test]
    fn selected_fill_and_stroke_update_directly_and_undo_one_gesture() {
        run(|ctx, e| {
            let mut mark = rect([100., 100.], [300., 300.], [10, 20, 30, 255]);
            mark.stroke = true;
            mark.color = [90, 80, 70, 255];
            e.doc.layers[0].frames[0].push(mark);
            e.canvas.selected = vec![(0, 0)];
            e.sync_selected_style(&[(0, 0)]);
            assert_eq!(e.fill_color.to_array(), [10, 20, 30, 255]);
            assert_eq!(e.color.to_array(), [90, 80, 70, 255]);
            e.fill_color = Color32::RED;
            e.change_selected_style(true, false, false, false);
            e.width = 23.;
            e.change_selected_style(false, false, true, false);
            assert_eq!(e.undo.len(), 1);
            let m = &e.doc.layers[0].frames[0][0];
            assert_eq!(m.fill, Some(Color32::RED.to_array()));
            assert_eq!(m.width, 23.);
            assert_eq!(m.color, [90, 80, 70, 255]);
            e.history(false);
            assert_eq!(e.doc.layers[0].frames[0][0].fill, Some([10, 20, 30, 255]));
            tick(ctx, e, vec![]);
        });
    }
    #[test]
    fn select_tool_edits_visible_bezier_handles_without_switching_tools() {
        run(|ctx, e| {
            e.tool = Tool::Select;
            let curve = crate::vector::Cubic([
                Pos2::new(180., 200.),
                Pos2::new(220., 100.),
                Pos2::new(320., 100.),
                Pos2::new(360., 200.),
            ]);
            e.doc.layers[0].frames[0].push(crate::vector::from_cubics(&[curve], &Mark::default()));
            e.canvas.selected = vec![(0, 0)];
            tick(ctx, e, vec![]);
            let v = view(e);
            let a = v.screen(curve.0[1]);
            let b = a + Vec2::new(25., 30.);
            tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
            tick(ctx, e, vec![egui::Event::PointerMoved(b), button(b, false)]);
            let changed = &e.doc.layers[0].frames[0][0];
            assert!(crate::vector::pos(changed.nodes[0].out_handle).distance(curve.0[1]) > 20.);
            assert_eq!(changed.nodes[0].position, [180., 200.]);
            assert_eq!(e.undo.len(), 1);
            e.history(false);
            assert_eq!(
                e.doc.layers[0].frames[0][0].nodes[0].out_handle,
                [220., 100.]
            );
        });
    }
    #[test]
    fn crossing_selection_deletes_only_the_chosen_segment_and_saves_curves() {
        run(|ctx, e| {
            e.tool = Tool::Select;
            e.doc.layers[0].frames[0].push(Mark {
                points: vec![[100., 200.], [500., 200.]].into(),
                width_profile: crate::width::Preset::Tapered.stops(),
                ..Default::default()
            });
            e.doc.layers[0].frames[0].push(Mark {
                points: vec![[300., 100.], [300., 300.]].into(),
                ..Default::default()
            });
            let p = view(e).screen(Pos2::new(180., 200.));
            click(ctx, e, p);
            assert_eq!(e.doc.layers[0].frames[0].len(), 3);
            assert_eq!(e.canvas.selected.len(), 1);
            let (l, m) = e.canvas.selected[0];
            assert!(
                crate::vector::control_bounds(&e.doc.layers[l].frames[0][m])
                    .max
                    .x
                    < 301.
            );
            tick(
                ctx,
                e,
                vec![egui::Event::Key {
                    key: egui::Key::Delete,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
            );
            assert_eq!(e.doc.layers[0].frames[0].len(), 2);
            assert!(
                e.doc.layers[0].frames[0]
                    .iter()
                    .any(|m| crate::vector::control_bounds(m).min.x > 299.)
            );
            let copy = crate::fla::decode(&crate::fla::encode(&e.doc).unwrap()).unwrap();
            assert_eq!(copy.layers[0].frames[0].len(), 2);
            e.history(false);
            assert_eq!(e.doc.layers[0].frames[0].len(), 3);
        });
    }
    #[test]
    fn lasso_and_polygon_select_cut_real_areas_with_undo() {
        run(|ctx, e| {
            for tool in [Tool::Lasso, Tool::PolygonSelect] {
                e.doc = crate::Document::default();
                e.canvas.clear_selection();
                e.doc.layers[0].frames[0].push(rect([100., 100.], [400., 400.], [1, 2, 3, 255]));
                e.tool = tool;
                let v = view(e);
                let points = [
                    Pos2::new(150., 150.),
                    Pos2::new(350., 150.),
                    Pos2::new(150., 350.),
                ]
                .map(|p| v.screen(p));
                if tool == Tool::Lasso {
                    tick(
                        ctx,
                        e,
                        vec![
                            egui::Event::PointerMoved(points[0]),
                            button(points[0], true),
                        ],
                    );
                    tick(
                        ctx,
                        e,
                        vec![
                            egui::Event::PointerMoved(points[1]),
                            egui::Event::PointerMoved(points[2]),
                            button(points[0], false),
                        ],
                    );
                } else {
                    for p in points {
                        click(ctx, e, p);
                    }
                    tick(
                        ctx,
                        e,
                        vec![egui::Event::Key {
                            key: egui::Key::Enter,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Default::default(),
                        }],
                    );
                }
                assert!(!e.canvas.selected.is_empty(), "{}", tool.name());
                let selected: f64 = e
                    .canvas
                    .selected
                    .iter()
                    .map(|&(l, m)| {
                        crate::vector::polygon(&e.doc.layers[l].frames[0][m]).unsigned_area()
                    })
                    .sum();
                assert!((selected - 20000.).abs() < 2., "{} {selected}", tool.name());
                let total: f64 = e.doc.layers[0].frames[0]
                    .iter()
                    .map(|m| crate::vector::polygon(m).unsigned_area())
                    .sum();
                assert!((total - 90000.).abs() < 2.);
                e.history(false);
                assert_eq!(e.doc.layers[0].frames[0].len(), 1);
            }
        });
    }
    #[test]
    fn eraser_removes_fill_and_axis_sections_in_a_single_undo_step() {
        run(|ctx, e| {
            e.doc.layers[0].frames[0].push(rect([100., 100.], [400., 400.], [10, 20, 30, 255]));
            e.doc.layers[0].frames[0].push(Mark {
                points: vec![[80., 250.], [450., 250.]].into(),
                ..Default::default()
            });
            e.tool = Tool::Eraser;
            e.width = 40.;
            let v = view(e);
            let a = v.screen(Pos2::new(250., 80.));
            let b = v.screen(Pos2::new(250., 420.));
            tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
            tick(ctx, e, vec![egui::Event::PointerMoved(b), button(b, false)]);
            assert!(
                !e.doc.layers[0].frames[0]
                    .iter()
                    .any(|m| crate::vector::contains_fill(m, Pos2::new(250., 250.)))
            );
            assert!(!e.doc.layers[0].frames[0].iter().any(|m| stroke_hit(
                m,
                Pos2::new(250., 250.),
                0.
            )));
            assert_eq!(e.undo.len(), 1);
            e.history(false);
            assert_eq!(e.doc.layers[0].frames[0].len(), 2);
        });
    }
    #[test]
    fn new_shapes_are_editable_and_roundtrip_native_fla() {
        run(|ctx, e| {
            for tool in [Tool::Line, Tool::Ellipse, Tool::Polygon] {
                e.doc = crate::Document::default();
                e.canvas.clear_selection();
                e.tool = tool;
                e.polygon_star = true;
                e.polygon_sides = 7;
                let a = Pos2::new(200., 200.);
                let b = Pos2::new(480., 400.);
                tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
                tick(ctx, e, vec![egui::Event::PointerMoved(b), button(b, false)]);
                assert_eq!(e.doc.layers[0].frames[0].len(), 1);
                let m = &e.doc.layers[0].frames[0][0];
                assert_eq!(
                    m.nodes.len(),
                    match tool {
                        Tool::Line => 2,
                        Tool::Ellipse => 4,
                        _ => 14,
                    }
                );
                if tool == Tool::Ellipse {
                    assert_ne!(m.nodes[0].out_handle, m.nodes[0].position);
                }
                let copy = crate::fla::decode(&crate::fla::encode(&e.doc).unwrap()).unwrap();
                assert_eq!(copy.layers[0].frames[0][0].nodes, m.nodes);
            }
        });
    }
    #[test]
    fn wand_eyedropper_and_connected_object_selection_are_distinct() {
        run(|ctx, e| {
            e.doc.layers[0].frames[0].extend([
                rect([100., 100.], [200., 200.], [255, 0, 0, 255]),
                rect([300., 100.], [400., 200.], [255, 0, 0, 255]),
                rect([500., 100.], [600., 200.], [0, 0, 255, 255]),
            ]);
            e.tool = Tool::Wand;
            click(ctx, e, view(e).screen(Pos2::new(150., 150.)));
            assert_eq!(e.canvas.selected, vec![(0, 1), (0, 0)]);
            e.canvas.clear_selection(); // This case samples without applying to a selection.
            e.activate_tool(Tool::Eyedropper);
            click(ctx, e, view(e).screen(Pos2::new(550., 150.)));
            assert_eq!(e.fill_color, Color32::BLUE);
            e.doc.layers[0].frames[0].push(Mark {
                points: vec![[190., 150.], [310., 150.]].into(),
                width: 4.,
                ..Default::default()
            });
            e.activate_tool(Tool::Object);
            click(ctx, e, view(e).screen(Pos2::new(150., 150.)));
            assert_eq!(e.canvas.selected.len(), 3);
            assert!(!e.canvas.selected.contains(&(0, 2)));
        });
    }
    #[test]
    fn fill_face_selection_conserves_area_and_leaves_crossing_contour_untouched() {
        run(|ctx, e| {
            e.doc.layers[0].frames[0].extend([
                rect([100., 100.], [400., 400.], [255, 0, 0, 255]),
                Mark {
                    points: vec![[250., 80.], [250., 420.]].into(),
                    ..Default::default()
                },
            ]);
            e.tool = Tool::Select;
            click(ctx, e, view(e).screen(Pos2::new(150., 150.)));
            assert_eq!(e.canvas.selected.len(), 1);
            let (l, m) = e.canvas.selected[0];
            assert!(
                crate::vector::control_bounds(&e.doc.layers[l].frames[0][m])
                    .max
                    .x
                    < 251.
            );
            assert_eq!(
                e.doc.layers[0].frames[0]
                    .iter()
                    .filter(|m| m.stroke)
                    .count(),
                1
            );
            let total: f64 = e.doc.layers[0].frames[0]
                .iter()
                .filter(|m| m.fill.is_some())
                .map(|m| crate::vector::polygon(m).unsigned_area())
                .sum();
            assert!((total - 90000.).abs() < 0.2);
        });
    }
    #[test]
    fn hand_and_zoom_preserve_art_and_zoom_anchors_at_pointer() {
        run(|ctx, e| {
            e.tool = Tool::Pan;
            let a = Pos2::new(300., 300.);
            let b = Pos2::new(400., 350.);
            tick(ctx, e, vec![egui::Event::PointerMoved(a), button(a, true)]);
            tick(ctx, e, vec![egui::Event::PointerMoved(b), button(b, false)]);
            assert!(e.canvas.pan.length() > 100.);
            assert!(e.doc.layers[0].frames[0].is_empty());
            e.tool = Tool::Zoom;
            let before = view(e).world(a);
            let old = e.canvas.zoom;
            click(ctx, e, a);
            assert!(e.canvas.zoom > old);
            assert!(view(e).world(a).distance(before) < 0.01);
            e.zoom_out = true;
            click(ctx, e, a);
            assert!((e.canvas.zoom - old).abs() < 0.01);
        });
    }
    #[test]
    fn brush_cursor_uses_high_contrast_radius_and_recovers_default_outside_canvas() {
        run(|ctx, e| {
            e.tool = Tool::Brush;
            e.width = 30.;
            let p = Pos2::new(300., 300.);
            let out = tick(ctx, e, vec![egui::Event::PointerMoved(p)]);
            assert_eq!(out.platform_output.cursor_icon, CursorIcon::None);
            let expected = e.width * e.canvas.actual_scale * 0.5;
            assert!(out.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Circle(c) if (c.radius-expected).abs()<0.01&&c.stroke.color==Color32::WHITE)));
            let out = tick(
                ctx,
                e,
                vec![egui::Event::PointerMoved(Pos2::new(-20., -20.))],
            );
            assert_eq!(out.platform_output.cursor_icon, CursorIcon::Default);
        });
    }
}
