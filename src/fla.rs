//! Interoperable XFL subset: vector strokes and frame-by-frame animation.
use crate::{
    canvas::vertices,
    document::{Document, Layer, Mark},
};

use std::collections::HashMap;
use xml::{
    attribute::OwnedAttribute,
    reader::{EventReader, XmlEvent},
};
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn attr<'a>(a: &'a [OwnedAttribute], name: &str) -> Option<&'a str> {
    a.iter()
        .find(|a| a.name.local_name == name)
        .map(|a| a.value.as_str())
}
fn number(a: &[OwnedAttribute], name: &str, default: f32) -> Result<f32, String> {
    match attr(a, name) {
        Some(s) => s
            .parse::<f32>()
            .map_err(|_| format!("Valor XFL no válido: {name}")),
        None => Ok(default),
    }
}
pub fn encode(doc: &Document) -> Result<Vec<u8>, String> {
    doc.validate()?;
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><DOMDocument xmlns=\"http://ns.adobe.com/xfl/2008/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" width=\"960\" height=\"540\" frameRate=\"{}\" backgroundColor=\"#FFFFFF\" xflVersion=\"2.97\" creatorInfo=\"LibreToon\" currentTimeline=\"1\"><timelines><DOMTimeline name=\"Escena 1\" currentFrame=\"0\"><layers>",
        doc.fps
    );
    for layer in doc.layers.iter().rev() {
        xml += &format!(
            "<DOMLayer name=\"{}\" color=\"#5B9EFF\" visible=\"{}\"><frames>",
            escape(&layer.name),
            layer.visible
        );
        for (index, marks) in layer.frames.iter().enumerate() {
            xml +=
                &format!("<DOMFrame index=\"{index}\" duration=\"1\" keyMode=\"9728\"><elements>");
            for m in marks {
                let pts = vertices(m);
                if pts.is_empty() {
                    continue;
                }
                let mut edges = format!("!{} {}", pts[0].x * 20., pts[0].y * 20.);
                for p in pts.iter().skip(1) {
                    edges += &format!("|{} {}", p.x * 20., p.y * 20.);
                }
                if pts.len() == 1 {
                    edges += &format!("|{} {}", pts[0].x * 20. + 0.02, pts[0].y * 20.);
                }
                xml += &format!(
                    "<DOMShape isFloating=\"true\"><matrix><Matrix/></matrix><strokes><StrokeStyle index=\"1\"><SolidStroke weight=\"{}\" caps=\"round\" joints=\"round\" scaleMode=\"none\"><fill><SolidColor color=\"#{:02X}{:02X}{:02X}\" alpha=\"{}\"/></fill></SolidStroke></StrokeStyle></strokes><edges><Edge strokeStyle=\"1\" edges=\"{edges}\"/></edges></DOMShape>",
                    m.width,
                    m.color[0],
                    m.color[1],
                    m.color[2],
                    m.color[3] as f32 / 255.
                );
            }
            xml += "</elements></DOMFrame>";
        }
        xml += "</frames></DOMLayer>";
    }
    xml += "</layers></DOMTimeline></timelines></DOMDocument>";
    Ok(crate::fla_zip::pack(&[
        ("mimetype", b"application/vnd.adobe.xfl".to_vec()),
        ("DOMDocument.xml", xml.into_bytes()),
    ]))
}
fn unsupported(what: &str) -> String {
    format!(
        "Este FLA contiene {what}, todavía no admitidos por LibreToon. No se ha abierto ni modificado el archivo. Compatibilidad actual: escenario 960×540, trazos vectoriales y animación por fotogramas."
    )
}
fn coordinate(token: &str) -> Result<f32, String> {
    if let Some(hex) = token.strip_prefix('#') {
        let (whole, frac) = hex.split_once('.').unwrap_or((hex, "0"));
        let n = i64::from_str_radix(whole, 16).map_err(|_| "Coordenada XFL no válida")?;
        let signed = if n >= 0x800000 { n - 0x1000000 } else { n };
        let fraction = u32::from_str_radix(frac, 16).map_err(|_| "Fracción XFL no válida")? as f32
            / 16f32.powi(frac.len() as i32);
        Ok(signed as f32 + fraction)
    } else {
        token.parse().map_err(|_| "Coordenada XFL no válida".into())
    }
}
fn paths(edges: &str) -> Result<Vec<Vec<[f32; 2]>>, String> {
    let mut tokens = String::new();
    for c in edges.chars() {
        if "!|[]".contains(c) {
            tokens.push(' ');
            tokens.push(c);
            tokens.push(' ');
        } else {
            tokens.push(c);
        }
    }
    let tokens: Vec<_> = tokens.split_whitespace().collect();
    let mut at = 0;
    let mut result = vec![];
    let mut path: Vec<[f32; 2]> = vec![];
    while at < tokens.len() {
        let command = tokens[at];
        at += 1;
        let count = match command {
            "!" | "|" => 2,
            "[" | "]" => 4,
            _ => return Err(unsupported("comandos de trazado no compatibles")),
        };
        if at + count > tokens.len() {
            return Err("Trazado XFL truncado".into());
        }
        let values = tokens[at..at + count]
            .iter()
            .map(|t| coordinate(t).map(|v| v / 20.))
            .collect::<Result<Vec<_>, _>>()?;
        at += count;
        match command {
            "!" => {
                if !path.is_empty() {
                    result.push(std::mem::take(&mut path));
                }
                path.push([values[0], values[1]]);
            }
            "|" => path.push([values[0], values[1]]),
            _ => {
                let start = *path.last().ok_or("Curva sin origen")?;
                for step in 1..=20 {
                    let t = step as f32 / 20.;
                    let u = 1. - t;
                    path.push([
                        u * u * start[0] + 2. * u * t * values[0] + t * t * values[2],
                        u * u * start[1] + 2. * u * t * values[1] + t * t * values[3],
                    ]);
                }
            }
        }
    }
    if !path.is_empty() {
        result.push(path);
    }
    Ok(result)
}
pub fn decode(bytes: &[u8]) -> Result<Document, String> {
    let xml = crate::fla_zip::document(bytes)?;
    let mut doc = Document {
        version: 1,
        fps: 12,
        layers: vec![],
    };
    let mut timelines = 0;
    let mut frame_index = 0;
    let mut duration = 1;
    let mut marks = vec![];
    let mut styles: HashMap<String, (f32, [u8; 4])> = HashMap::new();
    let mut stroke_index = String::new();
    let mut width = 1.;
    let mut color = [0, 0, 0, 255];
    let mut in_shape = false;
    let mut in_stroke = false;
    let mut matrix = [1., 0., 0., 1., 0., 0.];
    let mut root = false;
    for event in EventReader::from_str(&xml) {
        match event.map_err(|e| format!("XML FLA no válido: {e}"))? {
            XmlEvent::StartElement {
                name,
                attributes: a,
                ..
            } => match name.local_name.as_str() {
                "DOMDocument" => {
                    root = true;
                    let w = number(&a, "width", 550.)?;
                    let h = number(&a, "height", 400.)?;
                    if w != 960. || h != 540. {
                        return Err(unsupported("dimensiones de escenario distintas de 960×540"));
                    }
                    let fps = number(&a, "frameRate", 24.)?;
                    if fps.fract() != 0. || !(1.0..=60.0).contains(&fps) {
                        return Err(unsupported(
                            "velocidades de fotogramas fuera del rango 1–60 fps enteros",
                        ));
                    }
                    doc.fps = fps as u32;
                    if attr(&a, "backgroundColor").is_some_and(|c| c != "#FFFFFF" && c != "#ffffff")
                    {
                        return Err(unsupported("fondos de color"));
                    }
                }
                "DOMTimeline" => {
                    timelines += 1;
                    if timelines > 1 {
                        return Err(unsupported("varias escenas"));
                    }
                }
                "DOMLayer" => {
                    if attr(&a, "layerType").is_some_and(|t| t != "normal") {
                        return Err(unsupported("máscaras, guías o carpetas de capas"));
                    }
                    doc.layers.push(Layer {
                        name: attr(&a, "name").unwrap_or("Capa").into(),
                        visible: attr(&a, "visible") != Some("false"),
                        frames: vec![],
                    });
                }
                "DOMFrame" => {
                    frame_index = number(&a, "index", 0.)? as usize;
                    duration = number(&a, "duration", 1.)? as usize;
                    if frame_index + duration > 10000 || duration == 0 {
                        return Err("Duración XFL no válida".into());
                    }
                    if attr(&a, "tweenType").is_some_and(|t| t != "none")
                        || attr(&a, "soundName").is_some()
                    {
                        return Err(unsupported("interpolaciones o audio"));
                    }
                    marks.clear();
                }
                "DOMShape" => {
                    in_shape = true;
                    styles.clear();
                    matrix = [1., 0., 0., 1., 0., 0.];
                }
                "Matrix" if in_shape => {
                    matrix = [
                        number(&a, "a", 1.)?,
                        number(&a, "b", 0.)?,
                        number(&a, "c", 0.)?,
                        number(&a, "d", 1.)?,
                        number(&a, "tx", 0.)?,
                        number(&a, "ty", 0.)?,
                    ];
                }
                "StrokeStyle" => {
                    in_stroke = true;
                    stroke_index = attr(&a, "index").unwrap_or("1").into();
                    width = 1.;
                    color = [0, 0, 0, 255];
                }
                "SolidStroke" => {
                    width = number(&a, "weight", 1.)?;
                }
                "SolidColor" if in_stroke => {
                    let hex = attr(&a, "color")
                        .unwrap_or("#000000")
                        .trim_start_matches('#');
                    if hex.len() != 6 {
                        return Err("Color XFL no válido".into());
                    }
                    for (i, c) in color.iter_mut().take(3).enumerate() {
                        *c = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)
                            .map_err(|_| "Color XFL no válido")?;
                    }
                    color[3] = (number(&a, "alpha", 1.)?.clamp(0., 1.) * 255.).round() as u8;
                }
                "Edge" => {
                    if attr(&a, "fillStyle0").is_some_and(|v| v != "0")
                        || attr(&a, "fillStyle1").is_some_and(|v| v != "0")
                        || attr(&a, "cubics").is_some()
                    {
                        return Err(unsupported("rellenos vectoriales o curvas cúbicas"));
                    }
                    let id = attr(&a, "strokeStyle").unwrap_or("0");
                    if id == "0" {
                        continue;
                    }
                    let (width, color) =
                        *styles.get(id).ok_or("Estilo de trazo XFL desconocido")?;
                    for points in paths(attr(&a, "edges").unwrap_or(""))? {
                        let mut mark = Mark {
                            points,
                            color,
                            width,
                            rectangle: false,
                        };
                        for p in &mut mark.points {
                            let x = p[0];
                            let y = p[1];
                            *p = [
                                matrix[0] * x + matrix[2] * y + matrix[4],
                                matrix[1] * x + matrix[3] * y + matrix[5],
                            ];
                        }
                        marks.push(mark);
                    }
                }
                "Include"
                | "DOMBitmapItem"
                | "DOMSoundItem"
                | "DOMVideoItem"
                | "DOMOvalObject"
                | "DOMRectangleObject"
                | "DOMTLFText"
                | "RaggedStroke"
                | "StippleStroke"
                | "HatchedStroke"
                | "ActionScript"
                | "DOMSymbolInstance"
                | "DOMBitmapInstance"
                | "DOMStaticText"
                | "DOMDynamicText"
                | "DOMInputText"
                | "DOMGroup"
                | "LinearGradient"
                | "RadialGradient"
                | "BitmapFill"
                | "DOMVideoInstance"
                | "DOMComponentInstance"
                | "filters"
                | "script" => {
                    return Err(unsupported(
                        "símbolos, imágenes, textos, grupos, efectos o scripts",
                    ));
                }
                _ => {}
            },
            XmlEvent::EndElement { name } => match name.local_name.as_str() {
                "StrokeStyle" => {
                    styles.insert(stroke_index.clone(), (width, color));
                    in_stroke = false;
                }
                "DOMShape" => in_shape = false,
                "DOMFrame" => {
                    let layer = doc.layers.last_mut().ok_or("Fotograma sin capa")?;
                    layer.frames.resize(frame_index + duration, vec![]);
                    for f in frame_index..frame_index + duration {
                        layer.frames[f] = marks.clone();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    if !root || timelines != 1 || doc.layers.is_empty() {
        return Err("Documento XFL incompleto".into());
    }
    let count = doc
        .layers
        .iter()
        .map(|l| l.frames.len())
        .max()
        .unwrap_or(1)
        .max(1);
    for l in &mut doc.layers {
        l.frames.resize(count, vec![]);
    }
    doc.layers.reverse();
    doc.validate()?;
    Ok(doc)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::transformed;
    use eframe::egui::{Pos2, Vec2};
    #[test]
    fn native_fla_roundtrip_geometry_layers_and_frames() {
        let mut d = Document::default();
        d.layers[0].name = "Dibujo & escena".into();
        let m = Mark {
            points: vec![[20., 30.], [140., 180.]],
            width: 6.,
            color: [32, 64, 128, 200],
            rectangle: true,
        };
        d.layers[0].frames[2].push(transformed(
            &m,
            Pos2::new(20., 30.),
            Vec2::new(2., 1.),
            0.7,
            Vec2::new(-100., 10.),
        ));
        d.layers[0].frames[0].push(m.clone());
        let b = encode(&d).unwrap();
        assert_eq!(&b[..2], b"PK");
        if let Ok(path) = std::env::var("LIBRETOON_FLA_FIXTURE") {
            std::fs::write(path, &b).unwrap();
        }
        let restored = decode(&b).unwrap();
        assert_eq!(restored.layers[0].name, d.layers[0].name);
        assert_eq!(restored.layers[0].frames.len(), 24);
        assert_eq!(
            restored.layers[0].frames[2][0].points,
            d.layers[0].frames[2][0].points
        );
    }
    #[test]
    fn refuses_unsupported_content() {
        let x = "<DOMDocument width=\"960\" height=\"540\"><timelines><DOMTimeline><layers><DOMLayer><frames><DOMFrame><elements><DOMSymbolInstance/></elements></DOMFrame></frames></DOMLayer></layers></DOMTimeline></timelines></DOMDocument>";
        let b = crate::fla_zip::pack(&[("DOMDocument.xml", x.as_bytes().to_vec())]);
        assert!(decode(&b).err().unwrap().contains("símbolos"));
    }
    #[test]
    fn quadratic_curve_and_hex_coordinates() {
        let p = paths("!0 0[200 400 400 0").unwrap();
        assert_eq!(p[0].len(), 21);
        assert!((p[0][10][1] - 10.).abs() < 0.001);
        assert!((coordinate("#FFFFFF.80").unwrap() + 0.5).abs() < 0.001);
    }
}
