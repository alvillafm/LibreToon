//! UI palettes only. Artwork, stage white, reference-layer colors and exports
//! retain their document colors. A thread-local palette isolates render tests.
use eframe::egui::{self, Color32, Stroke};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Studio,
    Paper,
    Midnight,
    Forest,
    Plum,
    Graphite,
    Porcelain,
    Sand,
    Aurora,
    Ember,
}
#[derive(Clone, Copy)]
pub struct Palette {
    pub panel: Color32,
    pub canvas: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub selection: Color32,
    pub light: bool,
}
thread_local! {static CURRENT:std::cell::Cell<Theme>=const{std::cell::Cell::new(Theme::Studio)};}
impl Theme {
    pub const ALL: [Self; 10] = [
        Self::Studio,
        Self::Paper,
        Self::Midnight,
        Self::Forest,
        Self::Plum,
        Self::Graphite,
        Self::Porcelain,
        Self::Sand,
        Self::Aurora,
        Self::Ember,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Studio => "Estudio oscuro",
            Self::Paper => "Papel claro",
            Self::Midnight => "Medianoche",
            Self::Forest => "Bosque",
            Self::Plum => "Ciruela",
            Self::Graphite => "Grafito",
            Self::Porcelain => "Porcelain",
            Self::Sand => "Sand",
            Self::Aurora => "Aurora",
            Self::Ember => "Ember",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Studio => "El aspecto oscuro original con acento azul.",
            Self::Paper => "Paneles claros y tinta azul para trabajar de día.",
            Self::Midnight => "Azul profundo con acentos cian.",
            Self::Forest => "Grises verdes y acentos menta.",
            Self::Plum => "Paneles ciruela con acentos lavanda.",
            Self::Graphite => "Grises neutros con acentos ámbar.",
            Self::Porcelain => "Cool ivory panels with crisp indigo accents.",
            Self::Sand => "Warm cream panels with terracotta accents.",
            Self::Aurora => "Deep teal panels with luminous lime accents.",
            Self::Ember => "Charcoal aubergine panels with coral accents.",
        }
    }
    pub fn palette(self) -> Palette {
        let rgb = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
        let (panel, canvas, raised, border, text, muted, accent, selection, light) = match self {
            Self::Studio => (
                [39, 40, 44],
                [29, 30, 34],
                [61, 64, 72],
                [61, 63, 69],
                [218, 220, 226],
                [150, 154, 164],
                [91, 158, 255],
                [47, 73, 104],
                false,
            ),
            Self::Paper => (
                [242, 244, 248],
                [221, 226, 234],
                [229, 234, 242],
                [163, 176, 194],
                [30, 42, 60],
                [78, 94, 115],
                [28, 91, 175],
                [205, 225, 250],
                true,
            ),
            Self::Midnight => (
                [22, 33, 51],
                [12, 21, 35],
                [35, 53, 74],
                [60, 82, 107],
                [225, 239, 248],
                [153, 179, 201],
                [79, 218, 237],
                [28, 75, 90],
                false,
            ),
            Self::Forest => (
                [29, 43, 38],
                [18, 30, 25],
                [44, 62, 53],
                [68, 92, 77],
                [228, 239, 229],
                [160, 187, 169],
                [108, 225, 162],
                [35, 82, 57],
                false,
            ),
            Self::Plum => (
                [43, 31, 49],
                [28, 20, 34],
                [63, 46, 73],
                [97, 73, 107],
                [242, 230, 247],
                [193, 163, 204],
                [211, 161, 250],
                [87, 53, 111],
                false,
            ),
            Self::Graphite => (
                [40, 40, 40],
                [25, 25, 25],
                [61, 61, 61],
                [90, 90, 90],
                [238, 236, 230],
                [178, 175, 166],
                [244, 193, 91],
                [88, 68, 30],
                false,
            ),
            Self::Porcelain => (
                [248, 249, 252],
                [228, 232, 242],
                [236, 238, 247],
                [166, 175, 202],
                [35, 41, 67],
                [83, 93, 123],
                [76, 67, 170],
                [220, 219, 248],
                true,
            ),
            Self::Sand => (
                [251, 245, 232],
                [235, 225, 207],
                [242, 232, 213],
                [189, 170, 140],
                [62, 45, 35],
                [112, 89, 67],
                [161, 66, 40],
                [248, 214, 186],
                true,
            ),
            Self::Aurora => (
                [20, 43, 48],
                [10, 27, 33],
                [32, 62, 65],
                [62, 100, 101],
                [227, 243, 235],
                [151, 189, 181],
                [190, 233, 100],
                [53, 84, 59],
                false,
            ),
            Self::Ember => (
                [46, 30, 40],
                [29, 19, 28],
                [67, 43, 53],
                [103, 70, 79],
                [250, 231, 226],
                [203, 166, 165],
                [255, 151, 123],
                [101, 53, 55],
                false,
            ),
        };
        Palette {
            panel: rgb(panel),
            canvas: rgb(canvas),
            raised: rgb(raised),
            border: rgb(border),
            text: rgb(text),
            muted: rgb(muted),
            accent: rgb(accent),
            selection: rgb(selection),
            light,
        }
    }
}
pub fn current() -> Theme {
    CURRENT.with(|t| t.get())
}
pub fn palette() -> Palette {
    current().palette()
}
pub fn accent() -> Color32 {
    palette().accent
}
pub fn muted() -> Color32 {
    if current() == Theme::Studio {
        crate::design::MUTED
    } else {
        palette().muted
    }
}
pub fn panel() -> Color32 {
    palette().panel
}
pub fn line() -> Color32 {
    palette().border
}
/// Adapt existing decorative UI shades; semantic layer colors stay distinct.
pub fn color(c: Color32) -> Color32 {
    if current() == Theme::Studio || c.a() == 0 {
        return c;
    }
    let p = palette();
    let [r, g, b, a] = c.to_array();
    let mapped = if [r, g, b] == [91, 158, 255] {
        p.accent
    } else if [[47, 73, 104], [48, 78, 118], [45, 64, 87]].contains(&[r, g, b]) {
        p.selection
    } else if r.max(g).max(b) - r.min(g).min(b) <= 24 {
        match r.max(g).max(b) {
            0..=35 => p.canvas,
            36..=50 => p.panel,
            51..=85 => p.raised,
            86..=185 => p.muted,
            _ => p.text,
        }
    } else {
        return c;
    };
    Color32::from_rgba_premultiplied(mapped.r(), mapped.g(), mapped.b(), a)
}
pub fn ensure(ctx: &egui::Context, theme: Theme) {
    CURRENT.with(|t| t.set(theme));
    let key = egui::Id::unique("libretoon-applied-theme");
    if ctx.data(|d| d.get_temp::<Theme>(key)) != Some(theme) {
        apply(ctx, theme);
        ctx.data_mut(|d| d.insert_temp(key, theme));
    }
}
pub fn apply(ctx: &egui::Context, theme: Theme) {
    crate::design::theme(ctx);
    CURRENT.with(|t| t.set(theme));
    if theme == Theme::Studio {
        return;
    }
    let p = theme.palette();
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    let mut v = if p.light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.extreme_bg_color = p.canvas;
    v.faint_bg_color = p.raised;
    v.override_text_color = Some(p.text);
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.selection;
    v.selection.stroke = Stroke::new(1., p.accent);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = 3.into();
        w.bg_stroke = Stroke::new(1., p.border);
        w.fg_stroke = Stroke::new(1., p.text);
    }
    v.widgets.inactive.bg_fill = p.raised;
    v.widgets.hovered.bg_fill = p.selection;
    v.widgets.active.bg_fill = p.selection;
    v.widgets.open.bg_fill = p.selection;
    v.widgets.noninteractive.bg_fill = p.panel;
    v.widgets.noninteractive.weak_bg_fill = p.panel;
    style.visuals = v;
    ctx.set_theme(if p.light {
        egui::Theme::Light
    } else {
        egui::Theme::Dark
    });
    ctx.set_style_of(
        if p.light {
            egui::Theme::Light
        } else {
            egui::Theme::Dark
        },
        style,
    );
}
pub fn selector(ui: &mut egui::Ui, selected: &mut Theme) -> bool {
    let before = *selected;
    ui.vertical(|ui| {
        ui.horizontal_top(|ui| {
            for (light, heading) in [(true, "Light themes"), (false, "Dark themes")] {
                ui.allocate_ui_with_layout(
                    egui::vec2(175., 0.),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        crate::design::category(ui, heading);
                        for theme in Theme::ALL
                            .into_iter()
                            .filter(|theme| theme.palette().light == light)
                        {
                            let p = theme.palette();
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(
                                    egui::vec2(42., 24.),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(r, 4., p.panel);
                                ui.painter().rect_stroke(
                                    r,
                                    4.,
                                    Stroke::new(1., p.border),
                                    egui::StrokeKind::Inside,
                                );
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        r.min + egui::vec2(4., 4.),
                                        egui::vec2(7., 16.),
                                    ),
                                    2.,
                                    p.accent,
                                );
                                for (y, width, color) in [(8., 22., p.text), (15., 16., p.muted)] {
                                    ui.painter().line_segment(
                                        [
                                            r.min + egui::vec2(16., y),
                                            r.min + egui::vec2(16. + width, y),
                                        ],
                                        Stroke::new(2., color),
                                    );
                                }
                                ui.selectable_value(selected, theme, crate::i18n::tr(theme.name()));
                            });
                        }
                    },
                );
                if light {
                    ui.add_space(10.);
                }
            }
        });
        ui.small(crate::i18n::tr(selected.description()));
    });
    *selected != before
}
