use crate::{Editor, Mark, animation::Curve, canvas::Tool, features::FrameDrag};
use eframe::egui::{self, Pos2, Rect, Vec2};

fn editor() -> Editor {
    let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
        egui::Context::default(),
    ));
    e.doc = crate::fresh_document();
    e.doc.layers.truncate(1);
    e.doc.layers[0].name = "INK".into();
    e.recovery = std::env::temp_dir().join(format!(
        "interaction-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ));
    crate::i18n::set_locale(crate::i18n::Language::English);
    e
}
fn event(p: Pos2, down: bool, button: egui::PointerButton, shift: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: p,
        pressed: down,
        button,
        modifiers: egui::Modifiers {
            shift,
            ..Default::default()
        },
    }
}
fn tick(
    ctx: &egui::Context,
    e: &mut Editor,
    events: Vec<egui::Event>,
    timeline: bool,
    shift: bool,
) -> egui::FullOutput {
    let mut events = events;
    events.insert(
        0,
        egui::Event::ModifiersChanged(egui::Modifiers {
            shift,
            ..Default::default()
        }),
    );
    let size = if timeline {
        Vec2::new(1366., 768.)
    } else {
        Vec2::new(1000., 700.)
    };
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ui| {
            if timeline {
                e.sequencer(ui)
            } else {
                e.canvas_ui(ui)
            }
        },
    );
    out.textures_delta.clear();
    out
}
fn cell(out: &egui::FullOutput, frame: usize) -> Pos2 {
    let p = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "INK" => Some(t.pos),
            _ => None,
        })
        .expect("layer label");
    Pos2::new(p.x + 243. + 22. * (frame as f32 + 0.5), p.y + 6.)
}
fn tag(i: usize) -> Mark {
    Mark {
        points: vec![[i as f32, 10.], [i as f32 + 4., 20.]].into(),
        color: [i as u8, 20, 30, 255],
        ..Default::default()
    }
}
fn tags(e: &Editor) -> Vec<u8> {
    e.doc.layers[0]
        .frames
        .iter()
        .map(|f| f[0].color[0])
        .collect()
}

#[test]
fn click_picks_five_pixel_filled_brush_with_screen_margin_at_different_zooms() {
    for zoom in [0.4, 1., 2.] {
        let mut e = editor();
        let ctx = egui::Context::default();
        let axis = Mark {
            points: vec![[700., 450.], [1100., 450.]].into(),
            width: 5.,
            ..Default::default()
        };
        e.doc.layers[0].frames[0] = crate::vector::brush_fill(&axis, axis.color).into();
        e.tool = Tool::Select;
        e.canvas.zoom = zoom;
        tick(&ctx, &mut e, vec![], false, false);
        tick(&ctx, &mut e, vec![], false, false);
        let view = crate::canvas::View {
            center: Pos2::new(500., 350.) + e.canvas.pan,
            scale: e.canvas.actual_scale,
            angle: e.canvas.angle,
            stage_center: Pos2::new(e.doc.width as f32 / 2., e.doc.height as f32 / 2.),
        };
        let p = view.screen(Pos2::new(900., 452.5)) + Vec2::Y * 3.;
        tick(
            &ctx,
            &mut e,
            vec![
                egui::Event::PointerMoved(p),
                event(p, true, egui::PointerButton::Primary, false),
            ],
            false,
            false,
        );
        tick(
            &ctx,
            &mut e,
            vec![event(p, false, egui::PointerButton::Primary, false)],
            false,
            false,
        );
        assert!(!e.canvas.selected.is_empty(), "zoom={zoom}");
        assert_eq!(
            e.doc.layers[0].frames[0].len(),
            1,
            "picking must not cut the brush"
        );
    }
}

