use crate::{Editor, Mark, canvas::Tool};
use eframe::egui::{self, Pos2, Rect, Vec2};

fn editor() -> Editor {
    let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
        egui::Context::default(),
    ));
    e.doc = crate::fresh_document();
    e.recovery = std::env::temp_dir().join(format!(
        "studio-v2-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ));
    crate::i18n::set_locale(crate::i18n::Language::English);
    e
}
fn button(pos: Pos2, pressed: bool, button: egui::PointerButton) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button,
        modifiers: egui::Modifiers::NONE,
    }
}
fn tick(
    ctx: &egui::Context,
    e: &mut Editor,
    events: Vec<egui::Event>,
    workspace: bool,
) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1366., 768.))),
            events,
            ..Default::default()
        },
        |ui| {
            if workspace {
                e.workspace(ui)
            } else {
                e.canvas_ui(ui)
            }
        },
    );
    out.textures_delta.clear();
    out
}
fn label(out: &egui::FullOutput, text: &str) -> Pos2 {
    out.shapes
        .iter()
        .find_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                (t.galley.job.text == text).then_some(t.pos + Vec2::splat(4.))
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing label {text}"))
}
#[test]
fn new_project_configuration_precedes_creation_and_cancel_preserves_document() {
    let mut e = editor();
    let ctx = egui::Context::default();
    let tabs = e.hub.tabs.len();
    let before = e.doc.layers.identity();
    e.new_document();
    let settings = e.document_settings.as_ref().unwrap();
    assert!(settings.creating);
    assert_eq!(
        (settings.width, settings.height, settings.fps),
        (1920, 1080, 24)
    );
    assert_eq!(e.hub.tabs.len(), tabs);
    tick(&ctx, &mut e, vec![], true);
    let out = tick(&ctx, &mut e, vec![], true);
    let p = label(&out, "Cancel");
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(p),
            button(p, true, egui::PointerButton::Primary),
        ],
        true,
    );
    tick(
        &ctx,
        &mut e,
        vec![button(p, false, egui::PointerButton::Primary)],
        true,
    );
    assert!(e.document_settings.is_none());
    assert_eq!(e.hub.tabs.len(), tabs);
    assert_eq!(e.doc.layers.identity(), before);
    e.new_document();
    tick(&ctx, &mut e, vec![], true);
    let out = tick(&ctx, &mut e, vec![], true);
    let p = label(&out, "Create");
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(p),
            button(p, true, egui::PointerButton::Primary),
        ],
        true,
    );
    tick(
        &ctx,
        &mut e,
        vec![button(p, false, egui::PointerButton::Primary)],
        true,
    );
    assert_eq!(e.hub.tabs.len(), tabs + 1);
    assert_eq!((e.doc.width, e.doc.height, e.doc.fps), (1920, 1080, 24));
}
#[test]
fn symbol_context_conversion_still_draws_the_stage_on_the_conversion_frame() {
    let mut e = editor();
    let ctx = egui::Context::default();
    e.doc.layers[0].frames[0].push(Mark {
        points: vec![[100., 100.], [300., 300.]].into(),
        rectangle: true,
        closed: true,
        fill: Some([120, 50, 220, 255]),
        stroke: false,
        ..Default::default()
    });
    e.tool = Tool::Select;
    e.canvas.selected = vec![(0, 0)];
    tick(&ctx, &mut e, vec![], false);
    let p = Pos2::new(500., 300.);
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(p),
            button(p, true, egui::PointerButton::Secondary),
        ],
        false,
    );
    tick(
        &ctx,
        &mut e,
        vec![button(p, false, egui::PointerButton::Secondary)],
        false,
    );
    let out = tick(&ctx, &mut e, vec![], false);
    let p = out
        .shapes
        .iter()
        .find_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                t.galley
                    .job
                    .text
                    .contains("F8")
                    .then_some(t.pos + Vec2::splat(4.))
            } else {
                None
            }
        })
        .expect("convert menu");
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(p),
            button(p, true, egui::PointerButton::Primary),
        ],
        false,
    );
    let out = tick(
        &ctx,
        &mut e,
        vec![button(p, false, egui::PointerButton::Primary)],
        false,
    );
    assert_eq!(e.doc.extras.symbols.len(), 1);
    assert!(
        out.shapes
            .iter()
            .any(|s| if let egui::Shape::Path(p) = &s.shape {
                p.fill == egui::Color32::WHITE && p.points.len() == 4
            } else {
                false
            }),
        "conversion frame must paint the white stage"
    );
}
#[test]
fn text_boxes_can_be_drawn_typed_reopened_and_saved() {
    let mut e = editor();
    let ctx = egui::Context::default();
    e.activate_tool(Tool::Text);
    tick(&ctx, &mut e, vec![], false);
    let a = Pos2::new(250., 200.);
    let b = Pos2::new(600., 330.);
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(a),
            button(a, true, egui::PointerButton::Primary),
        ],
        false,
    );
    tick(&ctx, &mut e, vec![egui::Event::PointerMoved(b)], false);
    tick(
        &ctx,
        &mut e,
        vec![button(b, false, egui::PointerButton::Primary)],
        false,
    );
    assert_eq!(e.doc.layers[0].frames[0].len(), 1);
    assert!(e.doc.validate().is_ok());
    tick(
        &ctx,
        &mut e,
        vec![egui::Event::Text("Editable text".into())],
        false,
    );
    let text = e.doc.layers[0].frames[0][0].style.text.as_ref().unwrap();
    assert_eq!(text.content, "Editable text");
    assert!(text.box_size.unwrap()[0] > 100.);
    let bounds = crate::typography::text_box_bounds(text).unwrap();
    let source = e.doc.layers[0].frames[0][0].nodes.identity();
    e.activate_tool(Tool::Select);
    e.activate_tool(Tool::Text);
    e.text_at(bounds.right_bottom() - Vec2::splat(2.));
    assert!(
        e.drawing.text.editing.is_some(),
        "whitespace inside the box must select text"
    );
    e.drawing.text.spec.size = 65.;
    e.commit_text();
    assert_ne!(e.doc.layers[0].frames[0][0].nodes.identity(), source);
    let copy = crate::fla::decode(&crate::fla::encode(&e.doc).unwrap()).unwrap();
    let text = copy.layers[0].frames[0][0].style.text.as_ref().unwrap();
    assert_eq!(text.content, "Editable text");
    assert_eq!(text.size, 65.);
    assert!(text.box_size.is_some());
}
#[test]
fn mannequin_rotates_in_3d_preserves_lengths_and_loads_legacy_data() {
    let mut m = crate::mannequin::Mannequin::default();
    let original = m.skeleton3d().0;
    m.rotation = [0.5, 1.2, 0.3];
    m.depth_angles[5] = 0.7;
    m.twists[4] = 0.8;
    let current = m.skeleton3d().0;
    let parents = [0, 0, 1, 2, 1, 4, 5, 1, 7, 8, 0, 10, 11, 0, 13, 14];
    let distance = |a: [f32; 3], b: [f32; 3]| {
        a.iter()
            .zip(b)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    };
    for i in 1..16 {
        assert!(
            (distance(original[i], original[parents[i]])
                - distance(current[i], current[parents[i]]))
            .abs()
                < 0.001
        );
    }
    assert!(current.iter().any(|p| p[2].abs() > 10.));
    let restored: crate::mannequin::Mannequin =
        serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_eq!(restored.rotation, m.rotation);
    assert!(restored.valid());
    let legacy: crate::mannequin::Mannequin = serde_json::from_str("{}").unwrap();
    assert!(legacy.valid());
    assert_eq!(legacy.rotation, [0.; 3]);
}
#[test]
fn camera_timeline_keys_ripple_frames_roll_and_split_without_gaps() {
    let mut e = editor();
    e.add_camera();
    e.add_camera();
    let id = e.doc.extras.cameras[0].id;
    let total = e
        .doc
        .extras
        .cameras
        .iter()
        .map(|c| c.duration)
        .sum::<usize>();
    e.camera_timeline_action(id, 8, 0);
    assert!(
        e.doc.extras.cameras[0]
            .animation
            .keys
            .iter()
            .any(|k| k.frame == 8)
    );
    e.camera_timeline_action(id, 8, 2);
    assert_eq!(e.doc.extras.cameras[0].duration, 25);
    assert!(
        e.doc.extras.cameras[0]
            .animation
            .keys
            .iter()
            .any(|k| k.frame == 9)
    );
    e.camera_timeline_action(id, 8, 3);
    assert_eq!(e.doc.extras.cameras[0].duration, 24);
    e.camera_timeline_action(id, 8, 1);
    assert!(
        !e.doc.extras.cameras[0]
            .animation
            .keys
            .iter()
            .any(|k| k.frame == 8)
    );
    e.roll_camera_boundary(0, 18);
    assert_eq!(e.doc.extras.cameras[0].duration, 18);
    assert_eq!(e.doc.extras.cameras[1].start, 18);
    assert_eq!(
        e.doc
            .extras
            .cameras
            .iter()
            .map(|c| c.duration)
            .sum::<usize>(),
        total
    );
    e.camera_timeline_action(id, 10, 4);
    assert_eq!(e.doc.extras.cameras.len(), 3);
    for pair in e.doc.extras.cameras.windows(2) {
        assert_eq!(pair[0].start + pair[0].duration, pair[1].start);
    }
    assert!(e.doc.validate().is_ok());
}

