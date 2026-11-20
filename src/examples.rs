//! Editable, procedural project examples shipped with the application.
use crate::{
    Document, Mark,
    animation::{CameraAnimation, CameraKey, Curve},
    document::Layer,
    features::{self, Instance, Playback, Symbol, SymbolClock},
    rig::{self, Binding, Bone, BoneKey, Influence},
};
use eframe::egui::{Pos2, Rect};

pub(crate) const PROJECTS: [(&str, &str); 3] = [
    ("Pip · Wave rig", ":builtin-pip-rig"),
    ("Woodland · Focus pull", ":builtin-woodland-depth"),
    ("Orbit · Live lighting", ":builtin-orbit-lights"),
];

pub(crate) fn project(key: &str) -> Option<Result<Document, String>> {
    let mut doc = match key {
        ":builtin-pip-rig" => robot(),
        ":builtin-woodland-depth" => woodland(),
        ":builtin-orbit-lights" => orbit(),
        _ => return None,
    };
    for layer in &mut doc.layers {
        layer.init_keys();
    }
    Some(doc.validate().map(|()| doc))
}

fn polygon(points: &[[f32; 2]], color: [u8; 4]) -> Mark {
    Mark {
        points: points.to_vec().into(),
        fill: Some(color),
        stroke: false,
        closed: true,
        ..Default::default()
    }
}
fn rectangle(a: [f32; 2], b: [f32; 2], color: [u8; 4]) -> Mark {
    let mut mark = polygon(&[a, b], color);
    mark.rectangle = true;
    mark
}
fn ellipse(center: [f32; 2], radius: [f32; 2], color: [u8; 4]) -> Mark {
    let points: Vec<_> = (0..64)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::TAU / 64.;
            [
                center[0] + radius[0] * angle.cos(),
                center[1] + radius[1] * angle.sin(),
            ]
        })
        .collect();
    polygon(&points, color)
}
fn round_box(a: [f32; 2], b: [f32; 2], radius: f32, color: [u8; 4]) -> Mark {
    crate::drawing::rounded_rectangle(
        Pos2::from(a),
        Pos2::from(b),
        radius,
        &Mark {
            fill: Some(color),
            stroke: false,
            ..Default::default()
        },
    )
}
fn layer(name: &str, marks: Vec<Mark>, count: usize) -> Layer {
    let cel = marks.into();
    Layer {
        name: name.into(),
        visible: true,
        frames: vec![cel; count].into(),
        options: Default::default(),
    }
}
fn document(count: usize) -> Document {
    let mut doc = Document {
        width: 960,
        height: 540,
        fps: 24,
        ..Default::default()
    };
    doc.layers = vec![layer("Background", vec![], count)].into();
    doc
}
fn camera(doc: &mut Document, name: &str, end: [f32; 2], zoom: f32, optics: crate::depth::Optics) {
    let count = doc.layers[0].frames.len();
    let first = CameraKey {
        optics: Some(optics.clone()),
        frame: 0,
        center: [480., 270.],
        zoom: 1.,
        angle: 0.,
    };
    let mut last_optics = optics.clone();
    if optics.strength > 0. {
        last_optics.focus = 1500.;
    }
    let last = CameraKey {
        optics: Some(last_optics),
        frame: count - 1,
        center: end,
        zoom,
        angle: 0.,
    };
    doc.extras.cameras.push(features::Camera {
        id: features::id(),
        name: name.into(),
        start: 0,
        duration: count,
        center: first.center,
        zoom: first.zoom,
        angle: 0.,
        optics,
        animation: CameraAnimation {
            keys: vec![first, last].into(),
            curve: Curve {
                pace: [0., 0.],
                ..Default::default()
            },
        },
    });
}

