use crate::{Editor, Mark, canvas::Tool};
use eframe::egui::{self, Pos2, Rect, Vec2};

fn editor() -> Editor {
    let ctx = egui::Context::default();
    let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx));
    e.recovery = std::env::temp_dir().join(format!(
        "studio-test-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ));
    e.doc = crate::fresh_document();
    e
}
fn ink() -> Mark {
    Mark {
        points: vec![[0., 0.], [60., 60.]].into(),
        rectangle: true,
        closed: true,
        fill: Some([10, 80, 200, 255]),
        stroke: false,
        ..Default::default()
    }
}

#[test]
fn document_switch_preserves_art_history_view_and_symbol_context() {
    let mut e = editor();
    e.add_document(crate::fresh_document(), None);
    e.checkpoint();
    e.doc.layers[0].frames[0].push(ink());
    e.canvas.pan = Vec2::new(90., 40.);
    e.canvas.zoom = 2.;
    let id = e.doc.layers.identity();
    let points = e.doc.layers[0].frames[0][0].points.identity();
    e.add_document(crate::fresh_document(), None);
    e.checkpoint();
    e.doc.layers[0].frames[0].push(Mark {
        color: [255, 0, 0, 255],
        ..ink()
    });
    e.activate_document(0);
    assert_eq!(e.doc.layers.identity(), id);
    assert_eq!(e.doc.layers[0].frames[0][0].points.identity(), points);
    assert_eq!(e.canvas.pan, Vec2::new(90., 40.));
    assert_eq!(e.canvas.zoom, 2.);
    e.history(false);
    assert!(e.doc.layers[0].frames[0].is_empty());
    e.activate_document(1);
    assert_eq!(e.doc.layers[0].frames[0][0].color, [255, 0, 0, 255]);
    e.canvas.selected = vec![(0, 0)];
    e.convert_symbol();
    let symbol = e.doc.extras.symbols[0].id;
    e.enter_symbol(symbol);
    e.activate_document(0);
    e.activate_document(1);
    assert_eq!(e.feature.symbol_stack.len(), 1);
    assert_eq!(e.project_document().extras.symbols[0].id, symbol);
}

#[test]
fn startup_always_uses_home_and_many_documents_are_scrollable() {
    let mut e = editor();
    assert!(e.hub.home);
    for _ in 0..30 {
        e.add_document(crate::fresh_document(), None);
    }
    let ctx = egui::Context::default();
    let mut width = 0.;
    for _ in 0..2 {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 740.))),
                ..Default::default()
            },
            |ui| {
                e.document_tabs(ui);
                width = ui.clip_rect().width();
            },
        );
        out.textures_delta.clear();
    }
    assert_eq!(e.hub.tabs.len(), 30);
    assert!(width <= 1100.);
    e.activate_document(0);
    assert!(!e.hub.home);
    e.hub.home = true;
    assert_eq!(e.hub.tabs.len(), 30);
}

#[test]
fn rounded_rectangle_keeps_real_editable_curves_and_roundtrips() {
    let mut d = crate::fresh_document();
    let m = crate::drawing::rounded_rectangle(Pos2::ZERO, Pos2::new(120., 80.), 15., &ink());
    assert_eq!(m.nodes.len(), 8);
    assert!(!m.rectangle);
    assert!(!crate::vector::contains_fill(&m, Pos2::new(1., 1.)));
    assert!(crate::vector::contains_fill(&m, Pos2::new(60., 40.)));
    d.layers[0].frames[0].push(m.clone());
    let copy = crate::fla::decode(&crate::fla::encode(&d).unwrap()).unwrap();
    assert_eq!(copy.layers[0].frames[0][0].nodes, m.nodes);
    assert!(copy.svg(0).contains(" C"));
}

