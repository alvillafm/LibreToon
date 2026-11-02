use crate::{Editor, animation::*, document::Mark, features::*};
use eframe::egui::{self, Pos2, Vec2};
fn rect() -> Mark {
    Mark {
        points: vec![[10., 10.], [50., 30.]].into(),
        rectangle: true,
        closed: true,
        stroke: false,
        fill: Some([200, 50, 100, 255]),
        ..Default::default()
    }
}
fn editor() -> Editor {
    let ctx = egui::Context::default();
    let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx));
    e.doc = crate::fresh_document();
    e.recovery = std::env::temp_dir().join(format!(
        "animation-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ));
    e
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.01, "{a} != {b}");
}
#[test]
fn motion_interpolates_translation_rotation_scale_and_symbol_frame() {
    let mut a = rect();
    a.style.instance = Some(Instance {
        clock: Default::default(),
        symbol: 1,
        matrix: [1., 0., 0., 1., 0., 0.],
        mode: Playback::Single,
        first: 0,
        born: 0,
    });
    let center = crate::vector::control_bounds(&a).center();
    let mut b = crate::vector::transform(&a, |p| {
        center
            + Vec2::new(100., 60.)
            + crate::canvas::rotate(
                (p - center) * Vec2::new(2., 3.),
                std::f32::consts::FRAC_PI_2,
            )
    });
    b.style.instance.as_mut().unwrap().first = 8;
    let mid = interpolate(&a, &b, 0.5, &Curve::default());
    let c = crate::vector::control_bounds(&mid).center();
    close(c.x, center.x + 50.);
    close(c.y, center.y + 30.);
    let m = delta(&a, &mid);
    close(m[1].atan2(m[0]), std::f32::consts::FRAC_PI_4);
    close(Vec2::new(m[0], m[1]).length(), 1.5);
    close((m[0] * m[3] - m[1] * m[2]) / 1.5, 2.);
    assert_eq!(mid.style.instance.unwrap().first, 4);
}
#[test]
fn bezier_path_and_acceleration_change_timing_without_moving_endpoints() {
    let mut c = Curve::default();
    close(c.time(0.5), 0.5);
    c.ease = [[0.42, 0.], [1., 1.]];
    assert!(c.time(0.5) < 0.4);
    assert_eq!(c.time(0.), 0.);
    assert_eq!(c.time(1.), 1.);
    c.path = [[0., -100.], [0., -100.]];
    let p = c.point(Pos2::ZERO, Pos2::new(100., 0.), 0.5);
    close(p.x, 50.);
    close(p.y, -75.);
}
#[test]
fn explicitly_enabled_motion_preserves_source_and_undo() {
    let mut e = editor();
    let a = rect();
    for f in &mut e.doc.layers[0].frames {
        *f = vec![a.clone()].into();
    }
    e.doc.layers[0].options.keys[10] = true;
    e.doc.layers[0].options.tweens.push(Tween {
        start: 0,
        end: 10,
        curve: Default::default(),
    });
    e.frame = 10;
    e.checkpoint();
    e.autokey_motion(&[(0, 0)]);
    e.doc.layers[0].frames[10][0] = crate::vector::transform(&a, |p| p + Vec2::new(100., 0.));
    e.canvas.selected = vec![(0, 0)];
    e.recover();
    assert!(e.doc.layers[0].key(10));
    assert!(!e.doc.layers[0].key(5));
    close(
        crate::vector::control_bounds(&marks(&e.doc.layers[0], 5)[0])
            .center()
            .x,
        80.,
    );
    assert_eq!(e.doc.layers[0].frames[0][0].points, a.points);
    e.frame = 5;
    e.checkpoint();
    e.autokey_motion(&[(0, 0)]);
    close(
        crate::vector::control_bounds(&e.doc.layers[0].frames[5][0])
            .center()
            .x,
        80.,
    );
    assert!(e.doc.layers[0].key(5));
    let copy = crate::fla::decode(&crate::fla::encode(&e.project_document()).unwrap()).unwrap();
    assert_eq!(copy.layers[0].options.tweens.len(), 1);
    close(
        crate::vector::control_bounds(&marks(&copy.layers[0], 7)[0])
            .center()
            .x,
        100.,
    );
}
#[test]
fn scenes_keep_art_names_order_and_movie_projection_after_save() {
    let mut e = editor();
    e.doc.layers[0].frames[0].push(rect());
    let first = scene_from(&e.doc, 1, "Room".into());
    let mut second = crate::fresh_document();
    let mut mark = rect();
    mark.fill = Some([10, 200, 50, 255]);
    second.layers[0].frames[0].push(mark);
    e.doc.extras.scenes = vec![first, scene_from(&second, 2, "Garden".into())].into();
    e.switch_scene(1);
    assert_eq!(e.doc.layers[0].frames[0][0].fill, Some([10, 200, 50, 255]));
    e.doc.layers[0].frames[0][0].fill = Some([40, 220, 80, 255]);
    e.switch_scene(0);
    assert_eq!(e.doc.layers[0].frames[0][0].fill, rect().fill);
    store_scene(&mut e.doc);
    let scene = e.doc.extras.scenes.remove(1);
    e.doc.extras.scenes.insert(0, scene);
    e.doc.extras.active_scene = 1;
    let copy = crate::fla::decode(&crate::fla::encode(&e.project_document()).unwrap()).unwrap();
    assert_eq!(movie_count(&copy), 48);
    assert_eq!(copy.extras.scenes[0].name, "Garden");
    assert_eq!(
        resolve_movie(&copy, 0, false)[0].2.fill,
        Some([40, 220, 80, 255])
    );
    assert_eq!(resolve_movie(&copy, 24, false)[0].2.fill, rect().fill);
    assert_eq!(crate::features::baked(&copy).layers[0].frames.len(), 48);
    assert!(movie_svg(&copy, 0).contains("rgb(40,220,80)"));
    e.animation.play_all = true;
    e.advance_animation(24);
    assert_eq!(e.doc.extras.active_scene, 0);
    assert_eq!(e.frame, 0);
}
#[test]
fn camera_keys_interpolate_and_editor_frame_stays_fixed() {
    let mut e = editor();
    e.add_camera();
    let c = &mut e.doc.extras.cameras[0];
    let base = c.center;
    camera_record(
        c,
        CameraKey {
            frame: 10,
            center: [base[0] + 100., base[1] + 50.],
            zoom: 2.,
            angle: 1.,
        },
    );
    let mid = camera_at(c, 5);
    close(mid.center[0], base[0] + 50.);
    close(mid.zoom, 1.5);
    close(mid.angle, 0.5);
    e.frame = 5;
    e.tool = crate::canvas::Tool::Camera;
    let view = crate::canvas::View {
        center: Pos2::new(500., 300.),
        stage_center: Pos2::new(base[0], base[1]),
        scale: 1.,
        angle: 0.,
    };
    let camera = e.camera_view(view);
    assert_eq!(camera.center, view.center);
    close(camera.screen(crate::vector::pos(mid.center)).x, 500.);
    let d = e.project_document();
    assert!(d.validate().is_ok());
    let copy = crate::fla::decode(&crate::fla::encode(&d).unwrap()).unwrap();
    close(camera_at(&copy.extras.cameras[0], 5).zoom, 1.5);
}
#[test]
fn eyedropper_copies_gradient_stops_alpha_and_direction_into_selected_fill() {
    let mut e = editor();
    let target = rect();
    let mut source = crate::vector::transform(&target, |p| p + Vec2::new(100., 0.));
    let mut g = Gradient::default();
    g.start = [110., 10.];
    g.end = [150., 30.];
    g.stops[0].color = [100, 10, 200, 130];
    source.style.gradient = Some(g.clone());
    e.doc.layers[0].frames[0] = vec![target, source].into();
    e.sample_apply_at(&[(0, 0)], Pos2::new(130., 20.), 1.);
    let result = e.doc.layers[0].frames[0][0]
        .style
        .gradient
        .as_ref()
        .unwrap();
    assert_eq!(result.stops, g.stops);
    assert_eq!(result.start, [10., 10.]);
    assert_eq!(result.end, [50., 30.]);
    assert_eq!(e.canvas.selected, vec![(0, 0)]);
    assert!(e.feature.gradient_on);
}
#[test]
fn gradient_fit_preserves_orientation() {
    let mut g = Gradient::default();
    g.end = [0., 100.];
    g.fit(eframe::egui::Rect::from_min_size(
        Pos2::ZERO,
        Vec2::new(100., 60.),
    ));
    close(g.start[0], 50.);
    close(g.end[0], 50.);
    close(g.start[1], 0.);
    close(g.end[1], 60.);
}
#[test]
fn invalid_scene_and_tween_metadata_are_rejected() {
    let mut e = editor();
    e.doc.layers[0].options.tweens.push(Tween {
        start: 5,
        end: 500,
        curve: Default::default(),
    });
    assert!(e.doc.validate().is_err());
    e.doc.layers[0].options.tweens.clear();
    let s = scene_from(&e.doc, 1, "A".into());
    e.doc.extras.scenes = vec![s.clone(), s].into();
    assert!(e.doc.validate().is_err());
}
#[test]
#[ignore = "manual release benchmark"]
fn benchmark_motion_resolution() {
    let mut d = crate::fresh_document();
    let mut first = vec![];
    for i in 0..200 {
        let mut m = crate::vector::transform(&rect(), |p| {
            p + Vec2::new((i % 20) as f32 * 45., (i / 20) as f32 * 35.)
        });
        m.style.track = i + 1;
        first.push(m);
    }
    let last = first
        .iter()
        .map(|m| crate::vector::transform(m, |p| p + Vec2::new(100., 20.)))
        .collect();
    d.layers[0].frames[0] = first.into();
    d.layers[0].frames[23] = last;
    d.layers[0].options.keys[23] = true;
    d.layers[0].options.tweens.push(Tween {
        start: 0,
        end: 23,
        curve: Default::default(),
    });
    let t = std::time::Instant::now();
    for f in 0..200 {
        std::hint::black_box(crate::features::resolve(&d, 1 + f % 22, false));
    }
    println!(
        "200 animated rectangles: {:?} per resolved frame",
        t.elapsed() / 200
    );
}