#[test]
fn shift_drag_retimes_keys_without_shifting_other_keys_and_undoes_once() {
    let mut e = editor();
    let ctx = egui::Context::default();
    e.doc.layers[0].frames = (0..24)
        .map(|i| {
            vec![tag(if i < 2 {
                0
            } else if i < 4 {
                2
            } else if i < 12 {
                4
            } else {
                12
            })]
            .into()
        })
        .collect();
    e.doc.layers[0].options.keys = (0..24).map(|i| [0, 2, 4, 12].contains(&i)).collect();
    tick(&ctx, &mut e, vec![], true, false);
    let out = tick(&ctx, &mut e, vec![], true, false);
    let a = cell(&out, 2);
    let b = cell(&out, 4);
    let grab = cell(&out, 3);
    let drop = cell(&out, 8);
    for (p, shift) in [(a, false), (b, true)] {
        tick(
            &ctx,
            &mut e,
            vec![
                egui::Event::PointerMoved(p),
                event(p, true, egui::PointerButton::Primary, shift),
            ],
            true,
            shift,
        );
        tick(
            &ctx,
            &mut e,
            vec![event(p, false, egui::PointerButton::Primary, shift)],
            true,
            shift,
        );
    }
    assert_eq!(e.feature.frame_selection, Some((0, 2, 4)));
    let undo = e.undo.len();
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(grab),
            event(grab, true, egui::PointerButton::Primary, false),
        ],
        true,
        false,
    );
    tick(
        &ctx,
        &mut e,
        vec![egui::Event::PointerMoved(drop)],
        true,
        false,
    );
    assert!(
        e.feature.frame_drag.is_some(),
        "drag should start on a selected cell"
    );
    tick(
        &ctx,
        &mut e,
        vec![event(drop, false, egui::PointerButton::Primary, false)],
        true,
        false,
    );
    assert_eq!(&tags(&e)[7..10], &[2, 2, 4]);
    assert_eq!(
        e.doc.layers[0].options.keys.iter().filter(|k| **k).count(),
        4
    );
    assert!(e.doc.layers[0].key(12));
    assert!(e.doc.layers[0].frames.iter().all(|f| !f.is_empty()));
    assert_eq!(e.undo.len(), undo + 1);
    e.history(false);
    assert!(e.doc.layers[0].key(2) && e.doc.layers[0].key(4));
}

#[test]
fn frame_move_preserves_motion_span_and_rejects_inverted_keys_or_locked_layers() {
    let mut e = editor();
    e.doc.layers[0].frames = (0..12)
        .map(|i| {
            vec![tag(if i < 4 {
                0
            } else if i < 10 {
                4
            } else {
                10
            })]
            .into()
        })
        .collect();
    e.doc.layers[0].options.keys = (0..12).map(|i| [0, 4, 10].contains(&i)).collect();
    e.doc.layers[0]
        .options
        .tweens
        .push(crate::animation::Tween {
            start: 0,
            end: 4,
            curve: Curve::default(),
        });
    let drag = FrameDrag {
        layer: 0,
        start: 4,
        end: 9,
        grab: 4,
        target: None,
    };
    assert!(!e.move_timeline_frames(drag, 10));
    e.doc.layers[0].options.locked = true;
    assert!(!e.move_timeline_frames(drag, 7));
    e.doc.layers[0].options.locked = false;
    assert!(e.move_timeline_frames(drag, 7));
    let span = &e.doc.layers[0].options.tweens[0];
    assert_eq!((span.start, span.end), (0, 7));
    assert!(e.doc.layers[0].key(10));
    assert!(e.doc.validate().is_ok());
}

#[test]
fn layer_name_context_menu_has_actions_and_rename_targets_that_layer() {
    let mut e = editor();
    let ctx = egui::Context::default();
    tick(&ctx, &mut e, vec![], true, false);
    let out = tick(&ctx, &mut e, vec![], true, false);
    let p = cell(&out, 0) - Vec2::X * 250.;
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(p),
            event(p, true, egui::PointerButton::Secondary, false),
        ],
        true,
        false,
    );
    let mut out = tick(
        &ctx,
        &mut e,
        vec![event(p, false, egui::PointerButton::Secondary, false)],
        true,
        false,
    );
    if !out
        .shapes
        .iter()
        .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text=="Rename layer"))
    {
        out = tick(&ctx, &mut e, vec![], true, false);
    }
    for label in ["Rename layer", "Delete layer", "Lock layer", "Hide layer"] {
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text==label)),
            "missing {label}"
        );
    }
    e.timeline_layer_action(0, 0);
    assert_eq!(e.renaming_layer, Some(0));
    e.timeline_layer_action(0, 1);
    assert_eq!(e.doc.layers.len(), 1, "keep the last layer");
}

