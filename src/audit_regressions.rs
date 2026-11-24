#[cfg(test)]
mod audit_20261123_regressions {
    use crate::*;
    use eframe::egui::{self, Pos2, Rect, Vec2};
    fn editor(ctx: egui::Context) -> Editor {
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx));
        e.doc = crate::fresh_document();
        e.doc.layers.truncate(1);
        e.recovery =
            std::env::temp_dir().join(format!("audit-probe-{}.ltoon", crate::features::id()));
        crate::i18n::set_locale(crate::i18n::Language::English);
        e
    }
    #[test]
    fn puppet_keeps_stroke_gradient() {
        let mut binding = crate::rig::Binding::new(
            1,
            1,
            crate::rig::IDENTITY,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(100., 100.)),
        );
        binding.pins.push(crate::rig::Pin {
            id: 1,
            name: "pin".into(),
            rest: [0., 0.],
            keys: vec![crate::rig::PinKey {
                frame: 0,
                position: [10., 10.],
                curve: Default::default(),
            }]
            .into(),
        });
        let poses = std::collections::HashMap::new();
        let deform = crate::rig::Deformer::new(&binding, &poses, 0);
        let mut mark = Mark {
            points: vec![[0., 20.], [100., 20.]].into(),
            width: 10.,
            ..Default::default()
        };
        mark.style.stroke_gradient = Some(crate::features::Gradient::default());
        let rendered = deform.mark(&mark, crate::rig::IDENTITY);
        assert!(!rendered.is_empty());
        assert!(rendered.iter().all(|m| m.style.gradient.is_some()));
        eprintln!(
            "AUDIT rig: active puppet deformation produces {} solid outline pieces; stroke gradient retained",
            rendered.len()
        );
    }
    #[test]
    fn eyedropper_samples_displayed_stroke_gradient() {
        let mut e = editor(egui::Context::default());
        let mut mark = Mark {
            points: vec![[0., 20.], [100., 20.]].into(),
            color: [0, 0, 0, 255],
            width: 10.,
            ..Default::default()
        };
        let mut g = crate::features::Gradient::default();
        g.start = [0., 20.];
        g.end = [100., 20.];
        g.stops[0].color = [255, 0, 0, 255];
        g.stops[1].color = [0, 0, 255, 255];
        mark.style.stroke_gradient = Some(g.clone());
        e.doc.layers[0].frames[0] = vec![mark].into();
        e.sample_at(Pos2::new(10., 20.), 1., Some(false));
        let expected = g.color(Pos2::new(10., 20.));
        eprintln!(
            "AUDIT eyedropper: displayed={expected:?}, sampled={:?}",
            e.color.to_array()
        );
        assert_eq!(e.color.to_array(), expected);
    }
    #[test]
    fn wand_selects_visible_interpolated_drawing() {
        let mut e = editor(egui::Context::default());
        let mut mark = Mark {
            points: vec![[0., 0.], [100., 100.]].into(),
            rectangle: true,
            closed: true,
            fill: Some([255, 0, 0, 255]),
            ..Default::default()
        };
        mark.style.track = crate::features::id();
        for cel in &mut e.doc.layers[0].frames {
            *cel = vec![mark.clone()].into();
        }
        e.doc.layers[0].options.keys[10] = true;
        e.doc.layers[0]
            .options
            .tweens
            .push(crate::animation::Tween {
                shape: false,
                start: 0,
                end: 10,
                curve: Default::default(),
            });
        e.frame = 10;
        e.autokey_motion(&[(0, 0)]);
        e.doc.layers[0].frames[10][0] =
            crate::vector::transform(&mark, |p| p + Vec2::new(200., 0.));
        e.frame = 5;
        let p = Pos2::new(150., 50.);
        assert!(crate::vector::contains_fill(
            &crate::animation::marks(&e.doc.layers[0], 5)[0],
            p
        ));
        let mut state = crate::canvas::Canvas::default();
        e.wand_select(&mut state, p, false);
        eprintln!(
            "AUDIT wand: visible rectangle at frame 5, selected={:?}",
            state.selected
        );
        assert_eq!(state.selected, vec![(0, 0)]);
        assert!(e.undo.is_empty());
    }
    #[test]
    fn native_project_roundtrips_more_than_256_mib() {
        let mut doc = crate::fresh_document();
        let data: crate::shared::SharedVec<u8> = vec![0u8; 64 * 1024 * 1024].into();
        for id in 1..=4 {
            let mut asset:crate::media::Asset=serde_json::from_value(serde_json::json!({"id":id,"name":"audio","kind":"Audio","data":[1],"size":[0,0],"seconds":1.0,"peaks":[]})).unwrap();
            asset.data = data.clone();
            doc.extras.media.assets.push(asset);
        }
        doc.validate().unwrap();
        let bytes = crate::native_project::encode(&doc).unwrap();
        let restored = crate::native_project::decode_if_native(&bytes)
            .unwrap()
            .unwrap();
        assert_eq!(restored.extras.media.assets.len(), 4);
        assert_eq!(restored.extras.media.assets[3].data.len(), 64 * 1024 * 1024);
        drop(restored);
        let path =
            std::env::temp_dir().join(format!("large-native-{}.ltoon", crate::features::id()));
        crate::write_document(&path, &doc).unwrap();
        let reopened = crate::documents::read_project(&path).unwrap();
        assert_eq!(reopened.extras.media.assets.len(), 4);
        assert_eq!(reopened.extras.media.assets[0].data.len(), 64 * 1024 * 1024);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn one_light_drag_has_one_undo_entry() {
        let ctx = egui::Context::default();
        let mut e = editor(ctx.clone());
        e.doc.layers[0].options.light = Some(crate::light::Light::new(
            1920,
            1080,
            crate::light::Kind::Point,
        ));
        let tick = |e: &mut Editor, events| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600., 1200.))),
                    events,
                    ..Default::default()
                },
                |ui| e.light_properties(ui),
            );
            out.textures_delta.clear();
            out
        };
        let _ = tick(&mut e, vec![]);
        let out = tick(&mut e, vec![]);
        let texts = out
            .shapes
            .iter()
            .filter_map(|s| {
                if let egui::Shape::Text(t) = &s.shape {
                    Some(t)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let label = texts
            .iter()
            .find(|t| t.galley.job.text == "X")
            .expect("X label");
        let field = texts
            .iter()
            .filter(|t| (t.pos.y - label.pos.y).abs() < 5. && t.pos.x > label.pos.x + 10.)
            .min_by(|a, b| a.pos.x.total_cmp(&b.pos.x))
            .expect("X field");
        let p = field.pos + Vec2::new(6., 5.);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: Default::default(),
        };
        tick(&mut e, vec![egui::Event::PointerMoved(p), button(p, true)]);
        for x in [12., 24., 36.] {
            tick(
                &mut e,
                vec![egui::Event::PointerMoved(p + Vec2::new(x, 0.))],
            );
        }
        tick(&mut e, vec![button(p + Vec2::new(36., 0.), false)]);
        eprintln!(
            "AUDIT light X field: one drag produced {} undo entries",
            e.undo.len()
        );
        assert_eq!(e.undo.len(), 1);
    }
}