#[test]
fn sketch_and_mannequin_are_saved_but_never_exported_even_in_symbols() {
    let mut e = editor();
    let mut mark = ink();
    mark.style.guide = true;
    e.doc.layers[0].frames[0].push(mark);
    assert_eq!(crate::features::resolve(&e.doc, 0, false).len(), 1);
    assert!(crate::animation::resolve_movie(&e.doc, 0, true).is_empty());
    e.add_mannequin(Pos2::new(300., 250.));
    e.add_mannequin(Pos2::ZERO);
    assert_eq!(
        e.doc
            .layers
            .iter()
            .filter(|l| l.options.mannequin.is_some())
            .count(),
        1
    );
    let copy = crate::fla::decode(&crate::fla::encode(&e.project_document()).unwrap()).unwrap();
    assert!(copy.layers[0].frames[0][0].style.guide);
    assert!(copy.layers[1].options.mannequin.is_some());
    assert!(!copy.svg(0).contains("rgb(10,80,200)"));
    let baked = crate::features::baked(&copy);
    assert!(baked.layers[0].frames[0].is_empty());
    e.layer = 0;
    e.canvas.selected = vec![(0, 0)];
    e.convert_symbol();
    assert!(crate::animation::resolve_movie(&e.doc, 0, false).is_empty());
}

#[test]
fn airbrush_soft_edge_and_graphite_grain_are_deterministic() {
    let center = Mark {
        points: vec![[10., 20.], [80., 20.]].into(),
        width: 20.,
        color: [50, 100, 200, 255],
        ..Default::default()
    };
    let spray = crate::drawing::airbrush(&center, 0.5, 0.2, false);
    assert!(spray.len() > 4);
    let g = spray[0].style.gradient.as_ref().unwrap();
    assert!(g.radial);
    assert_eq!(g.stops.last().unwrap().color[3], 0);
    assert!(g.stops[0].color[3] > 0);
    let a = crate::drawing::airbrush(&center, 0.5, 0.2, true);
    let b = crate::drawing::airbrush(&center, 0.5, 0.2, true);
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    assert!(a.iter().all(|m| m.style.guide));
}

#[test]
fn mannequin_pose_keeps_lengths_when_joint_rotates_and_all_presets_validate() {
    let mut m = crate::mannequin::Mannequin::default();
    let (a, _) = m.pose();
    m.angles[5] += 0.9;
    let (b, _) = m.pose();
    assert!((a[5].distance(a[4]) - b[5].distance(b[4])).abs() < 0.001);
    assert!((a[6].distance(a[5]) - b[6].distance(b[5])).abs() < 0.001);
    assert!(a[6].distance(b[6]) > 10.);
    assert_eq!(a[8], b[8]);
    for heads in [3, 4, 6, 7, 8] {
        m.heads = heads;
        assert!(m.valid());
        assert!(m.pose().0.iter().all(|p| p.is_finite()));
    }
}

#[test]
fn editable_text_preserves_content_spacing_gradient_and_transform_without_font_on_reload() {
    let fonts = crate::typography::system_fonts();
    let (_, font) = fonts.first().expect("Test machine needs a TTF/OTF font");
    let mut s = crate::typography::TextSpec {
        content: "BOO\nTexto".into(),
        font: font.to_string_lossy().into_owned(),
        outline: true,
        letter_spacing: 3.,
        word_spacing: 7.,
        ..Default::default()
    };
    s.gradient = Some(Default::default());
    let m = crate::typography::geometry(&s).unwrap();
    let bounds = crate::vector::control_bounds(&m);
    assert!(bounds.width() > 50. && bounds.height() > 50.);
    let shifted = crate::vector::transform(&m, |p| p + Vec2::new(70., 50.));
    assert!((shifted.style.text.as_ref().unwrap().matrix[4] - s.matrix[4] - 70.).abs() < 0.01);
    let mut doc = crate::fresh_document();
    doc.layers[0].frames[0].push(shifted);
    let copy = crate::fla::decode(&crate::fla::encode(&doc).unwrap()).unwrap();
    let text = copy.layers[0].frames[0][0].style.text.as_ref().unwrap();
    assert_eq!(text.content, s.content);
    assert_eq!(text.word_spacing, 7.);
    assert!(copy.svg(0).contains("linearGradient"));
    let mut portable = copy.clone();
    portable.layers[0].frames[0][0]
        .style
        .text
        .as_mut()
        .unwrap()
        .font = "missing-font.ttf".into();
    assert!(portable.validate().is_ok());
    assert!(portable.svg(0).contains("<path"));
    let output = crate::features::resolve_export(&portable, 0, false);
    assert_eq!(output.len(), 1);
}