#[test]
fn direct_timing_is_monotone_and_handles_control_start_and_end_speed() {
    let mut c = Curve::default();
    c.edit_timing();
    c.timing = Some(vec![[0.3, 0.65], [0.7, 0.85]]);
    let mut last = 0.;
    for i in 0..=1000 {
        let y = c.time(i as f32 / 1000.);
        assert!(y >= last - 1e-6 && y <= 1.);
        last = y;
    }
    c.pace = [0., 0.];
    let slow_start = c.time(0.001);
    let slow_end = 1. - c.time(0.999);
    c.pace = [0.8, 0.8];
    assert!(c.time(0.001) > slow_start * 10.);
    assert!(1. - c.time(0.999) > slow_end * 10.);
    assert_eq!(c.time(0.), 0.);
    assert_eq!(c.time(1.), 1.);
    let loaded: Curve = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(c, loaded);
    let legacy:Curve=serde_json::from_str(r#"{"pace":[0,0],"ease":[[0.33333334,0.33333334],[0.6666667,0.6666667]],"path":[[0,0],[0,0]]}"#).unwrap();
    assert!(legacy.timing.is_none());
    assert!(legacy.valid());
}

#[test]
fn dragging_timing_graph_shapes_the_actual_curve() {
    let ctx = egui::Context::default();
    let mut c = Curve::default();
    let mut handle = None;
    let mut run = |events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 400.))),
                events,
                ..Default::default()
            },
            |ui| crate::visual_controls::timing_controls(ui, &mut c, &mut handle, 24),
        );
        output.textures_delta.clear();
        output
    };
    run(vec![]);
    let out = run(vec![]);
    let p = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Path(path) if path.points.len() == 97 => Some(path.points[48]),
            _ => None,
        })
        .unwrap();
    run(vec![
        egui::Event::PointerMoved(p),
        event(p, true, egui::PointerButton::Primary, false),
    ]);
    let q = p - Vec2::Y * 30.;
    run(vec![egui::Event::PointerMoved(q)]);
    run(vec![event(q, false, egui::PointerButton::Primary, false)]);
    assert!(c.time(0.5) > 0.65);
    assert!(c.timing.as_ref().is_some_and(|p| !p.is_empty()));
}

#[test]
fn bending_path_at_any_interior_point_keeps_endpoints_and_timing() {
    for t in [0.1, 0.35, 0.7, 0.9] {
        let mut c = Curve::default();
        let a = Pos2::new(10., 20.);
        let b = Pos2::new(400., 20.);
        let original = c.point(a, b, t);
        let delta = Vec2::new(5., -70.);
        c.bend_path(t, delta, c.path);
        assert!(c.point(a, b, t).distance(original + delta) < 0.001);
        assert_eq!(c.point(a, b, 0.), a);
        assert_eq!(c.point(a, b, 1.), b);
        assert!((c.time(0.5) - 0.5).abs() < 0.001);
    }
}