fn rig_piece(
    doc: &mut Document,
    name: &str,
    marks: Vec<Mark>,
    bone: u64,
    parent: Option<u64>,
    start: [f32; 2],
    end: [f32; 2],
    angles: &[f32],
) {
    let count = doc.layers[0].frames.len();
    let bounds = marks.iter().fold(Rect::NOTHING, |bounds, mark| {
        bounds.union(crate::vector::control_bounds(mark))
    });
    let symbol = features::id();
    let target = features::id();
    doc.extras.symbols.push(Symbol {
        id: symbol,
        name: name.into(),
        layers: vec![layer(name, marks, 1)].into(),
        rig: Default::default(),
        mouth_poses: Default::default(),
        library_folder: None,
        folders: Default::default(),
        perspective: Default::default(),
    });
    let mut mark = rectangle(bounds.min.into(), bounds.max.into(), [255; 4]);
    mark.fill = None;
    mark.style.track = target;
    mark.style.instance = Some(Instance {
        symbol,
        matrix: rig::IDENTITY,
        mode: Playback::Single,
        first: 0,
        born: 0,
        clock: SymbolClock::Scene,
    });
    doc.layers.push(layer(name, vec![mark], count));
    let keys = angles
        .iter()
        .enumerate()
        .map(|(index, angle)| BoneKey {
            frame: index * (count - 1) / (angles.len() - 1),
            angle: angle.to_radians(),
            curve: Curve {
                pace: [0., 0.],
                ..Default::default()
            },
            ..Default::default()
        })
        .collect();
    doc.extras.rig.bones.push(Bone {
        id: bone,
        name: name.into(),
        parent,
        start,
        end,
        radius: 35.,
        ik: false,
        targets: vec![target].into(),
        keys,
    });
    let mut binding = Binding::new(target, symbol, rig::IDENTITY, bounds);
    for cell in &mut binding.cells {
        cell.influences[0] = Influence { bone, weight: 1. };
    }
    doc.extras.rig.bindings.push(binding);
}

fn robot() -> Document {
    let mut doc = document(72);
    let mint = [126, 220, 190, 255];
    let dark = [31, 60, 72, 255];
    let pale = [221, 245, 229, 255];
    let yellow = [255, 200, 96, 255];
    doc.layers[0] = layer(
        "Backdrop",
        vec![
            rectangle([-100., -100.], [1060., 640.], [238, 232, 214, 255]),
            ellipse([476., 280.], [235., 228.], [223, 219, 201, 255]),
            ellipse([475., 458.], [118., 16.], [195, 202, 188, 255]),
        ],
        72,
    );
    let body = features::id();
    rig_piece(
        &mut doc,
        "Body",
        vec![
            round_box([413., 270.], [547., 397.], 29., dark),
            round_box([421., 276.], [539., 386.], 24., mint),
            round_box([426., 383.], [461., 450.], 14., dark),
            round_box([494., 383.], [529., 450.], 14., dark),
            round_box([414., 434.], [462., 459.], 10., mint),
            round_box([494., 434.], [542., 459.], 10., mint),
            ellipse([480., 318.], [19., 19.], pale),
            ellipse([480., 318.], [9., 9.], yellow),
            rectangle([454., 351.], [506., 359.], dark),
        ],
        body,
        None,
        [480., 390.],
        [480., 278.],
        &[0., -2., 0., 2., 0.],
    );
    let head = features::id();
    rig_piece(
        &mut doc,
        "Head",
        vec![
            round_box([399., 151.], [561., 273.], 34., dark),
            round_box([406., 157.], [554., 264.], 30., mint),
            round_box([418., 177.], [542., 235.], 22., dark),
            ellipse([449., 201.], [12., 17.], pale),
            ellipse([511., 201.], [12., 17.], pale),
            ellipse([454., 198.], [4., 6.], dark),
            ellipse([516., 198.], [4., 6.], dark),
            round_box([464., 243.], [496., 248.], 2., dark),
            rectangle([476., 129.], [484., 155.], dark),
            ellipse([480., 125.], [12., 12.], yellow),
            ellipse([398., 215.], [13., 23.], yellow),
            ellipse([562., 215.], [13., 23.], yellow),
        ],
        head,
        Some(body),
        [480., 278.],
        [480., 188.],
        &[0., 6., -5., 6., 0.],
    );
    let left = features::id();
    rig_piece(
        &mut doc,
        "Left arm",
        vec![
            ellipse([418., 291.], [21., 21.], yellow),
            round_box([371., 289.], [410., 365.], 18., dark),
            round_box([376., 291.], [405., 359.], 14., mint),
            ellipse([385., 374.], [23., 25.], mint),
        ],
        left,
        Some(body),
        [418., 291.],
        [386., 373.],
        &[0., -6., 4., -6., 0.],
    );
    let right = features::id();
    rig_piece(
        &mut doc,
        "Right upper arm",
        vec![
            ellipse([542., 291.], [21., 21.], yellow),
            polygon(
                &[[541., 275.], [564., 290.], [614., 241.], [596., 218.]],
                dark,
            ),
            polygon(
                &[[549., 278.], [564., 283.], [606., 239.], [595., 229.]],
                mint,
            ),
            ellipse([607., 232.], [18., 18.], yellow),
        ],
        right,
        Some(body),
        [542., 291.],
        [607., 232.],
        &[0., -5., 2., -5., 0.],
    );
    let forearm = features::id();
    rig_piece(
        &mut doc,
        "Waving hand",
        vec![
            round_box([591., 163.], [621., 234.], 14., dark),
            round_box([596., 164.], [616., 224.], 10., mint),
            round_box([583., 124.], [634., 175.], 18., mint),
            round_box([584., 107.], [596., 148.], 6., mint),
            round_box([603., 101.], [615., 148.], 6., mint),
            round_box([622., 110.], [634., 151.], 6., mint),
            round_box([570., 141.], [593., 163.], 9., mint),
        ],
        forearm,
        Some(right),
        [607., 232.],
        [607., 148.],
        &[0., -28., 24., -28., 0.],
    );
    doc
}

