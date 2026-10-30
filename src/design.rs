use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Vec2};
pub const ACCENT: Color32 = Color32::from_rgb(91, 158, 255);
pub const MUTED: Color32 = Color32::from_rgb(150, 154, 164);
pub const PANEL: Color32 = Color32::from_rgb(39, 40, 44);
pub const LINE: Color32 = Color32::from_rgb(61, 63, 69);
#[derive(Clone, Copy)]
pub enum Icon {
    New,
    Open,
    Save,
    SaveAs,
    Export,
    Undo,
    Redo,
    Brush,
    VectorBrush,
    Bucket,
    Width,
    Pen,
    Node,
    Rectangle,
    Trash,
    Plus,
    Copy,
    Play,
    Pause,
    First,
    Previous,
    Next,
    Onion,
    Eye,
    EyeOff,
    Up,
    Down,
    Layers,
    Film,
    Settings,
    Select,
    Transform,
    Rotate,
    Fit,
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

pub fn theme(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(25, 26, 29);
    v.faint_bg_color = Color32::from_rgb(45, 47, 52);
    v.override_text_color = Some(Color32::from_rgb(218, 220, 226));
    v.selection.bg_fill = Color32::from_rgb(48, 78, 118);
    v.selection.stroke = Stroke::new(1., ACCENT);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = 3.into();
        w.bg_stroke = Stroke::new(1., LINE);
    }
    v.widgets.inactive.bg_fill = Color32::from_rgb(48, 50, 55);
    v.widgets.hovered.bg_fill = Color32::from_rgb(65, 69, 78);
    v.widgets.active.bg_fill = Color32::from_rgb(48, 78, 118);
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.spacing.item_spacing = Vec2::new(6., 6.);
        s.spacing.button_padding = Vec2::new(8., 5.);
        s.spacing.slider_width = 135.;
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(12.));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(12.));
        s.text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(15.));
    });
}
/// Original 20-unit line icons, drawn as geometry so they stay sharp at any DPI.
pub fn draw(p: &egui::Painter, r: Rect, icon: Icon, c: Color32) {
    let origin = r.center() - Vec2::splat(10.);
    let pt = |x: f32, y: f32| origin + Vec2::new(x, y);
    let s = Stroke::new(1.5, c);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([pt(a.0, a.1), pt(b.0, b.1)], s);
    };
    let path = |coords: &[(f32, f32)]| {
        p.add(egui::Shape::line(
            coords.iter().map(|&(x, y)| pt(x, y)).collect(),
            s,
        ));
    };
    let rect = |x: f32, y: f32, w: f32, h: f32| {
        p.rect_stroke(
            Rect::from_min_size(pt(x, y), Vec2::new(w, h)),
            1.,
            s,
            egui::StrokeKind::Inside,
        );
    };
    match icon {
        Icon::Lasso => {
            path(&[
                (5., 15.),
                (2., 10.),
                (4., 4.),
                (12., 2.),
                (18., 6.),
                (17., 13.),
                (10., 16.),
                (5., 15.),
                (4., 19.),
            ]);
        }
        Icon::PolygonSelect => {
            path(&[(3., 16.), (5., 3.), (17., 6.), (13., 17.), (3., 16.)]);
            rect(2., 15., 2., 2.);
            rect(4., 2., 2., 2.);
            rect(16., 5., 2., 2.);
        }
        Icon::Wand => {
            line((3., 18.), (14., 7.));
            line((10., 8.), (13., 11.));
            line((15., 1.), (15., 4.));
            line((18., 5.), (20., 5.));
            line((9., 2.), (11., 4.));
        }
        Icon::Object => {
            rect(3., 3., 14., 14.);
            path(&[
                (7., 6.),
                (7., 14.),
                (10., 12.),
                (12., 16.),
                (14., 15.),
                (12., 11.),
                (16., 11.),
                (7., 6.),
            ]);
        }
        Icon::Eraser => {
            path(&[
                (2., 12.),
                (11., 3.),
                (18., 10.),
                (10., 18.),
                (8., 18.),
                (2., 12.),
            ]);
            line((6., 8.), (13., 15.));
            line((10., 18.), (19., 18.));
        }
        Icon::Eyedropper => {
            path(&[
                (3., 17.),
                (5., 12.),
                (14., 3.),
                (18., 7.),
                (9., 16.),
                (3., 17.),
            ]);
            line((11., 4.), (17., 10.));
        }
        Icon::Line => {
            line((3., 17.), (17., 3.));
        }
        Icon::Ellipse => {
            p.circle_stroke(pt(10., 10.), 7., s);
        }
        Icon::Polygon => {
            path(&[
                (10., 2.),
                (18., 8.),
                (15., 17.),
                (5., 17.),
                (2., 8.),
                (10., 2.),
            ]);
        }
        Icon::Pan => {
            path(&[
                (5., 11.),
                (5., 6.),
                (7., 6.),
                (7., 2.),
                (9., 2.),
                (9., 7.),
                (10., 7.),
                (10., 2.),
                (12., 2.),
                (12., 7.),
                (13., 7.),
                (13., 4.),
                (15., 4.),
                (15., 9.),
                (17., 7.),
                (19., 9.),
                (15., 17.),
                (9., 19.),
                (3., 13.),
                (3., 10.),
                (5., 11.),
            ]);
        }
        Icon::Zoom => {
            p.circle_stroke(pt(8., 8.), 5., s);
            line((12., 12.), (18., 18.));
            line((5., 8.), (11., 8.));
            line((8., 5.), (8., 11.));
        }
        Icon::New => {
            path(&[
                (5., 2.),
                (12., 2.),
                (16., 6.),
                (16., 18.),
                (5., 18.),
                (5., 2.),
            ]);
            path(&[(12., 2.), (12., 6.), (16., 6.)]);
            line((8., 11.), (13., 11.));
            line((10.5, 8.5), (10.5, 13.5));
        }
        Icon::Open => {
            path(&[(2., 16.), (2., 5.), (8., 5.), (10., 7.), (17., 7.)]);
            path(&[(2., 16.), (5., 9.), (19., 9.), (16., 16.), (2., 16.)]);
        }
        Icon::Save | Icon::SaveAs => {
            path(&[
                (3., 3.),
                (15., 3.),
                (17., 5.),
                (17., 17.),
                (3., 17.),
                (3., 3.),
            ]);
            rect(6., 3., 7., 5.);
            rect(6., 11., 8., 6.);
            if matches!(icon, Icon::SaveAs) {
                line((13., 15.), (19., 15.));
                line((16., 12.), (16., 18.));
            }
        }
        Icon::Export => {
            path(&[(3., 9.), (3., 17.), (17., 17.), (17., 9.)]);
            line((10., 13.), (10., 2.));
            path(&[(6., 6.), (10., 2.), (14., 6.)]);
        }
        Icon::Undo => {
            path(&[(7., 5.), (3., 9.), (7., 13.)]);
            path(&[(3., 9.), (12., 9.), (16., 11.), (16., 16.)]);
        }
        Icon::Redo => {
            path(&[(13., 5.), (17., 9.), (13., 13.)]);
            path(&[(17., 9.), (8., 9.), (4., 11.), (4., 16.)]);
        }
        Icon::Brush => {
            path(&[(7., 12.), (14., 3.), (17., 6.), (9., 14.)]);
            path(&[
                (7., 12.),
                (4., 13.),
                (3., 18.),
                (8., 17.),
                (9., 14.),
                (7., 12.),
            ]);
        }
        Icon::VectorBrush => {
            path(&[(10., 10.), (15., 3.), (18., 6.), (12., 13.)]);
            path(&[(10., 10.), (7., 12.), (6., 16.), (10., 15.), (12., 13.)]);
            p.add(egui::Shape::CubicBezier(
                egui::epaint::CubicBezierShape::from_points_stroke(
                    [pt(1., 18.), pt(10., 20.), pt(1., 8.), pt(7., 13.)],
                    false,
                    Color32::TRANSPARENT,
                    s,
                ),
            ));
            rect(0., 16., 3., 3.);
            rect(5., 11., 3., 3.);
        }
        Icon::Bucket => {
            path(&[(6., 3.), (15., 11.), (8., 18.), (1., 11.), (6., 3.)]);
            line((2., 10.), (13., 10.));
            path(&[(6., 3.), (10., 1.), (14., 5.)]);
            p.circle_filled(pt(17., 17.), 2., c);
            path(&[(16., 13.), (18., 16.)]);
        }
        Icon::Width => {
            path(&[
                (2., 10.),
                (5., 7.),
                (10., 4.),
                (15., 7.),
                (18., 10.),
                (15., 13.),
                (10., 16.),
                (5., 13.),
                (2., 10.),
            ]);
            line((1., 10.), (19., 10.));
            line((10., 1.), (10., 19.));
            path(&[(7., 3.), (10., 1.), (13., 3.)]);
            path(&[(7., 17.), (10., 19.), (13., 17.)]);
        }
        Icon::Pen => {
            path(&[
                (10., 2.),
                (17., 11.),
                (12., 17.),
                (8., 17.),
                (3., 11.),
                (10., 2.),
            ]);
            line((10., 2.), (10., 10.));
            p.circle_stroke(pt(10., 11.), 2., s);
            line((6., 19.), (14., 19.));
        }
        Icon::Node => {
            line((3., 16.), (17., 4.));
            p.circle_stroke(pt(3., 16.), 2., s);
            p.circle_stroke(pt(17., 4.), 2., s);
            p.rect_filled(Rect::from_center_size(pt(10., 10.), Vec2::splat(6.)), 0., c);
        }
        Icon::Rectangle => rect(3., 4., 14., 12.),
        Icon::Trash => {
            line((3., 5.), (17., 5.));
            path(&[(7., 5.), (7., 2.), (13., 2.), (13., 5.)]);
            path(&[(5., 5.), (6., 18.), (14., 18.), (15., 5.)]);
            line((8., 8.), (8., 15.));
            line((12., 8.), (12., 15.));
        }
        Icon::Plus => {
            line((4., 10.), (16., 10.));
            line((10., 4.), (10., 16.));
        }
        Icon::Copy => {
            rect(6., 6., 11., 12.);
            path(&[(13., 4.), (13., 2.), (2., 2.), (2., 14.), (4., 14.)]);
        }
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![pt(6., 3.), pt(17., 10.), pt(6., 17.)],
                c,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            p.rect_filled(Rect::from_min_size(pt(5., 4.), Vec2::new(3., 12.)), 0., c);
            p.rect_filled(Rect::from_min_size(pt(12., 4.), Vec2::new(3., 12.)), 0., c);
        }
        Icon::Previous | Icon::Next | Icon::First => {
            let flip = matches!(icon, Icon::Next);
            let x = |v: f32| if flip { 20. - v } else { v };
            path(&[(x(13.), 4.), (x(6.), 10.), (x(13.), 16.)]);
            if matches!(icon, Icon::First) {
                line((3., 4.), (3., 16.));
            }
        }
        Icon::Onion => {
            p.circle_stroke(pt(7., 10.), 5., s);
            p.circle_stroke(pt(13., 10.), 5., s);
        }
        Icon::Eye | Icon::EyeOff => {
            path(&[
                (1., 10.),
                (5., 6.),
                (10., 4.),
                (15., 6.),
                (19., 10.),
                (15., 14.),
                (10., 16.),
                (5., 14.),
                (1., 10.),
            ]);
            p.circle_stroke(pt(10., 10.), 2.5, s);
            if matches!(icon, Icon::EyeOff) {
                line((3., 2.), (17., 18.));
            }
        }
        Icon::Up => {
            path(&[(4., 12.), (10., 6.), (16., 12.)]);
        }
        Icon::Down => {
            path(&[(4., 8.), (10., 14.), (16., 8.)]);
        }
        Icon::Layers => {
            path(&[(2., 7.), (10., 3.), (18., 7.), (10., 11.), (2., 7.)]);
            path(&[(2., 11.), (10., 15.), (18., 11.)]);
            path(&[(2., 15.), (10., 19.), (18., 15.)]);
        }
        Icon::Film => {
            rect(2., 3., 16., 14.);
            line((6., 3.), (6., 17.));
            line((14., 3.), (14., 17.));
            for y in [7., 12.] {
                line((2., y), (6., y));
                line((14., y), (18., y));
            }
        }
        Icon::Select => {
            path(&[
                (4., 2.),
                (4., 17.),
                (8., 13.),
                (11., 19.),
                (14., 17.),
                (11., 12.),
                (17., 12.),
                (4., 2.),
            ]);
        }
        Icon::Transform => {
            rect(4., 4., 12., 12.);
            for (x, y) in [(2., 2.), (14., 2.), (2., 14.), (14., 14.)] {
                p.rect_filled(Rect::from_min_size(pt(x, y), Vec2::splat(4.)), 0., c);
            }
        }
        Icon::Rotate => {
            path(&[
                (16., 7.),
                (13., 3.),
                (7., 3.),
                (3., 7.),
                (3., 13.),
                (7., 17.),
                (13., 17.),
                (17., 13.),
            ]);
            path(&[(16., 2.), (16., 7.), (11., 7.)]);
        }
        Icon::Fit => {
            path(&[(2., 7.), (2., 2.), (7., 2.)]);
            path(&[(13., 2.), (18., 2.), (18., 7.)]);
            path(&[(18., 13.), (18., 18.), (13., 18.)]);
            path(&[(7., 18.), (2., 18.), (2., 13.)]);
            rect(6., 6., 8., 8.);
        }
        Icon::Settings => {
            for (y, x) in [(5., 7.), (10., 13.), (15., 8.)] {
                line((2., y), (18., y));
                p.circle_filled(pt(x, y), 2.5, PANEL);
                p.circle_stroke(pt(x, y), 2.5, s);
            }
        }
    }
}
pub fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    tip: &str,
    selected: bool,
    enabled: bool,
) -> Response {
    let text = ui.painter().layout_no_wrap(
        label.into(),
        egui::FontId::proportional(12.),
        Color32::WHITE,
    );
    let width = if label.is_empty() {
        32.
    } else {
        40. + text.size().x
    };
    let (r, response) = ui.allocate_exact_size(
        Vec2::new(width, 30.),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let fill = if selected {
        Color32::from_rgb(47, 73, 104)
    } else if response.hovered() && enabled {
        Color32::from_rgb(61, 64, 72)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(r, 3., fill);
    if selected {
        ui.painter()
            .rect_stroke(r, 3., Stroke::new(1., ACCENT), egui::StrokeKind::Inside);
    }
    let color = if !enabled {
        Color32::from_rgb(87, 90, 99)
    } else if selected {
        ACCENT
    } else {
        Color32::from_rgb(203, 207, 216)
    };
    draw(
        ui.painter(),
        Rect::from_center_size(Pos2::new(r.left() + 16., r.center().y), Vec2::splat(20.)),
        icon,
        color,
    );
    if !label.is_empty() {
        ui.painter().text(
            Pos2::new(r.left() + 32., r.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.),
            color,
        );
    }
    response.on_hover_text(tip)
}
/// The supplied app artwork, uploaded once and reused by the header.
pub fn logo(p: &egui::Painter, r: Rect) {
    let id = egui::Id::unique("libretoon-app-icon");
    let cached = p
        .ctx()
        .data_mut(|data| data.get_temp::<egui::TextureHandle>(id));
    // Loading a texture reads Context input; never do this while holding data_mut.
    let texture = cached.unwrap_or_else(|| {
        let icon = window_icon();
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [icon.width as usize, icon.height as usize],
            &icon.rgba,
        );
        let texture = p
            .ctx()
            .load_texture("LibreToon", image, egui::TextureOptions::LINEAR);
        p.ctx()
            .data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    });
    p.image(
        texture.id(),
        r,
        Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
        Color32::WHITE,
    );
}
pub fn window_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/libretoon-icon.png"))
        .expect("the bundled LibreToon icon must be a valid PNG")
}

#[cfg(test)]
mod icon_tests {
    #[test]
    fn header_icon_loads_and_reuses_texture_across_frames() {
        let ctx = eframe::egui::Context::default();
        for _ in 0..2 {
            let mut output = ctx.run_ui(Default::default(), |ui| {
                eframe::egui::Frame::NONE.show(ui, |ui| {
                    super::logo(
                        ui.painter(),
                        eframe::egui::Rect::from_min_size(
                            eframe::egui::Pos2::ZERO,
                            eframe::egui::vec2(32., 32.),
                        ),
                    );
                });
            });
            output.textures_delta.clear();
        }
    }
}