#[test]
fn canvas_path_can_be_bent_directly_and_undone_without_changing_endpoint_artwork() {
    let mut e = editor();
    let ctx = egui::Context::default();
    let first = Mark {
        points: vec![[650., 450.], [690., 490.]].into(),
        fill: Some([220, 40, 60, 255]),
        stroke: false,
        rectangle: true,
        closed: true,
        ..Default::default()
    };
    let end = crate::canvas::transformed(&first, Pos2::ZERO, Vec2::splat(1.), 0., Vec2::X * 300.);
    for frame in &mut e.doc.layers[0].frames {
        *frame = vec![first.clone()].into();
    }
    e.doc.layers[0].frames[4] = vec![end.clone()].into();
    let count = e.doc.layers[0].frames.len();
    e.doc.layers[0].options.keys = vec![false; count].into();
    e.doc.layers[0].options.keys[0] = true;
    e.doc.layers[0].options.keys[4] = true;
    e.doc.layers[0]
        .options
        .tweens
        .push(crate::animation::Tween {
            start: 0,
            end: 4,
            curve: Curve::default(),
        });
    e.frame = 2;
    e.tool = Tool::Select;
    tick(&ctx, &mut e, vec![], false, false);
    e.canvas.selected = vec![(0, 0)];
    tick(&ctx, &mut e, vec![], false, false);
    let view = crate::canvas::View {
        center: Pos2::new(500., 350.) + e.canvas.pan,
        scale: e.canvas.actual_scale,
        angle: e.canvas.angle,
        stage_center: Pos2::new(e.doc.width as f32 / 2., e.doc.height as f32 / 2.),
    };
    let a = crate::vector::control_bounds(&first).center();
    let b = crate::vector::control_bounds(&end).center();
    let p = view.screen(a.lerp(b, 0.23));
    let q = p - Vec2::Y * 35.;
    let undo = e.undo.len();
    tick(
        &ctx,
        &mut e,
        vec![
            egui::Event::PointerMoved(p),
            event(p, true, egui::PointerButton::Primary, false),
        ],
        false,
        false,
    );
    tick(
        &ctx,
        &mut e,
        vec![egui::Event::PointerMoved(q)],
        false,
        false,
    );
    tick(
        &ctx,
        &mut e,
        vec![event(q, false, egui::PointerButton::Primary, false)],
        false,
        false,
    );
    let curve = &e.doc.layers[0].options.tweens[0].curve;
    assert!(curve.point(a, b, 0.23).y < a.y - 30.);
    assert_eq!(curve.point(a, b, 0.), a);
    assert_eq!(curve.point(a, b, 1.), b);
    assert_eq!(e.undo.len(), undo + 1);
    e.history(false);
    assert_eq!(
        e.doc.layers[0].options.tweens[0].curve.path,
        [[0., 0.], [0., 0.]]
    );
}

#[test]
fn retiming_last_key_extends_holds_without_blank_keys() {
    let mut e = editor();
    e.doc.layers[0].frames = (0..6)
        .map(|i| vec![tag(if i < 4 { 0 } else { 4 })].into())
        .collect();
    e.doc.layers[0].options.keys = vec![true, false, false, false, true, false].into();
    let other = e.doc.layers[0].clone();
    e.doc.layers.push(other);
    assert!(e.move_timeline_frames(
        FrameDrag {
            layer: 0,
            start: 4,
            end: 5,
            grab: 4,
            target: None
        },
        8
    ));
    assert_eq!(tags(&e), vec![0, 0, 0, 0, 0, 0, 0, 0, 4]);
    assert_eq!(
        e.doc.layers[0].options.keys.iter().filter(|k| **k).count(),
        2
    );
    assert_eq!(e.doc.layers[1].frames.len(), 9);
    assert!(!e.doc.layers[1].key(8));
    e.history(false);
    assert_eq!(tags(&e), vec![0, 0, 0, 0, 4, 4]);
}

#[test]
fn timing_velocity_is_continuous_at_a_shaped_point_near_a_flat_section() {
    let mut c = Curve::default();
    c.edit_timing();
    c.pace = [1., 1.];
    c.timing = Some(vec![[0.3, 0.03], [0.7, 0.9]]);
    for t in [0.3, 0.7] {
        let h = 0.0001;
        let left = (c.time(t) - c.time(t - h)) / h;
        let right = (c.time(t + h) - c.time(t)) / h;
        assert!((left - right).abs() < 0.015, "left={left} right={right}");
    }
}