#[test]
fn locked_reference_layer_cannot_receive_text_or_paste() {
    let mut e = editor();
    e.add_mannequin(Pos2::ZERO);
    let before = serde_json::to_value(&e.doc).unwrap();
    e.studio.clipboard = vec![ink()];
    e.paste_selection();
    e.commit_text();
    assert_eq!(serde_json::to_value(&e.doc).unwrap(), before);
}

#[test]
fn tool_properties_and_reference_canvas_render_without_panics() {
    let mut e = editor();
    let ctx = egui::Context::default();
    for tool in [
        Tool::Text,
        Tool::Rectangle,
        Tool::Airbrush,
        Tool::SketchBrush,
        Tool::Mannequin,
    ] {
        e.tool = tool;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1400., 900.))),
                ..Default::default()
            },
            |ui| {
                egui::Panel::right("property-test")
                    .exact_size(305.)
                    .show(ui, |ui| e.properties_ui(ui));
                egui::CentralPanel::default().show(ui, |ui| e.canvas_ui(ui));
            },
        );
        assert!(!out.shapes.is_empty());
        out.textures_delta.clear();
    }
}

#[test]
fn real_pressure_changes_the_saved_width_profile_and_mouse_stays_constant() {
    let mut d = crate::drawing::Drawing::default();
    d.pen.test_pressure(None);
    d.start(Pos2::ZERO);
    d.sample(Pos2::new(40., 0.));
    assert!(d.profile(&[]).is_empty());
    d.pen.test_pressure(Some(0.2));
    d.start(Pos2::ZERO);
    d.pen.test_pressure(Some(0.8));
    d.sample(Pos2::new(40., 0.));
    // Releasing the pen must not erase pressure collected during the stroke.
    d.pen.test_pressure(None);
    let profile = d.profile(&[]);
    assert!((profile[0].width - 0.2).abs() < 0.001);
    assert!((profile[127].width - 0.8).abs() < 0.001);
    d.pressure = false;
    assert!(d.profile(&[]).is_empty());
    let mark = Mark {
        points: vec![[0., 0.], [40., 0.]].into(),
        width_profile: profile,
        width: 20.,
        ..Default::default()
    };
    let area = crate::width::outline(&mark);
    assert!(!area.0.is_empty());
    let mut doc = crate::fresh_document();
    doc.layers[0].frames[0].push(mark);
    let restored = crate::fla::decode(&crate::fla::encode(&doc).unwrap()).unwrap();
    assert!((restored.layers[0].frames[0][0].width_profile[127].width - 0.8).abs() < 0.001);
}

