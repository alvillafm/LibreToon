//! Opt-in documentation generator. Uses the real editor without a window or desktop capture.
use crate::{Document, Editor, Mark, document::Layer};
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::{collections::HashMap, path::Path};

const INK: [u8; 4] = [63, 44, 52, 255];
const BROWN: [u8; 4] = [164, 103, 73, 255];
const CREAM: [u8; 4] = [255, 229, 183, 255];

fn ellipse(c: [f32; 2], r: [f32; 2], fill: [u8; 4]) -> Mark {
    let k = 0.5522848;
    let nodes = (0..4)
        .map(|i| {
            let a = i as f32 * std::f32::consts::FRAC_PI_2;
            let p = [c[0] + r[0] * a.cos(), c[1] + r[1] * a.sin()];
            let t = [-r[0] * a.sin() * k, r[1] * a.cos() * k];
            crate::document::Node {
                mode: Default::default(),
                position: p,
                in_handle: [p[0] - t[0], p[1] - t[1]],
                out_handle: [p[0] + t[0], p[1] + t[1]],
            }
        })
        .collect::<Vec<_>>();
    Mark {
        nodes: nodes.into(),
        fill: Some(fill),
        color: INK,
        width: 5.,
        closed: true,
        ..Default::default()
    }
}
fn polygon(points: &[[f32; 2]], fill: [u8; 4]) -> Mark {
    Mark {
        points: points.to_vec().into(),
        fill: Some(fill),
        color: INK,
        width: 5.,
        closed: true,
        ..Default::default()
    }
}
fn rectangle(a: [f32; 2], b: [f32; 2], fill: [u8; 4]) -> Mark {
    Mark {
        points: vec![a, b].into(),
        rectangle: true,
        stroke: false,
        ..polygon(&[], fill)
    }
}
fn layer(name: &str, marks: Vec<Mark>, count: usize) -> Layer {
    let mut options = crate::features::LayerOptions::default();
    options.keys = (0..count).map(|f| f == 0).collect::<Vec<_>>().into();
    Layer {
        name: name.into(),
        visible: true,
        frames: vec![marks.into(); count].into(),
        options,
    }
}
fn eyes(closed: f32) -> Vec<Mark> {
    let mut marks = vec![];
    for x in [570., 710.] {
        marks.push(ellipse(
            [x, 319.],
            [42., (1. - closed) * 42. + 2.],
            [255, 250, 232, 255],
        ));
        if closed < 0.8 {
            marks.push(ellipse([x + 6., 321.], [15., (1. - closed) * 22.], INK));
            marks.push(ellipse([x + 10., 313.], [4., (1. - closed) * 5.], [255; 4]));
        }
    }
    marks
}
fn owl(stage: usize) -> Document {
    let count = 24;
    let mut body = vec![
        ellipse([640., 406.], [158., 202.], BROWN),
        polygon(&[[510., 269.], [499., 171.], [583., 232.]], BROWN),
        polygon(&[[697., 232.], [781., 171.], [770., 269.]], BROWN),
    ];
    if stage >= 1 {
        body.push(ellipse([640., 454.], [104., 129.], [223, 169, 111, 255]));
        body.push(ellipse([570., 319.], [68., 80.], CREAM));
        body.push(ellipse([710., 319.], [68., 80.], CREAM));
    }
    if stage >= 2 {
        body.push(polygon(
            &[[620., 356.], [660., 356.], [640., 391.]],
            [245, 164, 76, 255],
        ));
        for y in [438., 478., 518.] {
            for x in [610., 670.] {
                body.push(polygon(
                    &[[x - 7., y], [x + 7., y], [x, y + 13.]],
                    [185, 122, 79, 255],
                ));
            }
        }
        for x in [585., 675.] {
            body.push(ellipse([x, 605.], [40., 13.], [245, 164, 76, 255]));
        }
    }
    let mut doc = Document {
        width: 1280,
        height: 720,
        fps: 24,
        ..Default::default()
    };
    doc.layers = vec![
        layer(
            "Background",
            vec![rectangle([0., 0.], [1280., 720.], [246, 239, 224, 255])],
            count,
        ),
        layer("Body", body, count),
    ]
    .into();
    if stage >= 2 {
        doc.layers.push(layer(
            "Wing",
            vec![ellipse([784., 447.], [47., 113.], [126, 77, 62, 255])],
            count,
        ));
        doc.layers.push(layer("Eyes", eyes(0.), count));
    }
    doc
}
fn blink() -> Document {
    let mut doc = owl(2);
    let eye = doc.layers.last_mut().unwrap();
    for (frame, closure) in [(8, 0.5), (10, 1.), (12, 0.)] {
        eye.options.keys[frame] = true;
        for f in frame..24 {
            eye.frames[f] = eyes(closure).into();
        }
    }
    doc
}
fn forest(lit: bool) -> Document {
    let mut doc = blink();
    doc.layers[0] = layer(
        "Sky and distant trees",
        vec![
            rectangle([0., 0.], [1280., 720.], [32, 48, 73, 255]),
            ellipse([1015., 135.], [48., 48.], [243, 225, 172, 255]),
            rectangle([100., 40.], [137., 720.], [47, 75, 86, 255]),
            rectangle([305., -20.], [344., 720.], [47, 75, 86, 255]),
            rectangle([1010., 245.], [1048., 720.], [47, 75, 86, 255]),
            rectangle([1148., 0.], [1190., 720.], [47, 75, 86, 255]),
            ellipse([610., 820.], [930., 220.], [53, 85, 86, 255]),
        ],
        24,
    );
    doc.layers[0].options.depth.distance = 1500.;
    let mut front = layer(
        "Foreground tree",
        vec![
            polygon(
                &[[-110., -100.], [100., -100.], [160., 720.], [-110., 720.]],
                [17, 34, 49, 255],
            ),
            polygon(
                &[[57., 195.], [340., -20.], [390., -20.], [90., 260.]],
                [17, 34, 49, 255],
            ),
            ellipse([1180., 875.], [480., 270.], [17, 34, 49, 255]),
        ],
        24,
    );
    front.options.depth.distance = 650.;
    doc.layers.push(front);
    doc.extras.cameras.push(crate::features::Camera {
        id: crate::features::id(),
        name: "Forest camera".into(),
        start: 0,
        duration: 24,
        center: [640., 360.],
        zoom: 1.,
        angle: 0.,
        optics: Default::default(),
        animation: crate::animation::CameraAnimation {
            keys: vec![
                crate::animation::CameraKey {
                    frame: 0,
                    center: [640., 360.],
                    zoom: 1.,
                    angle: 0.,
                    optics: None,
                },
                crate::animation::CameraKey {
                    frame: 23,
                    center: [670., 345.],
                    zoom: 1.12,
                    angle: 0.,
                    optics: None,
                },
            ]
            .into(),
            curve: Default::default(),
        },
    });
    if lit {
        for i in 1..=4 {
            doc.layers[i].options.light_receiver = Some(100 + i as u64);
        }
        let mut light = layer("Warm rim light", vec![], 24);
        light.options.light = Some(
            serde_json::from_value(serde_json::json!({
                "kind": "Rim", "receivers": [{"Layer":101},{"Layer":102},{"Layer":103}],
                "base": { "color": [255, 184, 102], "intensity": 0.7, "angle": -140., "bevel": 32. }
            }))
            .unwrap(),
        );
        doc.layers.push(light);
    }
    doc
}
fn rigged() -> Document {
    use crate::{
        features::{Instance, Playback, Symbol, SymbolClock},
        rig::{self, Binding, Bone, BoneKey, Influence},
    };
    let mut doc = owl(2);
    for (index, name, parent, start, end, angles) in [
        (1, "Body", None, [640., 580.], [640., 350.], [0., 0., 0.]),
        (
            2,
            "Wing",
            Some(1000),
            [773., 350.],
            [793., 535.],
            [0., -0.55, 0.],
        ),
    ] {
        let bone = if index == 1 { 1000 } else { 1001 };
        let symbol = 2000 + index as u64;
        let track = 3000 + index as u64;
        let source = doc.layers[index].frames[0].clone();
        let bounds = source.iter().fold(Rect::NOTHING, |b, m| {
            b.union(crate::vector::control_bounds(m))
        });
        doc.extras.symbols.push(Symbol {
            id: symbol,
            name: name.into(),
            layers: vec![layer(name, source.to_vec(), 1)].into(),
            rig: Default::default(),
            mouth_poses: Default::default(),
            library_folder: None,
            folders: Default::default(),
            perspective: Default::default(),
        });
        let instance = Mark {
            points: vec![bounds.min.into(), bounds.max.into()].into(),
            rectangle: true,
            style: crate::features::MarkStyle {
                track,
                instance: Some(Instance {
                    symbol,
                    matrix: rig::IDENTITY,
                    mode: Playback::Single,
                    first: 0,
                    born: 0,
                    clock: SymbolClock::Scene,
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        doc.layers[index] = layer(name, vec![instance], 24);
        doc.extras.rig.bones.push(Bone {
            id: bone,
            name: name.into(),
            parent,
            start,
            end,
            radius: 35.,
            ik: false,
            targets: vec![track].into(),
            keys: angles
                .into_iter()
                .enumerate()
                .map(|(i, angle)| BoneKey {
                    frame: [0, 11, 23][i],
                    angle,
                    ..Default::default()
                })
                .collect::<Vec<_>>()
                .into(),
        });
        let mut binding = Binding::new(track, symbol, rig::IDENTITY, bounds);
        for cell in &mut binding.cells {
            cell.influences[0] = Influence { bone, weight: 1. };
        }
        doc.extras.rig.bindings.push(binding);
    }
    doc
}
fn save_frame(doc: &Document, frame: usize, path: &Path) {
    let ctx = egui::Context::default();
    let rgb = crate::exporter::raster(doc, frame, 960, 540, &ctx, &mut Default::default());
    image::save_buffer(path, &rgb, 960, 540, image::ColorType::Rgb8).unwrap();
}
fn screenshot(doc: Document, frame: usize, tool: crate::canvas::Tool, color: bool, path: &Path) {
    let ctx = egui::Context::default();
    let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
    e.add_document(doc, None);
    e.hub.home = false;
    e.frame = frame;
    e.layer = if tool == crate::canvas::Tool::Bones {
        2
    } else {
        e.doc.layers.len() - 1
    };
    e.studio.prefs = Default::default();
    e.studio.prefs.saving.enabled = false;
    e.studio.palette.open = color;
    e.onion = false;
    e.tool = tool;
    e.canvas.fit();
    if tool == crate::canvas::Tool::Camera {
        e.feature.selected_camera = e.doc.extras.cameras.first().map(|camera| camera.id);
    }
    if tool == crate::canvas::Tool::Bones {
        e.rigging.bone = Some(1001);
        e.rigging.create = false;
        e.canvas.selected = vec![(2, 0)];
    }
    if tool == crate::canvas::Tool::Node {
        e.canvas.selected = vec![(e.layer, 0)];
        e.canvas
            .set_node_ids(vec![(e.layer, 0, 0), (e.layer, 0, 1)]);
    }
    e.status.clear();
    crate::i18n::set_locale(crate::i18n::Language::English);
    crate::themes::ensure(&ctx, crate::themes::Theme::Studio);
    let mut textures = HashMap::<egui::TextureId, egui::ColorImage>::new();
    for tick in 0..20 {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600., 1000.))),
                time: Some(tick as f64 / 60.),
                ..Default::default()
            },
            |ui| {
                e.workspace(ui);
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(egui::Color32::from_gray(29))
                            .inner_margin(0.),
                    )
                    .show(ui, |ui| e.canvas_ui(ui));
            },
        );
        for (id, deltas) in &out.textures_delta.set {
            for delta in deltas {
                let egui::ImageData::Color(image) = &delta.image;
                if let Some([x, y]) = delta.pos {
                    let old = textures.get_mut(id).expect("initial texture");
                    for row in 0..image.size[1] {
                        let at = (y + row) * old.size[0] + x;
                        old.pixels[at..at + image.size[0]].copy_from_slice(
                            &image.pixels[row * image.size[0]..(row + 1) * image.size[0]],
                        );
                    }
                } else {
                    textures.insert(*id, (**image).clone());
                }
            }
        }
        if tick == 19 {
            out.textures_delta.set = textures
                .iter()
                .map(|(id, image)| {
                    (
                        *id,
                        [egui::epaint::ImageDelta::full(
                            image.clone(),
                            egui::TextureOptions::LINEAR,
                        )]
                        .into_iter()
                        .collect(),
                    )
                })
                .collect();
            let pixels = crate::exporter::raster_ui_test(&ctx, &mut out, 1600, 1000);
            image::save_buffer(path, &pixels, 1600, 1000, image::ColorType::Rgb8).unwrap();
        }
        out.textures_delta.clear();
    }
}