#[test]
fn retiming_reloaded_exposure_preserves_keys() {
    let mut e = editor();
    e.doc.layers[0].frames = (0..12)
        .map(|i| {
            vec![tag(if i < 4 {
                0
            } else if i < 8 {
                4
            } else {
                8
            })]
            .into()
        })
        .collect();
    e.doc.layers[0].options.keys = (0..12).map(|i| [0, 4, 8].contains(&i)).collect();
    let json = serde_json::to_string(&e.doc.layers[0]).unwrap();
    e.doc.layers[0] = serde_json::from_str(&json).unwrap();
    assert!(e.move_timeline_frames(
        FrameDrag {
            layer: 0,
            start: 4,
            end: 7,
            grab: 4,
            target: None
        },
        6
    ));
    assert_eq!(tags(&e), vec![0, 0, 0, 0, 0, 0, 4, 4, 8, 8, 8, 8]);
    assert_eq!(e.feature.frame_selection, Some((0, 6, 6)));
    assert_eq!(
        e.doc.layers[0].options.keys.iter().filter(|k| **k).count(),
        3
    );
}

#[test]
fn selecting_translucent_art_keeps_paint_opaque_without_changing_art() {
    let mut e = editor();
    let mut m = tag(10);
    m.color = [120, 80, 40, 32];
    m.fill = Some([30, 60, 90, 64]);
    m.stroke = true;
    e.doc.layers[0].frames[0] = vec![m].into();
    e.sync_selected_style(&[(0, 0)]);
    assert_eq!(e.color.to_array(), [120, 80, 40, 255]);
    assert_eq!(e.fill_color.to_array(), [30, 60, 90, 255]);
    assert_eq!(e.doc.layers[0].frames[0][0].color[3], 32);
    assert_eq!(e.doc.layers[0].frames[0][0].fill.unwrap()[3], 64);
}

#[test]
fn moving_on_an_exposure_does_not_create_motion_or_keys() {
    let mut e = editor();
    e.doc.layers[0].frames = vec![vec![tag(0)].into(); 12].into();
    e.doc.layers[0].options.keys = vec![
        true, false, false, false, false, false, false, false, false, false, false, false,
    ]
    .into();
    e.frame = 5;
    e.autokey_motion(&[(0, 0)]);
    assert!(e.doc.layers[0].options.tweens.is_empty());
    assert!(!e.doc.layers[0].key(5));
}

#[test]
fn scene_manager_opens_below_scene_button_and_closes_outside() {
    let mut e = editor();
    let ctx = egui::Context::default();
    let mut run = |events| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| e.scene_dropdown(ui),
        );
        out.textures_delta.clear();
        out
    };
    run(vec![]);
    let out = run(vec![]);
    let pos = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text.contains('▾') => {
                Some(t.pos + t.galley.size() / 2.)
            }
            _ => None,
        })
        .unwrap();
    run(vec![
        egui::Event::PointerMoved(pos),
        event(pos, true, egui::PointerButton::Primary, false),
    ]);
    run(vec![event(pos, false, egui::PointerButton::Primary, false)]);
    let out = run(vec![]);
    let heading = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == crate::i18n::tr("Gestor de escenas") => {
                Some(t.pos)
            }
            _ => None,
        })
        .expect("scene dropdown should remain open");
    assert!(
        heading.x < 80. && heading.y > pos.y && heading.y < 100.,
        "dropdown should be anchored to scene button: {heading:?}"
    );
    let outside = Pos2::new(900., 650.);
    run(vec![
        egui::Event::PointerMoved(outside),
        event(outside, true, egui::PointerButton::Primary, false),
    ]);
    run(vec![event(
        outside,
        false,
        egui::PointerButton::Primary,
        false,
    )]);
    drop(run);
    assert!(!e.animation.scenes_open);
}

