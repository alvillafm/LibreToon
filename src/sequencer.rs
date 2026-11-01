use crate::{
    Editor,
    design::{ACCENT, Icon, MUTED, button, draw},
    document::Layer,
    features::{Folder, LayerOptions, id},
    i18n::tr,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
#[derive(Clone, Copy)]
enum Row {
    Layer(usize, usize),
    Folder(usize, usize),
    Cameras,
}
fn rows(e: &Editor, parent: Option<u64>, depth: usize, out: &mut Vec<Row>) {
    if depth > 200 {
        return;
    }
    for (index, folder) in e
        .doc
        .extras
        .folders
        .iter()
        .enumerate()
        .filter(|(_, f)| f.parent == parent)
    {
        out.push(Row::Folder(index, depth));
        if !folder.collapsed {
            rows(e, Some(folder.id), depth + 1, out);
        }
    }
    for (index, _) in e
        .doc
        .layers
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, l)| l.options.folder == parent)
    {
        out.push(Row::Layer(index, depth));
    }
}
impl Editor {
    pub fn timeline_edit(&mut self, action: usize) {
        if self.doc.layers[self.layer].options.locked
            || self.doc.layers[self.layer].options.mannequin.is_some()
        {
            return;
        }
        self.finish_pending_pen();
        self.checkpoint();
        let f = self.frame;
        let layer = &mut self.doc.layers[self.layer];
        layer.init_keys();
        let exposure = layer.exposure(f);
        if action == 0 && crate::animation::span(layer, f).is_some() {
            layer.frames[f] = crate::animation::marks(layer, f);
        }
        match action {
            0 => {
                layer.options.keys[f] = true;
            }
            1 => {
                layer.options.keys[f] = true;
                layer.frames[f] = Default::default();
                for i in f + 1..exposure.end {
                    layer.frames[i] = Default::default();
                }
            }
            2 => {
                if layer.frames.len() < 10000 {
                    let content = layer.frames[f].clone();
                    for t in &mut layer.options.tweens {
                        if t.start >= exposure.end {
                            t.start += 1;
                        }
                        if t.end >= exposure.end {
                            t.end += 1;
                        }
                    }
                    layer.frames.insert(exposure.end, content);
                    layer.options.keys.insert(exposure.end, false);
                }
            }
            3 => {
                if exposure.len() > 1 {
                    for t in &mut layer.options.tweens {
                        if t.start >= exposure.end {
                            t.start -= 1;
                        }
                        if t.end >= exposure.end {
                            t.end -= 1;
                        }
                    }
                    layer.frames.remove(exposure.end - 1);
                    layer.options.keys.remove(exposure.end - 1);
                }
            }
            4 => {
                let at = exposure.start;
                layer
                    .options
                    .tweens
                    .retain(|s| s.start != at && s.end != at);
                let content = if at == 0 {
                    Default::default()
                } else {
                    layer.frames[at - 1].clone()
                };
                if at > 0 {
                    layer.options.keys[at] = false;
                }
                for i in at..exposure.end {
                    layer.frames[i] = content.clone();
                }
            }

            _ => {}
        }
        let count = self
            .doc
            .layers
            .iter()
            .map(|l| l.frames.len())
            .max()
            .unwrap_or(1)
            .max(1);
        for l in &mut self.doc.layers {
            if l.frames.len() < count {
                l.init_keys();
                let content = if action == 2 {
                    l.frames.last().cloned().unwrap_or_default()
                } else {
                    Default::default()
                };
                l.frames.resize(count, content);
                l.options.keys.resize(count, action != 2);
            }
        }
        self.frame = self.frame.min(count - 1);
        self.fit_camera_duration();
        self.recover();
    }
    fn add_layer_advanced(&mut self) {
        self.finish_pending_pen();
        if self.doc.layers.len() >= 100 {
            return;
        }
        self.checkpoint();
        let count = self.doc.layers[0].frames.len();
        let mut keys = vec![false; count];
        keys[0] = true;
        let number = self.doc.layers.len() + 1;
        self.doc.layers.push(Layer {
            name: format!("{} {}", tr("Capa"), number),
            visible: true,
            frames: vec![Default::default(); count].into(),
            options: LayerOptions {
                folder: self.feature.selected_folder,
                keys: keys.into(),
                ..Default::default()
            },
        });
        self.layer = self.doc.layers.len() - 1;
        self.recover();
    }
    fn delete_folder(&mut self) {
        if let Some(id) = self.feature.selected_folder.take() {
            if let Some(parent) = self
                .doc
                .extras
                .folders
                .iter()
                .find(|f| f.id == id)
                .map(|f| f.parent)
            {
                self.checkpoint();
                for f in &mut self.doc.extras.folders {
                    if f.parent == Some(id) {
                        f.parent = parent;
                    }
                }
                for l in &mut self.doc.layers {
                    if l.options.folder == Some(id) {
                        l.options.folder = parent;
                    }
                }
                self.doc.extras.folders.retain(|f| f.id != id);
                self.recover();
            }
        }
    }
    pub fn sequencer(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("timeline")
            .exact_size(295.)
            .resizable(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new(tr("TIMELINE")).size(10.).color(MUTED));
                ui.separator();
                ui.horizontal(|ui| {
                    if button(
                        ui,
                        Icon::First,
                        "",
                        "Primer fotograma",
                        false,
                        self.frame > 0,
                    )
                    .clicked()
                    {
                        self.frame = 0;
                        self.playing = false;
                    }
                    if button(
                        ui,
                        Icon::Previous,
                        "",
                        "Fotograma anterior",
                        false,
                        self.frame > 0,
                    )
                    .clicked()
                    {
                        self.frame -= 1;
                        self.playing = false;
                    }
                    if button(
                        ui,
                        if self.playing {
                            Icon::Pause
                        } else {
                            Icon::Play
                        },
                        "",
                        "Reproducir",
                        self.playing,
                        true,
                    )
                    .clicked()
                    {
                        self.animation.play_all = false;
                        self.playing = !self.playing;
                        self.tick = std::time::Instant::now();
                    }
                    if button(
                        ui,
                        Icon::Next,
                        "",
                        "Fotograma siguiente",
                        false,
                        self.frame + 1 < self.doc.layers[0].frames.len(),
                    )
                    .clicked()
                    {
                        self.frame += 1;
                        self.playing = false;
                    }
                    if button(
                        ui,
                        Icon::Last,
                        "",
                        "Último fotograma",
                        false,
                        self.frame + 1 < self.doc.layers[0].frames.len(),
                    )
                    .clicked()
                    {
                        self.frame = self.doc.layers[0].frames.len() - 1;
                        self.playing = false;
                    }
                    ui.separator();
                    ui.label(format!(
                        "{:03} / {}",
                        self.frame + 1,
                        self.doc.layers[0].frames.len()
                    ));
                    let mut fps = self.doc.fps;
                    if ui
                        .add(
                            egui::DragValue::new(&mut fps)
                                .range(1..=240)
                                .suffix(" fps")
                                .update_while_editing(false),
                        )
                        .changed()
                    {
                        self.set_fps(fps);
                    }
                    if !self.duration_editing {
                        self.timeline_duration =
                            self.doc.layers[0].frames.len() as f32 / self.doc.fps as f32;
                    }
                    let response = ui.add(
                        egui::DragValue::new(&mut self.timeline_duration)
                            .range(1. / self.doc.fps as f32..=10000. / self.doc.fps as f32)
                            .speed(0.05)
                            .suffix(" s")
                            .update_while_editing(false),
                    );
                    let editing = response.dragged() || response.has_focus();
                    if response.drag_stopped()
                        || response.lost_focus()
                        || (response.changed() && !editing)
                    {
                        self.set_duration(self.timeline_duration);
                    }
                    self.duration_editing = editing;
                    ui.separator();
                    if button(ui, Icon::Onion, "", "Papel cebolla", self.onion, true).clicked() {
                        self.onion = !self.onion;
                    }
                    ui.add(
                        egui::DragValue::new(&mut self.studio.onion_before)
                            .range(0..=24)
                            .prefix("< "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.studio.onion_after)
                            .range(0..=24)
                            .suffix(" >"),
                    );
                });
                ui.separator();
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(112.);
                        ui.label(egui::RichText::new(tr("CAPAS")).small().color(MUTED));
                        ui.separator();
                        ui.horizontal(|ui| {
                            if button(
                                ui,
                                Icon::Layers,
                                "",
                                "Añadir capa",
                                false,
                                self.doc.layers.len() < 100,
                            )
                            .clicked()
                            {
                                self.add_layer_advanced();
                            }
                            if button(
                                ui,
                                Icon::Trash,
                                "",
                                "Eliminar capa",
                                false,
                                self.doc.layers.len() > 1,
                            )
                            .clicked()
                            {
                                self.checkpoint();
                                self.finish_pending_pen();
                                self.doc.layers.remove(self.layer);
                                self.layer = self.layer.min(self.doc.layers.len() - 1);
                                self.renaming_layer = None;
                                self.recover();
                            }
                            if button(
                                ui,
                                Icon::Mask,
                                "",
                                "Máscara de recorte",
                                self.doc.layers[self.layer].options.clip,
                                self.layer > 0 || self.doc.layers[self.layer].options.clip,
                            )
                            .clicked()
                            {
                                self.checkpoint();
                                self.doc.layers[self.layer].options.clip =
                                    !self.doc.layers[self.layer].options.clip;
                                self.recover();
                            }
                        });
                    });
                    let (divider, _) = ui.allocate_exact_size(Vec2::new(1., 58.), Sense::hover());
                    ui.painter().line_segment(
                        [divider.left_top(), divider.left_bottom()],
                        Stroke::new(1., crate::design::LINE),
                    );
                    ui.vertical(|ui| {
                        ui.set_width(76.);
                        ui.label(egui::RichText::new(tr("CARPETAS")).small().color(MUTED));
                        ui.separator();
                        ui.horizontal(|ui| {
                            if button(
                                ui,
                                Icon::Folder,
                                "",
                                "Crear carpeta",
                                false,
                                self.doc.extras.folders.len() < 200,
                            )
                            .clicked()
                            {
                                self.checkpoint();
                                let id = id();
                                let number = self.doc.extras.folders.len() + 1;
                                self.doc.extras.folders.push(Folder {
                                    id,
                                    name: format!("Folder {}", number),
                                    parent: self.feature.selected_folder,
                                    collapsed: false,
                                });
                                self.feature.selected_folder = Some(id);
                                self.recover();
                            }
                            if button(
                                ui,
                                Icon::Trash,
                                "",
                                "Eliminar carpeta",
                                false,
                                self.feature.selected_folder.is_some(),
                            )
                            .clicked()
                            {
                                self.delete_folder();
                            }
                        });
                    });
                    let (divider, _) = ui.allocate_exact_size(Vec2::new(1., 58.), Sense::hover());
                    ui.painter().line_segment(
                        [divider.left_top(), divider.left_bottom()],
                        Stroke::new(1., crate::design::LINE),
                    );
                    ui.vertical(|ui| {
                        ui.set_width(240.);
                        ui.label(egui::RichText::new(tr("FOTOGRAMAS")).small().color(MUTED));
                        ui.separator();
                        ui.horizontal(|ui| {
                            if button(ui, Icon::Tween, "", "Motion tween", false, true).clicked() {
                                self.create_motion();
                            }
                            for (action, icon, tip) in [
                                (0, Icon::Copy, "Fotograma clave"),
                                (1, Icon::New, "Fotograma clave en blanco"),
                                (2, Icon::Plus, "Ampliar duración de fotograma"),
                                (3, Icon::Previous, "Reducir duración de fotograma"),
                                (4, Icon::Trash, "Eliminar fotograma clave"),
                            ] {
                                if button(
                                    ui,
                                    icon,
                                    "",
                                    tip,
                                    false,
                                    !self.doc.layers[self.layer].options.locked,
                                )
                                .clicked()
                                {
                                    self.timeline_edit(action);
                                }
                            }
                        });
                    });
                });
                ui.separator();
                let mut all = vec![];
                if !self.doc.extras.cameras.is_empty() {
                    all.push(Row::Cameras);
                }
                rows(self, None, 0, &mut all);
                let mut target = None;
                let mut camera_target = None;
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let count = self.doc.layers[0].frames.len();
                        let label_w = 300.;
                        let cell = 22.;
                        let row_h = 28.;
                        let (r, _) = ui.allocate_exact_size(
                            Vec2::new(
                                label_w + count as f32 * cell,
                                24. + all.len() as f32 * row_h,
                            ),
                            Sense::hover(),
                        );
                        let painter = ui.painter().clone();
                        let clip = ui.clip_rect();
                        let frames = crate::workspace::visible_range(
                            r.left() + label_w,
                            cell,
                            count,
                            clip.left(),
                            clip.right(),
                        );
                        let visible = crate::workspace::visible_range(
                            r.top() + 24.,
                            row_h,
                            all.len(),
                            clip.top(),
                            clip.bottom(),
                        );
                        let header = Rect::from_min_size(r.min, Vec2::new(r.width(), 24.));
                        painter.rect_filled(header, 0., Color32::from_gray(31));
                        painter.text(
                            r.min + Vec2::new(60., 12.),
                            egui::Align2::LEFT_CENTER,
                            tr("Nombre de capa"),
                            egui::FontId::proportional(10.),
                            MUTED,
                        );
                        if Rect::from_min_size(r.min, Vec2::new(label_w, 24.))
                            .contains(ui.input(|i| i.pointer.interact_pos().unwrap_or(Pos2::ZERO)))
                        {
                            target = Some((false, usize::MAX));
                        }
                        let scrub = ui.interact(
                            Rect::from_min_max(Pos2::new(r.left() + label_w, r.top()), header.max),
                            ui.scope_id().with("scrub"),
                            Sense::click_and_drag(),
                        );
                        if scrub.is_pointer_button_down_on() {
                            if let Some(p) = scrub.interact_pointer_pos() {
                                self.frame = (((p.x - r.left() - label_w) / cell).floor().max(0.)
                                    as usize)
                                    .min(count - 1);
                                self.playing = false;
                            }
                        }
                        for f in frames.clone() {
                            if f == 0 || (f + 1) % 5 == 0 {
                                painter.text(
                                    Pos2::new(
                                        r.left() + label_w + (f as f32 + 0.5) * cell,
                                        r.top() + 12.,
                                    ),
                                    egui::Align2::CENTER_CENTER,
                                    (f + 1).to_string(),
                                    egui::FontId::monospace(10.),
                                    MUTED,
                                );
                            }
                        }
                        for row in visible {
                            let y = r.top() + 24. + row as f32 * row_h;
                            let label = Rect::from_min_size(
                                Pos2::new(r.left(), y),
                                Vec2::new(label_w, row_h),
                            );
                            match all[row] {
                                Row::Folder(index, depth) => {
                                    let folder = self.doc.extras.folders[index].clone();
                                    painter.rect_filled(
                                        label,
                                        0.,
                                        if self.feature.selected_folder == Some(folder.id) {
                                            Color32::from_rgb(50, 70, 85)
                                        } else {
                                            Color32::from_gray(39)
                                        },
                                    );
                                    draw(
                                        &painter,
                                        Rect::from_min_size(
                                            label.min + Vec2::new(8. + depth as f32 * 12., 4.),
                                            Vec2::splat(20.),
                                        ),
                                        Icon::Folder,
                                        MUTED,
                                    );
                                    let response = ui.interact(
                                        label,
                                        ui.scope_id().with(("folder", folder.id)),
                                        Sense::click_and_drag(),
                                    );
                                    painter.text(
                                        label.min + Vec2::new(35. + depth as f32 * 12., 14.),
                                        egui::Align2::LEFT_CENTER,
                                        &folder.name,
                                        egui::FontId::proportional(11.),
                                        MUTED,
                                    );
                                    if response.clicked() {
                                        self.feature.selected_folder = Some(folder.id);
                                    }
                                    if response.double_clicked() {
                                        self.doc.extras.folders[index].collapsed =
                                            !folder.collapsed;
                                    }
                                    response.context_menu(|ui| {
                                        let mut name = folder.name.clone();
                                        if ui.text_edit_singleline(&mut name).changed() {
                                            self.checkpoint();
                                            self.doc.extras.folders[index].name = name;
                                            self.recover();
                                        }
                                    });
                                    if response.drag_started() {
                                        self.feature.row_drag = Some((true, index));
                                    }
                                    if label.contains(
                                        ui.input(|i| {
                                            i.pointer.interact_pos().unwrap_or(Pos2::ZERO)
                                        }),
                                    ) {
                                        target = Some((true, index));
                                    }
                                }
                                Row::Layer(l, depth) => {
                                    painter.rect_filled(
                                        label,
                                        0.,
                                        if self.doc.layers[l].options.mannequin.is_some() {
                                            Color32::from_rgb(69, 48, 91)
                                        } else if l == self.layer {
                                            Color32::from_rgb(43, 65, 94)
                                        } else {
                                            Color32::from_gray(42)
                                        },
                                    );
                                    let eye = Rect::from_min_size(
                                        label.min + Vec2::new(3., 3.),
                                        Vec2::splat(22.),
                                    );
                                    draw(
                                        &painter,
                                        eye,
                                        if self.doc.layers[l].visible {
                                            Icon::Eye
                                        } else {
                                            Icon::EyeOff
                                        },
                                        MUTED,
                                    );
                                    if ui
                                        .interact(
                                            eye,
                                            ui.scope_id().with(("eye", l)),
                                            Sense::click(),
                                        )
                                        .clicked()
                                    {
                                        self.checkpoint();
                                        self.finish_pending_pen();
                                        self.doc.layers[l].visible = !self.doc.layers[l].visible;
                                        self.recover();
                                    }
                                    let lock = eye.translate(Vec2::new(25., 0.));
                                    draw(
                                        &painter,
                                        lock,
                                        Icon::Lock,
                                        if self.doc.layers[l].options.locked {
                                            ACCENT
                                        } else {
                                            Color32::from_gray(85)
                                        },
                                    );
                                    if ui
                                        .interact(
                                            lock,
                                            ui.scope_id().with(("lock", l)),
                                            Sense::click(),
                                        )
                                        .clicked()
                                    {
                                        self.checkpoint();
                                        self.finish_pending_pen();
                                        self.doc.layers[l].options.locked =
                                            !self.doc.layers[l].options.locked;
                                        self.recover();
                                    }
                                    let name_rect = Rect::from_min_max(
                                        label.min + Vec2::new(57. + depth as f32 * 10., 2.),
                                        Pos2::new(label.right() - 60., label.bottom() - 2.),
                                    );
                                    if self.renaming_layer == Some(l) {
                                        let mut name = self.doc.layers[l].name.clone();
                                        let response = ui
                                            .put(name_rect, egui::TextEdit::singleline(&mut name));
                                        if self.rename_focus {
                                            response.request_focus();
                                            self.rename_focus = false;
                                        }
                                        if response.changed() {
                                            self.checkpoint();
                                            self.doc.layers[l].name = name;
                                            self.recover();
                                        }
                                        if response.lost_focus() {
                                            self.renaming_layer = None;
                                        }
                                    } else {
                                        painter.text(
                                            name_rect.left_center(),
                                            egui::Align2::LEFT_CENTER,
                                            &self.doc.layers[l].name,
                                            egui::FontId::proportional(11.),
                                            Color32::from_gray(210),
                                        );
                                        let response = ui.interact(
                                            name_rect,
                                            ui.scope_id().with(("layer-row", l)),
                                            Sense::click_and_drag(),
                                        );
                                        if response.clicked() {
                                            self.layer = l;
                                            self.feature.selected_folder =
                                                self.doc.layers[l].options.folder;
                                        }
                                        if response.double_clicked() {
                                            self.renaming_layer = Some(l);
                                            self.rename_focus = true;
                                        }
                                        if response.drag_started() {
                                            self.feature.row_drag = Some((false, l));
                                        }
                                    }
                                    let mut opacity = self.doc.layers[l].options.opacity * 100.;
                                    if ui
                                        .put(
                                            Rect::from_min_max(
                                                Pos2::new(label.right() - 56., y + 2.),
                                                label.max - Vec2::splat(2.),
                                            ),
                                            egui::DragValue::new(&mut opacity)
                                                .range(0. ..=100.)
                                                .suffix("%")
                                                .update_while_editing(false),
                                        )
                                        .changed()
                                    {
                                        self.checkpoint();
                                        self.doc.layers[l].options.opacity = opacity / 100.;
                                        self.recover();
                                    }
                                    if label.contains(
                                        ui.input(|i| {
                                            i.pointer.interact_pos().unwrap_or(Pos2::ZERO)
                                        }),
                                    ) {
                                        target = Some((false, l));
                                    }
                                    for f in frames.clone() {
                                        let rect = Rect::from_min_size(
                                            Pos2::new(r.left() + label_w + f as f32 * cell, y),
                                            Vec2::new(cell, row_h),
                                        );
                                        let key = self.doc.layers[l].key(f);
                                        let color =
                                            if self.doc.layers[l].options.mannequin.is_some() {
                                                Color32::from_rgb(71, 49, 94)
                                            } else if crate::animation::span(&self.doc.layers[l], f)
                                                .is_some()
                                                && f != self.frame
                                            {
                                                Color32::from_rgb(95, 61, 145)
                                            } else if f == self.frame {
                                                Color32::from_rgb(50, 88, 130)
                                            } else if key {
                                                Color32::from_gray(54)
                                            } else {
                                                Color32::from_gray(45)
                                            };
                                        painter.rect_filled(rect, 0., color);
                                        painter.rect_stroke(
                                            rect,
                                            0.,
                                            Stroke::new(0.5, Color32::from_gray(30)),
                                            egui::StrokeKind::Inside,
                                        );
                                        if self.doc.layers[l].options.mannequin.is_some() {
                                            if f == 0 {
                                                draw(
                                                    &painter,
                                                    rect.shrink(4.),
                                                    Icon::Mannequin,
                                                    crate::mannequin::COLOR,
                                                );
                                            }
                                        } else if key {
                                            if self.doc.layers[l].frames[f].is_empty() {
                                                painter.circle_stroke(
                                                    rect.center(),
                                                    3.,
                                                    Stroke::new(1., MUTED),
                                                );
                                            } else {
                                                painter.circle_filled(
                                                    rect.center(),
                                                    3.5,
                                                    if f == self.frame { ACCENT } else { MUTED },
                                                );
                                            }
                                        } else {
                                            painter.line_segment(
                                                [rect.left_center(), rect.right_center()],
                                                Stroke::new(1., Color32::from_gray(85)),
                                            );
                                        }
                                        if ui
                                            .interact(
                                                rect,
                                                ui.scope_id().with(("cell", l, f)),
                                                Sense::click(),
                                            )
                                            .clicked()
                                        {
                                            self.layer = l;
                                            self.frame = f;
                                            self.playing = false;
                                        }
                                    }
                                }
                                Row::Cameras => {
                                    let orange = Color32::from_rgb(230, 157, 55);
                                    painter.rect_filled(label, 0., Color32::from_rgb(80, 56, 32));
                                    draw(
                                        &painter,
                                        Rect::from_min_size(
                                            label.min + Vec2::new(8., 4.),
                                            Vec2::splat(20.),
                                        ),
                                        Icon::Camera,
                                        orange,
                                    );
                                    painter.text(
                                        label.min + Vec2::new(36., 14.),
                                        egui::Align2::LEFT_CENTER,
                                        tr("Cámaras"),
                                        egui::FontId::proportional(11.),
                                        orange,
                                    );
                                    for (index, c) in
                                        self.doc.extras.cameras.clone().iter().enumerate()
                                    {
                                        let rect = Rect::from_min_size(
                                            Pos2::new(
                                                r.left() + label_w + c.start as f32 * cell,
                                                y + 1.,
                                            ),
                                            Vec2::new(c.duration as f32 * cell, row_h - 2.),
                                        );
                                        if !rect.intersects(clip) {
                                            continue;
                                        }
                                        painter.rect_filled(
                                            rect,
                                            3.,
                                            if self.feature.selected_camera == Some(c.id) {
                                                Color32::from_rgb(159, 101, 36)
                                            } else {
                                                Color32::from_rgb(105, 73, 39)
                                            },
                                        );
                                        painter.rect_stroke(
                                            rect,
                                            3.,
                                            Stroke::new(1., orange),
                                            egui::StrokeKind::Inside,
                                        );
                                        painter.text(
                                            rect.left_center() + Vec2::new(6., 0.),
                                            egui::Align2::LEFT_CENTER,
                                            &c.name,
                                            egui::FontId::proportional(11.),
                                            orange,
                                        );
                                        let response = ui.interact(
                                            rect,
                                            ui.scope_id().with(("camera-cut", c.id)),
                                            Sense::click_and_drag(),
                                        );
                                        for key in &c.animation.keys {
                                            let f = c.start + key.frame;
                                            if !frames.contains(&f) {
                                                continue;
                                            }
                                            let center = Pos2::new(
                                                r.left() + label_w + (f as f32 + 0.5) * cell,
                                                rect.bottom() - 4.,
                                            );
                                            painter.add(egui::Shape::convex_polygon(
                                                vec![
                                                    center + Vec2::new(-3., 0.),
                                                    center + Vec2::new(0., -3.),
                                                    center + Vec2::new(3., 0.),
                                                    center + Vec2::new(0., 3.),
                                                ],
                                                orange,
                                                Stroke::NONE,
                                            ));
                                        }
                                        if response.drag_started() {
                                            self.feature.camera_drag = Some(index);
                                        }
                                        if rect.contains(ui.input(|i| {
                                            i.pointer.interact_pos().unwrap_or(Pos2::ZERO)
                                        })) {
                                            camera_target = Some(index);
                                        }
                                        if response.clicked() {
                                            self.feature.selected_camera = Some(c.id);
                                            self.frame = c.start;
                                            self.activate_tool(crate::canvas::Tool::Camera);
                                        }
                                    }
                                }
                            }
                        }
                        let x = r.left() + label_w + (self.frame as f32 + 0.5) * cell;
                        painter.line_segment(
                            [Pos2::new(x, r.top()), Pos2::new(x, r.bottom())],
                            Stroke::new(1., ACCENT),
                        );
                    });
                if ui.input(|i| i.pointer.primary_released()) {
                    if let (Some(source), Some(target)) =
                        (self.feature.camera_drag.take(), camera_target)
                    {
                        self.move_camera(source, target);
                    }
                    if let (Some(source), Some(target)) = (self.feature.row_drag.take(), target) {
                        if source != target {
                            self.reorder_row(source, target);
                        }
                    }
                }
            });
    }
    pub fn move_camera(&mut self, source: usize, target: usize) {
        if source == target
            || source >= self.doc.extras.cameras.len()
            || target >= self.doc.extras.cameras.len()
        {
            return;
        }
        self.checkpoint();
        let camera = self.doc.extras.cameras.remove(source);
        self.doc.extras.cameras.insert(target, camera);
        self.normalize_cameras();
        self.recover();
    }
    fn reorder_row(&mut self, source: (bool, usize), target: (bool, usize)) {
        self.finish_pending_pen();
        let parent = if target.1 == usize::MAX {
            None
        } else if target.0 {
            Some(self.doc.extras.folders[target.1].id)
        } else {
            self.doc.layers[target.1].options.folder
        };
        if source.0 {
            let id = self.doc.extras.folders[source.1].id;
            let mut test = parent;
            for _ in 0..=self.doc.extras.folders.len() {
                if test == Some(id) {
                    return;
                }
                test = test.and_then(|id| {
                    self.doc
                        .extras
                        .folders
                        .iter()
                        .find(|f| f.id == id)
                        .and_then(|f| f.parent)
                });
                if test.is_none() {
                    break;
                }
            }
            self.checkpoint();
            self.doc.extras.folders[source.1].parent = parent;
        } else {
            self.checkpoint();
            self.doc.layers[source.1].options.folder = parent;
            if !target.0 && target.1 != usize::MAX {
                let layer = self.doc.layers.remove(source.1);
                let index = target.1.min(self.doc.layers.len());
                self.doc.layers.insert(index, layer);
                self.layer = index;
            } else {
                self.layer = source.1;
            }
        }
        self.recover();
    }
}