fn woodland() -> Document {
    let mut doc = document(72);
    doc.layers[0] = layer(
        "Sky",
        vec![
            rectangle([-200., -150.], [1200., 700.], [225, 218, 187, 255]),
            ellipse([710., 120.], [70., 70.], [250, 235, 177, 255]),
        ],
        72,
    );
    let trees = |offset: f32, base: f32, color| {
        (0..11)
            .map(|index| {
                let x = index as f32 * 120. - 120. + offset;
                let top = base - 180. - ((index * 37) % 105) as f32;
                polygon(&[[x, top], [x - 65., base], [x + 65., base]], color)
            })
            .collect::<Vec<_>>()
    };
    let mut distant = layer(
        "Distant pines · 1800",
        trees(0., 380., [124, 155, 141, 255]),
        72,
    );
    distant.options.depth.distance = 1800.;
    doc.layers.push(distant);
    let mut middle = trees(65., 430., [64, 112, 108, 255]);
    middle.push(rectangle([-200., 410.], [1200., 700.], [73, 116, 105, 255]));
    let mut middle = layer("Middle forest · 1400", middle, 72);
    middle.options.depth.distance = 1400.;
    doc.layers.push(middle);
    let cabin = vec![
        ellipse([485., 469.], [162., 26.], [53, 85, 81, 255]),
        rectangle([395., 342.], [581., 451.], [187, 103, 72, 255]),
        polygon(
            &[[370., 349.], [486., 253.], [608., 349.]],
            [49, 73, 79, 255],
        ),
        polygon(
            &[[386., 348.], [486., 269.], [591., 348.]],
            [156, 76, 65, 255],
        ),
        rectangle([462., 384.], [506., 452.], [58, 78, 76, 255]),
        rectangle([412., 369.], [445., 404.], [249, 204, 122, 255]),
        rectangle([529., 369.], [562., 404.], [249, 204, 122, 255]),
        rectangle([540., 270.], [557., 307.], [87, 79, 76, 255]),
        polygon(
            &[[462., 451.], [506., 451.], [584., 560.], [397., 560.]],
            [180, 170, 129, 255],
        ),
    ];
    doc.layers.push(layer("Cabin · focus 1000", cabin, 72));
    let mut foreground = trees(24., 690., [27, 70, 72, 255]);
    foreground.truncate(2);
    foreground.push(polygon(
        &[[920., 0.], [890., 540.], [1020., 540.], [1040., 0.]],
        [27, 70, 72, 255],
    ));
    let mut foreground = layer("Foreground · 500", foreground, 72);
    foreground.options.depth.distance = 500.;
    doc.layers.push(foreground);
    camera(
        &mut doc,
        "Focus cabin to forest",
        [540., 258.],
        1.08,
        crate::depth::Optics {
            focus: 1000.,
            range: 150.,
            strength: 48.,
            quality: crate::depth::Quality::High,
            ..Default::default()
        },
    );
    doc
}