#[test]
fn malformed_native_and_svg_inputs_do_not_panic() {
    let doc = crate::fresh_document();
    let bytes = crate::native_project::encode(&doc).unwrap();
    for i in (0..bytes.len()).step_by((bytes.len() / 80).max(1)) {
        let mut changed = bytes.clone();
        changed[i] ^= 0xff;
        assert!(
            std::panic::catch_unwind(|| crate::native_project::decode_if_native(&changed)).is_ok()
        );
    }
    for xml in [
        "<svg><path d='M NaN Inf'/></svg>",
        "<!DOCTYPE svg [<!ENTITY a 'x'>]><svg>&a;</svg>",
        "<svg><g>",
        "<svg><path d='M0 0 C'/></svg>",
    ] {
        assert!(std::panic::catch_unwind(|| crate::svg_import::parse(xml)).is_ok());
    }
}

#[test]
fn repeated_edits_in_one_gesture_keep_held_exposures_synchronized() {
    let ctx = eframe::egui::Context::default();
    let mut e = crate::Editor::new(&eframe::CreationContext::_new_kittest(ctx));
    e.doc = crate::fresh_document();
    let cel: crate::shared::SharedVec<crate::Mark> = vec![crate::Mark {
        points: vec![[10., 20.], [30., 40.]].into(),
        ..Default::default()
    }]
    .into();
    for frame in &mut e.doc.layers[0].frames {
        *frame = cel.clone();
    }
    e.checkpoint();
    for x in [25., 35., 45.] {
        e.doc.layers[0].frames[0][0].points[0][0] = x;
        e.sync_exposures();
        assert!(
            e.doc.layers[0]
                .frames
                .iter()
                .all(|cel| cel[0].points[0][0] == x)
        );
    }
}