#[test]
fn folder_color_picker_remains_open_and_applies_color() {
    let mut e = editor();
    let folder_color = egui::Color32::from_rgb(19, 113, 207);
    e.doc.extras.library.folders.push(crate::library::Folder {
        id: crate::features::id(),
        name: "Folder test".into(),
        parent: None,
        color: folder_color.to_array(),
        collapsed: false,
    });
    let ctx = egui::Context::default();
    let mut run = |events| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| e.asset_library_ui(ui),
        );
        out.textures_delta.clear();
        out
    };
    run(vec![]);
    let out = run(vec![]);
    let pos = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "Folder test" => {
                Some(t.pos + t.galley.size() / 2.)
            }
            _ => None,
        })
        .unwrap();
    run(vec![
        egui::Event::PointerMoved(pos),
        event(pos, true, egui::PointerButton::Secondary, false),
    ]);
    run(vec![event(
        pos,
        false,
        egui::PointerButton::Secondary,
        false,
    )]);
    let out = run(vec![]);
    let square = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Rect(r)
                if r.rect.width() > 150.
                    && r.rect.height() > 150.
                    && r.rect.width() < 350.
                    && r.rect.height() < 350. =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .expect("color picker square");
    let q = square.min + square.size() * 0.4;
    run(vec![
        egui::Event::PointerMoved(q),
        event(q, true, egui::PointerButton::Primary, false),
    ]);
    run(vec![event(q, false, egui::PointerButton::Primary, false)]);
    let out = run(vec![]);
    assert!(out.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text==crate::i18n::tr("COLOR DE CARPETA"))),"editing must keep the color picker open");
    drop(run);
    assert_ne!(
        e.doc.extras.library.folders[0].color,
        folder_color.to_array()
    );
}

#[test]
fn scene_hand_drag_reorders_over_other_handles_and_undo_preserves_active_scene() {
    let mut e = editor();
    let scenes: Vec<_> = ["Alpha", "Beta", "Gamma"]
        .into_iter()
        .map(|name| crate::animation::scene_from(&e.doc, crate::features::id(), name.into()))
        .collect();
    e.doc.extras.scenes = scenes.into();
    e.doc.extras.active_scene = 0;
    let active = e.doc.extras.scenes[0].id;
    e.animation.scenes_open = true;
    let ctx = egui::Context::default();
    let mut run = |events| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| e.scene_dropdown(ui),
        );
        out.textures_delta.clear();
        out
    };
    run(vec![]);
    let out = run(vec![]);
    let hand = |number: &str| {
        out.shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == number => {
                    Some(t.pos + Vec2::new(-20., 7.))
                }
                _ => None,
            })
            .unwrap()
    };
    let first = hand("1.");
    let last = hand("3.") + Vec2::Y * 7.;
    run(vec![
        egui::Event::PointerMoved(first),
        event(first, true, egui::PointerButton::Primary, false),
    ]);
    run(vec![egui::Event::PointerMoved(last)]);
    run(vec![event(
        last,
        false,
        egui::PointerButton::Primary,
        false,
    )]);
    drop(run);
    assert_eq!(
        e.doc
            .extras
            .scenes
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Beta", "Gamma", "Alpha"]
    );
    assert_eq!(e.doc.extras.scenes[e.doc.extras.active_scene].id, active);
    assert!(e.animation.scene_drag.is_none());
    assert!(e.animation.scenes_open);
    e.history(false);
    assert_eq!(
        e.doc
            .extras
            .scenes
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Alpha", "Beta", "Gamma"]
    );
}

#[test]
fn ffmpeg_prompt_needs_consent_and_can_be_postponed_without_network() {
    let mut e = editor();
    let ctx = egui::Context::default();
    e.studio.ffmpeg_setup.prompt = true;
    let mut render = |events| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| e.export_ui(ui.ctx()),
        );
        out.textures_delta.clear();
        out
    };
    render(vec![]);
    let out = render(vec![]);
    assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text==crate::i18n::tr("This exporter needs FFmpeg to work. Download and configure it automatically?"))));
    let button = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == crate::i18n::tr("Not now") => {
                Some(t.pos + t.galley.size() / 2.)
            }
            _ => None,
        })
        .unwrap();
    render(vec![
        egui::Event::PointerMoved(button),
        event(button, true, egui::PointerButton::Primary, false),
    ]);
    render(vec![event(
        button,
        false,
        egui::PointerButton::Primary,
        false,
    )]);
    drop(render);
    assert!(!e.studio.ffmpeg_setup.prompt);
    assert!(e.studio.ffmpeg_setup.job.is_none());
}