#[test]
#[ignore = "writes headless documentation images and tutorial projects; run alone with isolated LOCALAPPDATA"]
fn generate_manual_visuals() {
    let output = std::path::PathBuf::from(
        std::env::var_os("LIBRETOON_MANUAL_OUTPUT").expect("explicit output directory"),
    );
    let isolated = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    assert!(
        isolated.ends_with("manual-sandbox"),
        "use the documented isolated data directory"
    );
    std::fs::create_dir_all(output.join("projects")).unwrap();
    std::fs::create_dir_all(output.join("images")).unwrap();
    for (name, doc) in [
        ("01-owl", owl(2)),
        ("02-blink", blink()),
        ("03-forest", forest(false)),
        ("04-lighting", forest(true)),
        ("05-rig", rigged()),
    ] {
        doc.validate()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let bytes = crate::native_project::encode(&doc).unwrap();
        let restored = crate::native_project::decode_if_native(&bytes)
            .unwrap()
            .unwrap();
        restored.validate().unwrap();
        std::fs::write(output.join(format!("projects/{name}.ltoon")), bytes).unwrap();
        save_frame(&restored, 0, &output.join(format!("images/{name}.png")));
        println!("Generated {name}");
    }
    for stage in [0, 1] {
        save_frame(
            &owl(stage),
            0,
            &output.join(format!("images/01-build-{stage}.png")),
        );
    }
    for frame in [0, 8, 10, 12] {
        save_frame(
            &blink(),
            frame,
            &output.join(format!("images/blink-{frame}.png")),
        );
    }
    for frame in [0, 11, 23] {
        save_frame(
            &forest(false),
            frame,
            &output.join(format!("images/forest-{frame}.png")),
        );
        save_frame(
            &rigged(),
            frame,
            &output.join(format!("images/wing-{frame}.png")),
        );
    }
    use crate::canvas::Tool;
    screenshot(
        owl(2),
        0,
        Tool::Ellipse,
        true,
        &output.join("images/workspace-color.png"),
    );
    screenshot(
        blink(),
        10,
        Tool::Select,
        false,
        &output.join("images/workspace-timeline.png"),
    );
    screenshot(
        forest(false),
        0,
        Tool::Camera,
        false,
        &output.join("images/workspace-camera.png"),
    );
    screenshot(
        rigged(),
        0,
        Tool::Bones,
        false,
        &output.join("images/workspace-bones.png"),
    );
    println!(
        "Headless captures and native projects written to {}",
        output.display()
    );
}

#[test]
#[ignore = "headless node editor visual QA; use isolated LOCALAPPDATA"]
fn node_editor_visual_qa() {
    let local = std::env::var("LOCALAPPDATA").unwrap();
    assert!(Path::new(&local).ends_with("manual-sandbox"));
    let output = Path::new("target/qa/node-editor.png");
    screenshot(owl(2), 0, crate::canvas::Tool::Node, false, output);
}