fn orbit() -> Document {
    let mut doc = document(72);
    doc.layers[0] = layer(
        "Backdrop",
        vec![
            rectangle([-200., -100.], [1200., 700.], [23, 27, 43, 255]),
            ellipse([480., 451.], [270., 35.], [13, 17, 31, 255]),
        ],
        72,
    );
    let receiver = features::id();
    let mut marks = vec![
        round_box([285., 222.], [675., 428.], 44., [113, 123, 150, 255]),
        ellipse([393., 206.], [84., 84.], [191, 158, 185, 255]),
        ellipse([552., 202.], [79., 79.], [151, 181, 181, 255]),
        polygon(
            &[[439., 143.], [489., 71.], [541., 143.], [489., 213.]],
            [219, 185, 130, 255],
        ),
    ];
    for index in 0..6 {
        marks.push(round_box(
            [322. + index as f32 * 54., 291.],
            [348. + index as f32 * 54., 365.],
            11.,
            [186, 192, 203, 255],
        ));
    }
    let mut sculpture = layer("Sculpture", marks, 72);
    sculpture.options.light_receiver = Some(receiver);
    doc.layers.push(sculpture);
    for (name, color, phase) in [
        ("Coral orbit", [255, 130, 136], 0.),
        ("Blue orbit", [108, 187, 255], std::f32::consts::PI),
    ] {
        let mut light = crate::light::Light::demo_orbit(doc.width, doc.height, color, phase);
        light
            .receivers
            .push(crate::light::Receiver::Layer(receiver));
        let mut row = layer(name, vec![], 72);
        row.options.light = Some(light);
        doc.layers.push(row);
    }
    camera(
        &mut doc,
        "Studio camera",
        [480., 270.],
        1.,
        Default::default(),
    );
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_examples_remain_editable_and_animate_after_save() {
        let ctx = eframe::egui::Context::default();
        for (_, key) in PROJECTS {
            let doc = project(key).unwrap().unwrap();
            let bytes = crate::native_project::encode(&doc).unwrap();
            assert!(bytes.len() < 512 * 1024);
            let restored = crate::native_project::decode_if_native(&bytes)
                .unwrap()
                .unwrap();
            let mut cache = crate::paint::RenderCache::default();
            let first = crate::exporter::raster(&restored, 0, 192, 108, &ctx, &mut cache);
            let middle = crate::exporter::raster(&restored, 25, 192, 108, &ctx, &mut cache);
            assert_ne!(first, middle, "{key} must visibly animate");
            match key {
                ":builtin-pip-rig" => {
                    assert_eq!(restored.extras.rig.bones.len(), 5);
                    assert_eq!(restored.extras.rig.bindings.len(), 5);
                }
                ":builtin-woodland-depth" => {
                    assert!(restored.extras.cameras[0].optics.strength > 0.)
                }
                _ => assert!(
                    restored
                        .layers
                        .iter()
                        .filter(|layer| layer.options.light.is_some())
                        .count()
                        >= 2
                ),
            }
        }
    }
}

#[cfg(test)]
#[test]
#[ignore = "manual rendering of shipped examples for visual review"]
fn render_shipped_examples() {
    let folder = std::path::PathBuf::from(std::env::var_os("LIBRETOON_EXAMPLE_PREVIEWS").unwrap());
    std::fs::create_dir_all(&folder).unwrap();
    let ctx = eframe::egui::Context::default();
    for (index, (_, key)) in PROJECTS.into_iter().enumerate() {
        let doc = project(key).unwrap().unwrap();
        let mut cache = crate::paint::RenderCache::default();
        for frame in [0, 25] {
            let pixels = crate::exporter::raster(&doc, frame, 960, 540, &ctx, &mut cache);
            image::save_buffer(
                folder.join(format!("example-{index}-{frame}.png")),
                &pixels,
                960,
                540,
                image::ColorType::Rgb8,
            )
            .unwrap();
        }
    }
}