#[test]
fn camera_boundary_drag_transfers_frames_between_adjacent_cuts_and_undo_restores_them() {
    let mut e = editor();
    let ctx = egui::Context::default();
    e.hub.home = false;
    e.add_camera();
    e.add_camera();
    e.frame = 0;
    tick(&ctx, &mut e, vec![], true);
    let out = tick(&ctx, &mut e, vec![], true);
    let label = label(&out, "Camera 2");
    let a = label + Vec2::new(-12., 4.);
    let b = a - Vec2::new(40., 0.);
    tick(&ctx, &mut e, vec![egui::Event::PointerMoved(a)], true);
    tick(
        &ctx,
        &mut e,
        vec![button(a, true, egui::PointerButton::Primary)],
        true,
    );
    tick(&ctx, &mut e, vec![egui::Event::PointerMoved(b)], true);
    tick(
        &ctx,
        &mut e,
        vec![button(b, false, egui::PointerButton::Primary)],
        true,
    );
    assert!(
        e.doc.extras.cameras[0].duration < 24,
        "drag must trim the first camera"
    );
    assert_eq!(
        e.doc.extras.cameras[0].duration + e.doc.extras.cameras[1].duration,
        48
    );
    assert_eq!(
        e.doc.extras.cameras[0].duration,
        e.doc.extras.cameras[1].start
    );
    e.history(false);
    assert_eq!(e.doc.extras.cameras[0].duration, 24);
    assert_eq!(e.doc.extras.cameras[1].start, 24);
}
#[test]
fn text_wraps_to_box_width_and_empty_boxes_roundtrip() {
    let mut e = editor();
    e.ensure_text_fonts();
    let mut spec = e.drawing.text.spec.clone();
    spec.content = "alpha beta gamma delta".into();
    spec.box_size = Some([150., 300.]);
    spec.size = 40.;
    let narrow = crate::typography::geometry(&spec).unwrap();
    spec.box_size = Some([1000., 300.]);
    let wide = crate::typography::geometry(&spec).unwrap();
    assert!(
        crate::vector::control_bounds(&narrow).height()
            > crate::vector::control_bounds(&wide).height() * 1.5
    );
    spec.content.clear();
    e.doc.layers[0].frames[0].push(crate::typography::geometry(&spec).unwrap());
    let copy = crate::fla::decode(&crate::fla::encode(&e.doc).unwrap()).unwrap();
    assert!(
        copy.layers[0].frames[0][0]
            .style
            .text
            .as_ref()
            .unwrap()
            .box_size
            .is_some()
    );
    assert!(copy.validate().is_ok());
}
#[test]
#[ignore = "manual CPU render benchmark; no GPU or real input latency"]
fn benchmark_3d_mannequin_projection() {
    let mut e = editor();
    let ctx = egui::Context::default();
    e.add_mannequin(Pos2::new(960., 600.));
    e.tool = Tool::Mannequin;
    let start = std::time::Instant::now();
    for i in 0..100 {
        e.doc.layers[e.layer]
            .options
            .mannequin
            .as_mut()
            .unwrap()
            .rotation = [0.3, i as f32 * 0.03, 0.1];
        tick(&ctx, &mut e, vec![], false);
    }
    println!(
        "3D mannequin CPU shape generation: {:?} per frame (100 frames; no GPU)",
        start.elapsed() / 100
    );
}

