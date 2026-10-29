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
pub fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(12.);
    ui.label(egui::RichText::new(title).size(11.).color(MUTED).strong());
    ui.add_space(4.);
}

/// Original mouse mascot: asymmetric ears, long muzzle and blue pencil whisker.
pub fn logo(p: &egui::Painter, r: Rect) {
    let at = |x: f32, y: f32| r.min + Vec2::new(x / 64. * r.width(), y / 64. * r.height());
    let scale = r.width() / 64.;
    let cream = Color32::from_rgb(224, 228, 237);
    let pink = Color32::from_rgb(205, 132, 155);
    p.circle_filled(at(17., 16.), 12. * scale, cream);
    p.circle_filled(at(43., 12.), 10. * scale, cream);
    p.circle_filled(at(17., 16.), 7. * scale, pink);
    p.circle_filled(at(43., 12.), 5.5 * scale, pink);
    p.add(egui::Shape::convex_polygon(
        vec![
            at(13., 23.),
            at(39., 21.),
            at(53., 41.),
            at(40., 55.),
            at(18., 51.),
            at(10., 37.),
        ],
        cream,
        Stroke::NONE,
    ));
    p.circle_filled(at(28., 32.), 2.5 * scale, Color32::from_rgb(30, 32, 40));
    p.circle_filled(at(42., 31.), 2.5 * scale, Color32::from_rgb(30, 32, 40));
    p.circle_filled(at(45., 43.), 5. * scale, Color32::from_rgb(30, 32, 40));
    p.line_segment(
        [at(41., 48.), at(56., 54.)],
        Stroke::new(3. * scale, ACCENT),
    );
    p.line_segment(
        [at(40., 51.), at(53., 60.)],
        Stroke::new(2. * scale, ACCENT),
    );
}
pub fn window_icon() -> egui::IconData {
    let mut rgba = vec![0; 64 * 64 * 4];
    for y in 0..64 {
        for x in 0..64 {
            let ellipse = |cx: f32, cy: f32, rx: f32, ry: f32| {
                ((x as f32 - cx) / rx).powi(2) + ((y as f32 - cy) / ry).powi(2) <= 1.
            };
            let mut c = [0, 0, 0, 0];
            if ellipse(17., 16., 12., 12.)
                || ellipse(43., 12., 10., 10.)
                || ellipse(29., 35., 19., 21.)
                || ellipse(41., 42., 13., 12.)
            {
                c = [224, 228, 237, 255];
            }
            if ellipse(17., 16., 7., 7.) || ellipse(43., 12., 5.5, 5.5) {
                c = [205, 132, 155, 255];
            }
            if ellipse(28., 32., 2.5, 3.) || ellipse(42., 31., 2.5, 3.) || ellipse(45., 43., 5., 4.)
            {
                c = [30, 32, 40, 255];
            }
            if x >= 42
                && x <= 59
                && ((y as f32 - (48. + (x - 42) as f32 * 0.42)).abs() < 1.5
                    || (y as f32 - (52. + (x - 42) as f32 * 0.7)).abs() < 1.)
            {
                c = [91, 158, 255, 255];
            }
            rgba[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4].copy_from_slice(&c);
        }
    }
    egui::IconData {
        rgba,
        width: 64,
        height: 64,
    }
}