#[test]
#[ignore = "manual UI review; writes PPM images to LIBRETOON_PREVIEWS"]
fn render_studio_review_images() {
    use std::{collections::HashMap, io::Write};
    let folder = std::env::var_os("LIBRETOON_PREVIEWS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("libretoon-previews"));
    std::fs::create_dir_all(&folder).unwrap();
    let mut e = editor();
    let ctx = egui::Context::default();
    crate::design::theme(&ctx);
    e.add_document(crate::fresh_document(), None);
    e.doc.layers[0].frames[0].push(crate::drawing::rounded_rectangle(
        Pos2::new(80., 80.),
        Pos2::new(380., 260.),
        35.,
        &ink(),
    ));
    let fonts = crate::typography::system_fonts();
    if let Some((_, font)) = fonts
        .iter()
        .find(|(n, _)| n.to_lowercase().contains("sans"))
        .or(fonts.first())
    {
        e.drawing.text.spec.font = font.to_string_lossy().into_owned();
        e.drawing.text.spec.content = "Hello, LibreToon".into();
        e.drawing.text.spec.matrix[4] = 95.;
        e.drawing.text.spec.matrix[5] = 320.;
        e.drawing.text.spec.size = 38.;
        e.commit_text();
    }
    e.add_mannequin(Pos2::new(650., 280.));
    e.tool = Tool::Mannequin;
    let mut textures: HashMap<egui::TextureId, egui::ColorImage> = HashMap::new();
    for (name, home, tool) in [
        ("home", true, Tool::Select),
        ("mannequin", false, Tool::Mannequin),
        ("text", false, Tool::Text),
        ("airbrush", false, Tool::Airbrush),
    ] {
        e.hub.home = home;
        e.tool = tool;
        e.canvas.clear_selection();
        for frame in 0..3 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440., 960.))),
                    ..Default::default()
                },
                |ui| {
                    e.workspace(ui);
                    if !e.hub.home {
                        egui::CentralPanel::default()
                            .frame(
                                egui::Frame::new()
                                    .fill(egui::Color32::from_gray(29))
                                    .inner_margin(0.),
                            )
                            .show(ui, |ui| e.canvas_ui(ui));
                    }
                },
            );
            for (id, deltas) in &output.textures_delta.set {
                for delta in deltas {
                    let egui::ImageData::Color(image) = &delta.image;
                    if let Some([x, y]) = delta.pos {
                        if let Some(old) = textures.get_mut(id) {
                            for row in 0..image.size[1] {
                                let start = (y + row) * old.size[0] + x;
                                old.pixels[start..start + image.size[0]].copy_from_slice(
                                    &image.pixels[row * image.size[0]..(row + 1) * image.size[0]],
                                );
                            }
                        }
                    } else {
                        textures.insert(*id, (**image).clone());
                    }
                }
            }
            output.textures_delta.clear();
            if frame != 2 {
                continue;
            }
            let mut rgb = vec![24u8; 1440 * 960 * 3];
            for primitive in ctx.tessellate(output.shapes, output.pixels_per_point) {
                let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive else {
                    continue;
                };
                let texture = textures.get(&mesh.texture_id);
                for indices in mesh.indices.chunks_exact(3) {
                    let v = [
                        mesh.vertices[indices[0] as usize],
                        mesh.vertices[indices[1] as usize],
                        mesh.vertices[indices[2] as usize],
                    ];
                    let [a, b, c] = v.map(|v| v.pos);
                    let cross = |u: Vec2, v: Vec2| u.x * v.y - u.y * v.x;
                    let det = cross(b - a, c - a);
                    if det.abs() < 0.00001 {
                        continue;
                    }
                    let bounds = Rect::from_min_max(a.min(b).min(c), a.max(b).max(c))
                        .intersect(primitive.clip_rect);
                    for y in (bounds.top().floor().max(0.) as usize)
                        ..(bounds.bottom().ceil().max(0.) as usize).min(960)
                    {
                        for x in (bounds.left().floor().max(0.) as usize)
                            ..(bounds.right().ceil().max(0.) as usize).min(1440)
                        {
                            let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                            let w = [
                                cross(b - p, c - p) / det,
                                cross(c - p, a - p) / det,
                                cross(a - p, b - p) / det,
                            ];
                            if w.iter().any(|v| *v < 0.) {
                                continue;
                            }
                            let color: [f32; 4] = std::array::from_fn(|ch| {
                                (0..3)
                                    .map(|i| v[i].color.to_array()[ch] as f32 * w[i] / 255.)
                                    .sum()
                            });
                            let uv = Pos2::ZERO
                                + v[0].uv.to_vec2() * w[0]
                                + v[1].uv.to_vec2() * w[1]
                                + v[2].uv.to_vec2() * w[2];
                            let sample = texture
                                .map(|t| {
                                    let x = ((uv.x * t.size[0] as f32) as usize).min(t.size[0] - 1);
                                    let y = ((uv.y * t.size[1] as f32) as usize).min(t.size[1] - 1);
                                    t.pixels[y * t.size[0] + x].to_array()
                                })
                                .unwrap_or([255; 4]);
                            let alpha = color[3] * sample[3] as f32 / 255.;
                            let at = (y * 1440 + x) * 3;
                            for ch in 0..3 {
                                rgb[at + ch] = (color[ch] * sample[ch] as f32
                                    + rgb[at + ch] as f32 * (1. - alpha))
                                    .clamp(0., 255.)
                                    as u8;
                            }
                        }
                    }
                }
            }
            let mut file = std::fs::File::create(folder.join(format!("{name}.ppm"))).unwrap();
            write!(file, "P6\n1440 960\n255\n").unwrap();
            file.write_all(&rgb).unwrap();
        }
    }
}