#[test]
fn text_edit_checks_the_target_layer_and_3d_orbit_uses_real_pointer_input() {
    let mut e = editor();
    e.ensure_text_fonts();
    e.drawing.text.spec.content = "Text".into();
    e.drawing.text.spec.box_size = Some([400., 150.]);
    e.commit_text();
    e.add_mannequin(Pos2::new(1200., 580.));
    e.drawing.text.spec.size = 70.;
    e.commit_text();
    assert_eq!(
        e.doc.layers[0].frames[0][0]
            .style
            .text
            .as_ref()
            .unwrap()
            .size,
        70.
    );
    e.doc.layers[0].options.locked = true;
    e.drawing.text.spec.size = 80.;
    e.commit_text();
    assert_eq!(
        e.doc.layers[0].frames[0][0]
            .style
            .text
            .as_ref()
            .unwrap()
            .size,
        70.
    );
    e.tool = Tool::Mannequin;
    e.drawing.reference_rotate = true;
    let ctx = egui::Context::default();
    tick(&ctx, &mut e, vec![], false);
    let a = Pos2::new(800., 350.);
    let b = Pos2::new(900., 410.);
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(a),
            button(a, true, egui::PointerButton::Primary),
        ],
        false,
    );
    tick(&ctx, &mut e, vec![egui::Event::PointerMoved(b)], false);
    tick(
        &ctx,
        &mut e,
        vec![button(b, false, egui::PointerButton::Primary)],
        false,
    );
    let m = e.doc.layers[e.mannequin_layer().unwrap()]
        .options
        .mannequin
        .as_ref()
        .unwrap();
    assert!(m.rotation[0].abs() > 0.1 && m.rotation[1].abs() > 0.1);
    assert!(e.doc.validate().is_ok());
}
