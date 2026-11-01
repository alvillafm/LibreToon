//! Ordered library resources and colored folders, shared by all scene/symbol
//! editing contexts. Dragging changes references and ordering, never file paths.
use crate::{
    Editor,
    design::{self, Icon},
    features::id,
    i18n::tr,
    shared::SharedVec,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub folders: SharedVec<Folder>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: u64,
    pub name: String,
    pub parent: Option<u64>,
    pub color: [u8; 4],
    pub collapsed: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Asset {
    Folder(u64),
    Symbol(u64),
    Root,
}
#[derive(Default)]
pub struct LibraryState {
    pub selected: Option<Asset>,
    drag: Option<Asset>,
    rename: Option<(Asset, String)>,
    focus: bool,
}
impl Library {
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
    }
    pub fn can_parent(&self, id: u64, mut parent: Option<u64>) -> bool {
        let mut seen = HashSet::new();
        while let Some(p) = parent {
            if p == id || !seen.insert(p) || seen.len() > 32 {
                return false;
            }
            let Some(f) = self.folders.iter().find(|f| f.id == p) else {
                return false;
            };
            parent = f.parent;
        }
        true
    }
    pub fn validate(&self, symbols: &[crate::features::Symbol]) -> Result<(), String> {
        let mut ids = HashSet::new();
        if self.folders.len() > 200 {
            return Err("Too many library folders".into());
        }
        for f in &self.folders {
            if f.id == 0
                || !ids.insert(f.id)
                || f.name.len() > 1024
                || !self.can_parent(f.id, f.parent)
            {
                return Err("Invalid library folder hierarchy".into());
            }
        }
        if symbols
            .iter()
            .any(|s| s.library_folder.is_some_and(|f| !ids.contains(&f)))
        {
            return Err("Missing symbol library folder".into());
        }
        Ok(())
    }
}
fn rows(e: &Editor, parent: Option<u64>, depth: usize, out: &mut Vec<(Asset, usize)>) {
    if depth > 32 {
        return;
    }
    for f in e
        .doc
        .extras
        .library
        .folders
        .iter()
        .filter(|f| f.parent == parent)
    {
        out.push((Asset::Folder(f.id), depth));
        if !f.collapsed {
            rows(e, Some(f.id), depth + 1, out);
        }
    }
    for s in e
        .doc
        .extras
        .symbols
        .iter()
        .filter(|s| s.library_folder == parent)
    {
        out.push((Asset::Symbol(s.id), depth));
    }
}
impl Editor {
    pub fn reorder_library_folder(&mut self, id: u64, target: u64, before: bool) {
        if id == target {
            return;
        }
        let Some(parent) = self
            .doc
            .extras
            .library
            .folders
            .iter()
            .find(|f| f.id == target)
            .map(|f| f.parent)
        else {
            return;
        };
        if !self.doc.extras.library.can_parent(id, parent) {
            return;
        }
        let Some(index) = self
            .doc
            .extras
            .library
            .folders
            .iter()
            .position(|f| f.id == id)
        else {
            return;
        };
        self.checkpoint();
        let mut f = self.doc.extras.library.folders.remove(index);
        f.parent = parent;
        let at = self
            .doc
            .extras
            .library
            .folders
            .iter()
            .position(|f| f.id == target)
            .unwrap()
            + usize::from(!before);
        self.doc.extras.library.folders.insert(at, f);
        self.recover();
    }
    pub fn move_library_asset(&mut self, source: Asset, target: Asset) {
        if source == target || source == Asset::Root {
            return;
        }
        let parent = match target {
            Asset::Root => None,
            Asset::Folder(id) => Some(id),
            Asset::Symbol(id) => self
                .doc
                .extras
                .symbols
                .iter()
                .find(|s| s.id == id)
                .and_then(|s| s.library_folder),
        };
        if let Asset::Folder(id) = source {
            if !self.doc.extras.library.can_parent(id, parent) {
                self.status = tr("No se pueden crear ciclos de carpetas").into();
                return;
            }
        }
        self.checkpoint();
        match source {
            Asset::Folder(id) => {
                if let Some(i) = self
                    .doc
                    .extras
                    .library
                    .folders
                    .iter()
                    .position(|f| f.id == id)
                {
                    self.doc.extras.library.folders[i].parent = parent;
                    if let Asset::Symbol(_) = target {
                    } else if target == Asset::Root {
                        let f = self.doc.extras.library.folders.remove(i);
                        self.doc.extras.library.folders.push(f);
                    }
                }
            }
            Asset::Symbol(id) => {
                if let Some(i) = self.doc.extras.symbols.iter().position(|s| s.id == id) {
                    let mut s = self.doc.extras.symbols.remove(i);
                    s.library_folder = parent;
                    let at = match target {
                        Asset::Symbol(other) => self
                            .doc
                            .extras
                            .symbols
                            .iter()
                            .position(|s| s.id == other)
                            .unwrap_or(self.doc.extras.symbols.len()),
                        _ => self.doc.extras.symbols.len(),
                    };
                    self.doc.extras.symbols.insert(at, s);
                }
            }
            Asset::Root => {}
        }
        self.recover();
    }
    fn rename_asset(&mut self, asset: Asset, name: String) {
        let name = name.trim();
        if name.is_empty() || name.len() > 1024 {
            return;
        }
        self.checkpoint();
        match asset {
            Asset::Folder(id) => {
                if let Some(f) = self
                    .doc
                    .extras
                    .library
                    .folders
                    .iter_mut()
                    .find(|f| f.id == id)
                {
                    f.name = name.into();
                }
            }
            Asset::Symbol(id) => {
                if let Some(s) = self.doc.extras.symbols.iter_mut().find(|s| s.id == id) {
                    s.name = name.into();
                }
            }
            Asset::Root => {}
        }
        self.recover();
    }
    pub fn asset_library_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading(tr("Biblioteca"));
        ui.add_space(6.);
        let mut new_folder = false;
        let mut edit = None;
        let mut insert = None;
        let mut remove = None;
        let mut rename = None;
        let mut color = None;
        let mut drop_target = None;
        let mut reorder = None;
        ui.horizontal(|ui| {
            if design::button(
                ui,
                Icon::FolderPlus,
                "",
                "Nueva carpeta",
                false,
                self.doc.extras.library.folders.len() < 200,
            )
            .clicked()
            {
                new_folder = true;
            }
            if design::button(
                ui,
                Icon::Rename,
                "",
                "Renombrar",
                false,
                self.assets.selected.is_some(),
            )
            .clicked()
            {
                rename = self.assets.selected;
            }
            if design::button(
                ui,
                Icon::Trash,
                "",
                "Eliminar recurso o carpeta",
                false,
                self.assets.selected.is_some(),
            )
            .clicked()
            {
                remove = self.assets.selected;
            }
        });
        let root = ui.add_sized(
            [ui.available_width(), 28.],
            egui::Button::new(tr("Todos los recursos · soltar para sacar de carpeta")).wrap(),
        );
        if root.clicked() {
            self.assets.selected = Some(Asset::Root);
        }
        if root
            .rect
            .contains(ui.input(|i| i.pointer.interact_pos().unwrap_or(Pos2::ZERO)))
        {
            drop_target = Some(Asset::Root);
        }
        let mut list = vec![];
        rows(self, None, 0, &mut list);
        let mut commit_rename = None;
        egui::ScrollArea::vertical()
            .id_salt("library-resources")
            .max_height((ui.available_height() - 115.).max(120.))
            .show_rows(ui, 44., list.len(), |ui, visible| {
                for row in visible {
                    let (asset, depth) = list[row];
                    let (name, tag, collapsed) = match asset {
                        Asset::Folder(id) => {
                            let f = self
                                .doc
                                .extras
                                .library
                                .folders
                                .iter()
                                .find(|f| f.id == id)
                                .unwrap();
                            (
                                f.name.clone(),
                                Color32::from_rgba_unmultiplied(
                                    f.color[0], f.color[1], f.color[2], f.color[3],
                                ),
                                f.collapsed,
                            )
                        }
                        Asset::Symbol(id) => {
                            let s = self.doc.extras.symbols.iter().find(|s| s.id == id).unwrap();
                            let tag = s
                                .library_folder
                                .and_then(|id| {
                                    self.doc.extras.library.folders.iter().find(|f| f.id == id)
                                })
                                .map_or(design::ACCENT, |f| {
                                    Color32::from_rgb(f.color[0], f.color[1], f.color[2])
                                });
                            (s.name.clone(), tag, false)
                        }
                        Asset::Root => continue,
                    };
                    let (r, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 42.),
                        Sense::click_and_drag(),
                    );
                    if self.assets.selected == Some(asset) || response.hovered() {
                        ui.painter().rect_filled(
                            r,
                            4.,
                            if self.assets.selected == Some(asset) {
                                Color32::from_rgb(45, 64, 87)
                            } else {
                                Color32::from_gray(51)
                            },
                        );
                    }
                    ui.painter().rect_filled(
                        Rect::from_min_size(r.min + Vec2::new(2., 5.), Vec2::new(3., 32.)),
                        1.,
                        tag,
                    );
                    let icon = Rect::from_min_size(
                        r.min + Vec2::new(10. + depth as f32 * 12., 9.),
                        Vec2::splat(24.),
                    );
                    match asset {
                        Asset::Folder(_) => {
                            design::draw(ui.painter(), icon, Icon::Folder, tag);
                            if collapsed {
                                ui.painter().text(
                                    icon.center() + Vec2::new(12., 0.),
                                    egui::Align2::CENTER_CENTER,
                                    "+",
                                    egui::FontId::proportional(10.),
                                    tag,
                                );
                            }
                        }
                        Asset::Symbol(id) => {
                            ui.scope_builder(
                                egui::UiBuilder::new().max_rect(Rect::from_min_size(
                                    icon.min - Vec2::new(0., 3.),
                                    Vec2::new(32., 30.),
                                )),
                                |ui| {
                                    self.symbol_thumbnail(ui, id, 0, Vec2::new(32., 30.));
                                },
                            );
                        }
                        _ => {}
                    }
                    let text_rect = Rect::from_min_max(
                        Pos2::new(icon.right() + 14., r.top() + 7.),
                        r.max - Vec2::new(6., 7.),
                    );
                    if self
                        .assets
                        .rename
                        .as_ref()
                        .is_some_and(|(id, _)| *id == asset)
                    {
                        let (_, buffer) = self.assets.rename.as_mut().unwrap();
                        let edit = ui.put(text_rect, egui::TextEdit::singleline(buffer));
                        if self.assets.focus {
                            edit.request_focus();
                            self.assets.focus = false;
                        } else if edit.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            commit_rename = Some((asset, buffer.clone()));
                        }
                    } else {
                        let mut job = egui::text::LayoutJob::simple(
                            name.clone(),
                            egui::FontId::proportional(12.),
                            Color32::from_gray(225),
                            text_rect.width(),
                        );
                        job.wrap.max_rows = 1;
                        let galley = ui.painter().layout_job(job);
                        ui.painter().galley(
                            text_rect.min + Vec2::new(0., 4.),
                            galley,
                            Color32::WHITE,
                        );
                    }
                    // Register the row above the thumbnail's click widget, so
                    // dragging its preview behaves like dragging its name.
                    // Keep the inline rename field interactive while editing.
                    let response = if self
                        .assets
                        .rename
                        .as_ref()
                        .is_some_and(|(id, _)| *id == asset)
                    {
                        response
                    } else {
                        ui.interact(
                            r,
                            ui.scope_id().with(("library-row", row)),
                            Sense::click_and_drag(),
                        )
                    };
                    if response.clicked() {
                        self.assets.selected = Some(asset);
                        if let Asset::Symbol(id) = asset {
                            self.feature.selected_symbol = Some(id);
                        }
                    }
                    if response.double_clicked() {
                        match asset {
                            Asset::Folder(id) => {
                                if let Some(f) = self
                                    .doc
                                    .extras
                                    .library
                                    .folders
                                    .iter_mut()
                                    .find(|f| f.id == id)
                                {
                                    f.collapsed = !f.collapsed;
                                }
                            }
                            Asset::Symbol(id) => edit = Some(id),
                            _ => {}
                        }
                    }
                    if response.drag_started() {
                        self.assets.drag = Some(asset);
                    }
                    if response
                        .rect
                        .contains(ui.input(|i| i.pointer.interact_pos().unwrap_or(Pos2::ZERO)))
                    {
                        drop_target = Some(asset);
                        if let (Some(Asset::Folder(source)), Asset::Folder(target), Some(pos)) = (
                            self.assets.drag,
                            asset,
                            ui.input(|i| i.pointer.interact_pos()),
                        ) {
                            if pos.y < r.top() + 10. || pos.y > r.bottom() - 10. {
                                let before = pos.y < r.center().y;
                                reorder = Some((source, target, before));
                                let y = if before { r.top() } else { r.bottom() };
                                ui.painter().line_segment(
                                    [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
                                    egui::Stroke::new(2., design::ACCENT),
                                );
                            }
                        }
                    }
                    response.context_menu(|ui| {
                        if ui.button(tr("Renombrar")).clicked() {
                            rename = Some(asset);
                            ui.close();
                        }
                        if let Asset::Folder(id) = asset {
                            design::category(ui, "COLOR DE CARPETA");
                            let mut c = tag;
                            if ui.color_edit_button_srgba(&mut c).changed() {
                                color = Some((id, c.to_srgba_unmultiplied()));
                            }
                        }
                        if ui.button(tr("Sacar a la raíz")).clicked() {
                            self.assets.drag = Some(asset);
                            drop_target = Some(Asset::Root);
                            ui.close();
                        }
                    });
                }
            });
        if let Some((asset, name)) = commit_rename {
            self.assets.rename = None;
            self.rename_asset(asset, name);
        }
        if let Some(Asset::Symbol(id)) = self.assets.selected {
            ui.separator();
            ui.horizontal(|ui| {
                if design::button(
                    ui,
                    Icon::Plus,
                    "",
                    "Insertar símbolo en el contexto actual",
                    false,
                    self.symbol_insert_allowed(id),
                )
                .clicked()
                {
                    insert = Some(id);
                }
                if design::button(ui, Icon::Node, "", "Editar símbolo", false, true).clicked() {
                    edit = Some(id);
                }
                if design::button(
                    ui,
                    Icon::Trash,
                    "",
                    "Eliminar símbolo",
                    false,
                    !self.symbol_in_use(id),
                )
                .clicked()
                {
                    remove = Some(Asset::Symbol(id));
                }
            });
        }
        ui.small(tr(
            "Arrastra recursos entre carpetas. Doble clic abre; Renombrar cambia el nombre.",
        ));
        if new_folder {
            let parent = match self.assets.selected {
                Some(Asset::Folder(id)) => Some(id),
                _ => None,
            };
            self.checkpoint();
            let folder = Folder {
                id: id(),
                name: tr("Nueva carpeta").into(),
                parent,
                color: [91, 158, 255, 255],
                collapsed: false,
            };
            let id = folder.id;
            let name = folder.name.clone();
            self.doc.extras.library.folders.push(folder);
            self.assets.selected = Some(Asset::Folder(id));
            self.assets.rename = Some((Asset::Folder(id), name));
            self.assets.focus = true;
            self.recover();
        }
        if let Some(asset) = rename {
            let name = match asset {
                Asset::Folder(id) => self
                    .doc
                    .extras
                    .library
                    .folders
                    .iter()
                    .find(|f| f.id == id)
                    .map(|f| f.name.clone()),
                Asset::Symbol(id) => self
                    .doc
                    .extras
                    .symbols
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| s.name.clone()),
                _ => None,
            };
            if let Some(name) = name {
                self.assets.rename = Some((asset, name));
                self.assets.focus = true;
            }
        }
        if let Some((id, c)) = color {
            self.checkpoint();
            if let Some(f) = self
                .doc
                .extras
                .library
                .folders
                .iter_mut()
                .find(|f| f.id == id)
            {
                f.color = c;
            }
            self.recover();
        }
        if let Some(asset) = remove {
            match asset {
                Asset::Symbol(id) => {
                    if self.symbol_in_use(id) {
                        self.status = tr("El símbolo está en uso").into();
                    } else {
                        self.checkpoint();
                        self.doc.extras.symbols.retain(|s| s.id != id);
                        self.recover();
                        self.assets.selected = None;
                    }
                }
                Asset::Folder(id) => {
                    let parent = self
                        .doc
                        .extras
                        .library
                        .folders
                        .iter()
                        .find(|f| f.id == id)
                        .and_then(|f| f.parent);
                    self.checkpoint();
                    for f in &mut self.doc.extras.library.folders {
                        if f.parent == Some(id) {
                            f.parent = parent;
                        }
                    }
                    for s in &mut self.doc.extras.symbols {
                        if s.library_folder == Some(id) {
                            s.library_folder = parent;
                        }
                    }
                    self.doc.extras.library.folders.retain(|f| f.id != id);
                    self.assets.selected = None;
                    self.recover();
                }
                _ => {}
            }
        }
        if ui.input(|i| i.pointer.primary_released()) {
            if let Some((source, target, before)) = reorder {
                self.assets.drag = None;
                self.reorder_library_folder(source, target, before);
            } else if let (Some(source), Some(target)) = (self.assets.drag.take(), drop_target) {
                self.move_library_asset(source, target);
            }
        }
        if let Some(id) = insert {
            self.insert_symbol(id);
        }
        if let Some(id) = edit {
            self.enter_symbol(id);
        }
    }
}
