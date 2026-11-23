//! Bounded, data-only SVG import. Unsupported appearance is reported, never silently executed.
use crate::{
    Editor, Mark,
    document::Node,
    rig::{self, Matrix},
    shared::SharedVec,
};
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::mpsc,
};
use svgtypes::{SimplePathSegment as Segment, SimplifyingPathParser};

const LIMIT: usize = 16 * 1024 * 1024;
pub(crate) struct Imported {
    pub marks: Vec<Mark>,
    pub warnings: BTreeSet<String>,
}
pub(crate) struct Dialog {
    owner: PathBuf,
    name: String,
    job: mpsc::Receiver<Result<Imported, String>>,
    result: Option<Result<Imported, String>>,
    fit: bool,
}
#[derive(Clone)]
struct Style {
    fill: String,
    stroke: String,
    width: f32,
    opacity: f32,
    fill_opacity: f32,
    stroke_opacity: f32,
    cap: crate::features::Cap,
    hidden: bool,
    evenodd: bool,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            fill: "black".into(),
            stroke: "none".into(),
            width: 1.,
            opacity: 1.,
            fill_opacity: 1.,
            stroke_opacity: 1.,
            cap: crate::features::Cap::Flat,
            hidden: false,
            evenodd: false,
        }
    }
}
#[derive(Clone)]
struct Context {
    matrix: Matrix,
    style: Style,
    skip: bool,
}
#[derive(Clone, Default)]
struct GradientDef {
    attrs: BTreeMap<String, String>,
    stops: Vec<crate::features::Stop>,
    radial: bool,
}
fn attrs(attributes: Vec<xml::attribute::OwnedAttribute>) -> BTreeMap<String, String> {
    let mut a: BTreeMap<_, _> = attributes
        .into_iter()
        .map(|a| (a.name.local_name, a.value))
        .collect();
    if let Some(style) = a.get("style").cloned() {
        for rule in style.split(';') {
            if let Some((k, v)) = rule.split_once(':') {
                a.insert(k.trim().into(), v.trim().into());
            }
        }
    }
    a
}
fn gradients(xml: &str) -> Result<BTreeMap<String, GradientDef>, String> {
    use xml::reader::{ParserConfig, XmlEvent};
    let mut gradients = BTreeMap::new();
    let mut active: Option<String> = None;
    let mut depth = 0;
    for event in ParserConfig::new()
        .max_attributes(256)
        .max_attribute_length(LIMIT)
        .max_data_length(LIMIT)
        .create_reader(xml.as_bytes())
    {
        match event.map_err(|e| e.to_string())? {
            XmlEvent::StartElement {
                name,
                attributes: a,
                ..
            } => {
                depth += 1;
                if depth > 64 {
                    return Err("SVG exceeds nesting limits".into());
                }
                let a = attrs(a);
                if matches!(
                    name.local_name.as_str(),
                    "linearGradient" | "radialGradient"
                ) {
                    active = a.get("id").cloned();
                    if let Some(id) = &active {
                        gradients.insert(
                            id.clone(),
                            GradientDef {
                                attrs: a,
                                stops: vec![],
                                radial: name.local_name == "radialGradient",
                            },
                        );
                    }
                } else if name.local_name == "stop"
                    && let Some(def) = active.as_ref().and_then(|id| gradients.get_mut(id))
                {
                    if def.stops.len() >= 32 {
                        return Err("SVG gradient exceeds 32 stops".into());
                    }
                    let offset = a.get("offset").map_or("0", String::as_str);
                    let t = if let Some(percent) = offset.strip_suffix('%') {
                        percent
                            .parse::<f32>()
                            .map_err(|_| "Invalid gradient stop")?
                            / 100.
                    } else {
                        offset.parse().map_err(|_| "Invalid gradient stop")?
                    };
                    if !t.is_finite() {
                        return Err("Invalid gradient stop".into());
                    }
                    let c = a
                        .get("stop-color")
                        .map_or("black", String::as_str)
                        .parse::<svgtypes::Color>()
                        .map_err(|_| "Invalid gradient color")?;
                    let alpha = a
                        .get("stop-opacity")
                        .map_or(Ok(1.), |v| v.parse::<f32>())
                        .map_err(|_| "Invalid gradient opacity")?;
                    def.stops.push(crate::features::Stop {
                        t: t.clamp(0., 1.),
                        color: [
                            c.red,
                            c.green,
                            c.blue,
                            (c.alpha as f32 * alpha.clamp(0., 1.)) as u8,
                        ],
                    });
                }
                if gradients.len() > 1000 {
                    return Err("SVG exceeds 1000 gradients".into());
                }
            }
            XmlEvent::EndElement { name, .. } => {
                depth -= 1;
                if matches!(
                    name.local_name.as_str(),
                    "linearGradient" | "radialGradient"
                ) {
                    active = None;
                }
            }
            _ => {}
        }
    }
    Ok(gradients)
}
fn gradient(
    value: &str,
    defs: &BTreeMap<String, GradientDef>,
    bounds: Rect,
    alpha: f32,
    warnings: &mut BTreeSet<String>,
) -> Option<crate::features::Gradient> {
    let id = value.strip_prefix("url(#")?.strip_suffix(')')?;
    let mut def = defs.get(id)?.clone();
    let radial = def.radial;
    let mut parent = def
        .attrs
        .get("href")
        .and_then(|s| s.strip_prefix('#'))
        .map(str::to_owned);
    let mut visited = BTreeSet::from([id.to_string()]);
    while let Some(id) = parent.take() {
        if visited.len() > 16 || !visited.insert(id.clone()) {
            return None;
        }
        let inherited = defs.get(&id)?;
        if def.stops.is_empty() {
            def.stops = inherited.stops.clone();
        }
        for (key, value) in &inherited.attrs {
            def.attrs
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
        parent = inherited
            .attrs
            .get("href")
            .and_then(|s| s.strip_prefix('#'))
            .map(str::to_owned);
    }
    if def.stops.is_empty() {
        return None;
    }
    if def.stops.len() == 1 {
        def.stops.push(crate::features::Stop {
            t: 1.,
            color: def.stops[0].color,
        });
    }
    def.stops.sort_by(|a, b| a.t.total_cmp(&b.t));
    for stop in &mut def.stops {
        stop.color[3] = (stop.color[3] as f32 * alpha.clamp(0., 1.)) as u8;
    }
    let user = def
        .attrs
        .get("gradientUnits")
        .is_some_and(|v| v == "userSpaceOnUse");
    let coordinate = |key: &str, default: f32, axis: usize| -> Option<f32> {
        let value = def.attrs.get(key);
        let n = match value {
            Some(value) if value.ends_with('%') => {
                value.trim_end_matches('%').parse::<f32>().ok()? / 100.
            }
            Some(value) => number(value).ok()?,
            None => default,
        };
        Some(if user {
            n
        } else {
            [bounds.left(), bounds.top()][axis] + n * [bounds.width(), bounds.height()][axis]
        })
    };
    let (start, end) = if radial {
        let center = [coordinate("cx", 0.5, 0)?, coordinate("cy", 0.5, 1)?];
        let radius = def
            .attrs
            .get("r")
            .map(|s| {
                s.trim_end_matches('%')
                    .parse::<f32>()
                    .ok()
                    .map(|n| if s.ends_with('%') { n / 100. } else { n })
            })
            .unwrap_or(Some(0.5))?;
        if def.attrs.contains_key("fx") || def.attrs.contains_key("fy") {
            warnings.insert("Radial gradient focal offsets were approximated as centered.".into());
        }
        if !user && (bounds.width() - bounds.height()).abs() > 0.1 {
            warnings.insert("Elliptical radial gradients were approximated as circular.".into());
        }
        (
            center,
            [
                center[0] + radius * if user { 1. } else { bounds.width() },
                center[1],
            ],
        )
    } else {
        (
            [coordinate("x1", 0., 0)?, coordinate("y1", 0., 1)?],
            [coordinate("x2", 1., 0)?, coordinate("y2", 0., 1)?],
        )
    };
    if def.attrs.get("spreadMethod").is_some_and(|s| s != "pad") {
        warnings.insert("Gradient repeat/reflect spread was approximated as pad.".into());
    }
    let matrix = def
        .attrs
        .get("gradientTransform")
        .map_or(Ok(rig::IDENTITY), |s| transform(s))
        .ok()?;
    let map = |p: [f32; 2]| {
        if user {
            rig::point(matrix, Pos2::from(p))
        } else {
            bounds.min
                + rig::point(
                    matrix,
                    ((Pos2::from(p) - bounds.min) / bounds.size().max(Vec2::splat(0.0001)))
                        .to_pos2(),
                )
                .to_vec2()
                    * bounds.size()
        }
    };
    let g = crate::features::Gradient {
        radial,
        start: map(start).into(),
        end: map(end).into(),
        stops: def.stops.into(),
    };
    g.valid().then_some(g)
}
fn number(value: &str) -> Result<f32, String> {
    let length: svgtypes::Length = value
        .parse()
        .map_err(|_| format!("Invalid SVG number: {value}"))?;
    let factor = match length.unit {
        svgtypes::LengthUnit::None | svgtypes::LengthUnit::Px => 1.,
        svgtypes::LengthUnit::In => 96.,
        svgtypes::LengthUnit::Cm => 96. / 2.54,
        svgtypes::LengthUnit::Mm => 96. / 25.4,
        svgtypes::LengthUnit::Pt => 96. / 72.,
        svgtypes::LengthUnit::Pc => 16.,
        _ => return Err(format!("Unsupported relative SVG length: {value}")),
    };
    let n = (length.number * factor) as f32;
    if !n.is_finite() || n.abs() > 1e6 {
        return Err("SVG coordinate exceeds supported limits".into());
    }
    Ok(n)
}
fn transform(value: &str) -> Result<Matrix, String> {
    let mut matrix = rig::IDENTITY;
    for token in svgtypes::TransformListParser::from(value) {
        use svgtypes::TransformListToken::*;
        let next = match token.map_err(|e| e.to_string())? {
            Matrix { a, b, c, d, e, f } => [a, b, c, d, e, f].map(|v| v as f32),
            Translate { tx, ty } => [1., 0., 0., 1., tx as f32, ty as f32],
            Scale { sx, sy } => [sx as f32, 0., 0., sy as f32, 0., 0.],
            Rotate { angle } => {
                let (s, c) = angle.to_radians().sin_cos();
                [c as f32, s as f32, -s as f32, c as f32, 0., 0.]
            }
            SkewX { angle } => [1., 0., angle.to_radians().tan() as f32, 1., 0., 0.],
            SkewY { angle } => [1., angle.to_radians().tan() as f32, 0., 1., 0., 0.],
        };
        matrix = rig::multiply(matrix, next);
        if matrix.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
            return Err("Invalid SVG transform".into());
        }
    }
    Ok(matrix)
}
fn color(value: &str, alpha: f32, warnings: &mut BTreeSet<String>) -> Option<[u8; 4]> {
    if value == "none" {
        return None;
    }
    match value.parse::<svgtypes::Color>() {
        Ok(c) => Some([
            c.red,
            c.green,
            c.blue,
            (c.alpha as f32 * alpha.clamp(0., 1.)).round() as u8,
        ]),
        Err(_) => {
            warnings.insert(format!(
                "Paint '{value}' is unsupported; choose a solid color after import."
            ));
            None
        }
    }
}
fn paths(data: &str) -> Result<Vec<Mark>, String> {
    let mut paths = vec![];
    let mut mark = Mark::default();
    let flush = |mark: &mut Mark, paths: &mut Vec<Mark>| {
        if !mark.nodes.is_empty() {
            paths.push(std::mem::take(mark));
        }
    };
    let mut total = 0;
    for segment in SimplifyingPathParser::from(data) {
        total += 1;
        if total > 200000 {
            return Err("SVG path is too complex".into());
        }
        match segment.map_err(|e| e.to_string())? {
            Segment::MoveTo { x, y } => {
                flush(&mut mark, &mut paths);
                mark.nodes.push(Node::corner([x as f32, y as f32]));
            }
            Segment::LineTo { x, y } => {
                mark.nodes.push(Node::corner([x as f32, y as f32]));
            }
            Segment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                mark.nodes
                    .last_mut()
                    .ok_or("Curve without a starting point")?
                    .out_handle = [x1 as f32, y1 as f32];
                mark.nodes.push(Node {
                    mode: Default::default(),
                    position: [x as f32, y as f32],
                    in_handle: [x2 as f32, y2 as f32],
                    out_handle: [x as f32, y as f32],
                });
            }
            Segment::Quadratic { x1, y1, x, y } => {
                let last = mark
                    .nodes
                    .last_mut()
                    .ok_or("Curve without a starting point")?;
                let p = Pos2::from(last.position);
                let q = Pos2::new(x1 as f32, y1 as f32);
                let end = Pos2::new(x as f32, y as f32);
                last.out_handle = (p + (q - p) * (2. / 3.)).into();
                mark.nodes.push(Node {
                    mode: Default::default(),
                    position: end.into(),
                    in_handle: (end + (q - end) * (2. / 3.)).into(),
                    out_handle: end.into(),
                });
            }
            Segment::ClosePath => {
                mark.closed = true;
                if mark.nodes.len() > 1
                    && mark.nodes.first().unwrap().position == mark.nodes.last().unwrap().position
                {
                    let last = mark.nodes.pop().unwrap();
                    mark.nodes[0].in_handle = last.in_handle;
                }
                flush(&mut mark, &mut paths);
            }
        }
    }
    flush(&mut mark, &mut paths);
    for path in &mut paths {
        for node in &mut path.nodes {
            node.mode = crate::node_edit::Mode::Inferred;
        }
    }
    if paths
        .iter()
        .flat_map(|m| m.nodes.iter())
        .flat_map(|n| [n.position, n.in_handle, n.out_handle])
        .flatten()
        .any(|n| !n.is_finite() || n.abs() > 1e6)
    {
        return Err("Invalid SVG path coordinates".into());
    }
    Ok(paths)
}
fn shape(name: &str, a: &BTreeMap<String, String>) -> Result<Vec<Mark>, String> {
    let n = |key: &str, default: f32| a.get(key).map_or(Ok(default), |v| number(v));
    let data = match name {
        "path" => a.get("d").cloned().unwrap_or_default(),
        "rect" => {
            let (x, y, w, h) = (n("x", 0.)?, n("y", 0.)?, n("width", 0.)?, n("height", 0.)?);
            if w <= 0. || h <= 0. {
                return Ok(vec![]);
            }
            let rx = n("rx", n("ry", 0.)?)?.clamp(0., w / 2.);
            let ry = n("ry", rx)?.clamp(0., h / 2.);
            format!(
                "M{} {} H{} A{rx} {ry} 0 0 1 {} {} V{} A{rx} {ry} 0 0 1 {} {} H{} A{rx} {ry} 0 0 1 {x} {} V{} A{rx} {ry} 0 0 1 {} {y} Z",
                x + rx,
                y,
                x + w - rx,
                x + w,
                y + ry,
                y + h - ry,
                x + w - rx,
                y + h,
                x + rx,
                y + h - ry,
                y + ry,
                x + rx
            )
        }
        "circle" | "ellipse" => {
            let (cx, cy) = (n("cx", 0.)?, n("cy", 0.)?);
            let (rx, ry) = if name == "circle" {
                let r = n("r", 0.)?;
                (r, r)
            } else {
                (n("rx", 0.)?, n("ry", 0.)?)
            };
            if rx <= 0. || ry <= 0. {
                return Ok(vec![]);
            }
            format!(
                "M{} {cy} A{rx} {ry} 0 1 0 {} {cy} A{rx} {ry} 0 1 0 {} {cy} Z",
                cx - rx,
                cx + rx,
                cx - rx
            )
        }
        "line" => format!(
            "M{} {} L{} {}",
            n("x1", 0.)?,
            n("y1", 0.)?,
            n("x2", 0.)?,
            n("y2", 0.)?
        ),
        "polygon" | "polyline" => {
            let points = a.get("points").map_or("", String::as_str);
            if points.trim().is_empty() {
                return Ok(vec![]);
            }
            let mut data = format!("M {points}");
            if name == "polygon" {
                data.push('Z');
            }
            data
        }
        _ => return Ok(vec![]),
    };
    paths(&data)
}
fn compound(marks: Vec<Mark>, evenodd: bool, warnings: &mut BTreeSet<String>) -> Vec<Mark> {
    use geo::{Area, BooleanOps, Contains};
    if marks.len() < 2 || marks.iter().all(|m| m.fill.is_none()) {
        return marks;
    }
    let fill = marks[0].fill.unwrap();
    let gradient = marks[0].style.gradient.clone();
    let mut rings: Vec<_> = marks.iter().map(|m| crate::vector::polygon(m)).collect();
    rings.sort_by(|a, b| b.unsigned_area().total_cmp(&a.unsigned_area()));
    let mut filled = geo::MultiPolygon(vec![]);
    for (i, ring) in rings.iter().enumerate() {
        let Some(first) = ring.exterior().0.first() else {
            continue;
        };
        let point = geo::Point::from(*first);
        let mut winding = 0;
        for parent in &rings[..i] {
            if parent.contains(&point) {
                winding += if evenodd {
                    1
                } else {
                    parent.signed_area().signum() as i32
                };
            }
        }
        let before = if evenodd {
            winding % 2 != 0
        } else {
            winding != 0
        };
        winding += if evenodd {
            1
        } else {
            ring.signed_area().signum() as i32
        };
        let after = if evenodd {
            winding % 2 != 0
        } else {
            winding != 0
        };
        if after && !before {
            filled = filled.union(ring);
        } else if before && !after {
            filled = filled.difference(ring);
        }
    }
    let mut result = crate::vector::polygons_to_marks(filled, fill);
    for mark in &mut result {
        mark.style.gradient = gradient.clone();
    }
    for mut mark in marks {
        if mark.stroke {
            mark.fill = None;
            result.push(mark);
        }
    }
    warnings.insert("Compound fills use editable flattened boundaries; original Bezier stroke paths are retained.".into());
    result
}
pub(crate) fn parse(xml: &str) -> Result<Imported, String> {
    use xml::reader::{ParserConfig, XmlEvent};
    if xml.len() > LIMIT {
        return Err("SVG exceeds 16 MiB".into());
    }
    if xml.contains("<!DOCTYPE") || xml.contains("<!ENTITY") {
        return Err("SVG document types and entities are unsupported".into());
    }
    let gradients = gradients(xml)?;
    let mut stack = vec![Context {
        matrix: rig::IDENTITY,
        style: Style::default(),
        skip: false,
    }];
    let mut result = Imported {
        marks: vec![],
        warnings: BTreeSet::new(),
    };
    let mut count = 0;
    let mut geometry = 0usize;
    let mut root = false;
    for event in ParserConfig::new()
        .max_attributes(256)
        .max_attribute_length(LIMIT)
        .max_data_length(LIMIT)
        .max_entity_expansion_length(1024)
        .create_reader(xml.as_bytes())
    {
        match event.map_err(|e| format!("Invalid SVG XML: {e}"))? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                count += 1;
                if count > 50000 || stack.len() > 64 {
                    return Err("SVG exceeds complexity limits".into());
                }
                let name = name.local_name;
                let mut a = attrs(attributes);
                if !root {
                    if name != "svg" {
                        return Err("The document is not SVG".into());
                    }
                    root = true;
                }
                if let Some(style) = a.get("style").cloned() {
                    for rule in style.split(';') {
                        if let Some((k, v)) = rule.split_once(':') {
                            a.insert(k.trim().into(), v.trim().into());
                        }
                    }
                }
                let mut current = stack.last().unwrap().clone();
                if matches!(
                    name.as_str(),
                    "script" | "foreignObject" | "use" | "image" | "text"
                ) {
                    result.warnings.insert(format!("SVG <{name}> content was skipped; convert supported artwork to paths in the source editor."));
                }
                current.skip |= matches!(
                    name.as_str(),
                    "defs"
                        | "metadata"
                        | "namedview"
                        | "script"
                        | "foreignObject"
                        | "clipPath"
                        | "mask"
                        | "symbol"
                );
                if let Some(value) = a.get("transform") {
                    current.matrix = rig::multiply(current.matrix, transform(value)?);
                }
                if stack.len() == 1
                    && let Some(v) = a.get("viewBox")
                {
                    let v: svgtypes::ViewBox = v.parse().map_err(|_| "Invalid SVG viewBox")?;
                    let w = a
                        .get("width")
                        .and_then(|v| number(v).ok())
                        .unwrap_or(v.w as f32);
                    let h = a
                        .get("height")
                        .and_then(|v| number(v).ok())
                        .unwrap_or(v.h as f32);
                    if v.w <= 0. || v.h <= 0. {
                        return Err("Empty SVG viewBox".into());
                    }
                    let scale = (w / v.w as f32).min(h / v.h as f32);
                    let map = [
                        scale,
                        0.,
                        0.,
                        scale,
                        (w - v.w as f32 * scale) / 2. - v.x as f32 * scale,
                        (h - v.h as f32 * scale) / 2. - v.y as f32 * scale,
                    ];
                    current.matrix = rig::multiply(current.matrix, map);
                    if a.get("preserveAspectRatio")
                        .is_some_and(|v| v != "xMidYMid meet")
                    {
                        result.warnings.insert(
                            "Custom preserveAspectRatio was approximated as centered meet.".into(),
                        );
                    }
                }
                let style = &mut current.style;
                for (key, value) in &a {
                    match key.as_str() {
                        "fill" => style.fill = value.clone(),
                        "stroke" => style.stroke = value.clone(),
                        "stroke-width" => style.width = number(value)?.max(0.),
                        "opacity" => {
                            style.opacity *= value
                                .parse::<f32>()
                                .map_err(|_| "Invalid opacity")?
                                .clamp(0., 1.)
                        }
                        "fill-opacity" => {
                            style.fill_opacity = value
                                .parse::<f32>()
                                .map_err(|_| "Invalid opacity")?
                                .clamp(0., 1.)
                        }
                        "stroke-opacity" => {
                            style.stroke_opacity = value
                                .parse::<f32>()
                                .map_err(|_| "Invalid opacity")?
                                .clamp(0., 1.)
                        }
                        "stroke-linecap" => {
                            style.cap = match value.as_str() {
                                "round" => crate::features::Cap::Round,
                                "square" => crate::features::Cap::Square,
                                _ => crate::features::Cap::Flat,
                            }
                        }
                        "fill-rule" => style.evenodd = value == "evenodd",
                        "display" => style.hidden |= value == "none",
                        "visibility" => style.hidden |= value == "hidden" || value == "collapse",
                        "filter" | "clip-path" | "mask" | "stroke-dasharray" | "class"
                        | "vector-effect" => {
                            if value != "none" {
                                result
                                    .warnings
                                    .insert(format!("SVG {key} is unsupported or approximated."));
                            }
                        }
                        _ => {}
                    }
                }
                if !current.skip && !style.hidden {
                    if matches!(
                        name.as_str(),
                        "path" | "rect" | "circle" | "ellipse" | "polygon" | "polyline" | "line"
                    ) {
                        let mut marks = shape(&name, &a)?;
                        geometry = geometry
                            .saturating_add(marks.iter().map(|m| m.nodes.len()).sum::<usize>());
                        if geometry > 200000 || marks.len() > 256 {
                            return Err("SVG exceeds the editable geometry budget".into());
                        }
                        let bounds = marks.iter().fold(Rect::NOTHING, |r, m| {
                            r.union(crate::vector::control_bounds(m))
                        });
                        let gradient = gradient(
                            &style.fill,
                            &gradients,
                            bounds,
                            style.opacity * style.fill_opacity,
                            &mut result.warnings,
                        );
                        let fill = if gradient.is_some() {
                            Some([255; 4])
                        } else {
                            color(
                                &style.fill,
                                style.opacity * style.fill_opacity,
                                &mut result.warnings,
                            )
                        };
                        let stroke = color(
                            &style.stroke,
                            style.opacity * style.stroke_opacity,
                            &mut result.warnings,
                        );
                        for mark in &mut marks {
                            mark.fill = fill;
                            mark.stroke = stroke.is_some() && style.width > 0.;
                            mark.color = stroke.unwrap_or([0; 4]);
                            mark.width = style.width.clamp(0.01, 100.);
                            mark.style.gradient = gradient.clone();
                            mark.style.start_cap = style.cap;
                            mark.style.end_cap = style.cap;
                        }
                        if marks.len() > 1 && fill.is_some() {
                            marks = compound(marks, style.evenodd, &mut result.warnings);
                        }
                        for mark in marks {
                            if mark.fill.is_none() && !mark.stroke {
                                continue;
                            }
                            let m = current.matrix;
                            let mut mark = crate::vector::transform(&mark, |p| rig::point(m, p));
                            let width = mark.width * (m[0] * m[3] - m[1] * m[2]).abs().sqrt();
                            if width <= 1e-8 {
                                continue;
                            }
                            if width > 100. {
                                result.warnings.insert(
                                    "Stroke widths over 100 document pixels were limited to 100."
                                        .into(),
                                );
                            }
                            mark.width = width.clamp(0.01, 100.);
                            if (Vec2::new(m[0], m[1]).length() - Vec2::new(m[2], m[3]).length())
                                .abs()
                                > 0.01
                                && mark.stroke
                            {
                                result.warnings.insert(
                                    "Non-uniformly transformed stroke widths were approximated."
                                        .into(),
                                );
                            }
                            mark.style.track = crate::features::id();
                            result.marks.push(mark);
                        }
                    } else if !matches!(name.as_str(), "svg" | "g" | "title" | "desc") {
                        result.warnings.insert(format!("SVG element <{name}> was skipped. Convert text and special objects to paths in the source editor."));
                    }
                }
                if result.marks.len() > 10000 {
                    return Err("SVG exceeds 10000 paths".into());
                }
                stack.push(current);
            }
            XmlEvent::EndElement { .. } => {
                stack.pop().ok_or("Invalid SVG nesting")?;
            }
            _ => {}
        }
    }
    if !root || result.marks.is_empty() {
        return Err("No supported visible vector artwork found in SVG".into());
    }
    let mut doc = crate::Document::default();
    doc.layers[0].frames[0] = result.marks.clone().into();
    doc.validate()?;
    Ok(result)
}
impl Editor {
    pub(crate) fn import_svg(&mut self, ctx: &egui::Context) {
        if self.vector_import.is_some() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Scalable Vector Graphics", &["svg"])
            .pick_file()
        else {
            return;
        };
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            let result = (|| {
                let mut bytes = String::new();
                std::fs::File::open(path)
                    .map_err(|e| e.to_string())?
                    .take(LIMIT as u64 + 1)
                    .read_to_string(&mut bytes)
                    .map_err(|e| e.to_string())?;
                parse(&bytes)
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.vector_import = Some(Dialog {
            owner: self.recovery.clone(),
            name,
            job: rx,
            result: None,
            fit: true,
        });
    }
    pub(crate) fn svg_import_ui(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.vector_import.take() else {
            return;
        };
        if dialog.result.is_none() {
            match dialog.job.try_recv() {
                Ok(result) => dialog.result = Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    dialog.result = Some(Err("SVG import interrupted".into()))
                }
                _ => {}
            }
        }
        let mut open = true;
        let mut apply = false;
        egui::Window::new("Import SVG")
            .open(&mut open)
            .collapsible(false)
            .default_width(480.)
            .show(ctx, |ui| {
                ui.heading(&dialog.name);
                match &dialog.result {
                    None => {
                        ui.spinner();
                        ui.label("Reading vector artwork…");
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    Some(Err(error)) => {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                    Some(Ok(result)) => {
                        ui.label(format!("{} editable vector paths", result.marks.len()));
                        ui.checkbox(&mut dialog.fit, "Fit artwork to stage");
                        ui.label("Artwork will be added on a new vector layer.");
                        egui::ScrollArea::vertical()
                            .max_height(200.)
                            .show(ui, |ui| {
                                for warning in &result.warnings {
                                    ui.label(format!("• {warning}"));
                                }
                            });
                        let same = dialog.owner == self.recovery && !self.hub.home;
                        if !same {
                            ui.label("Return to the original document to finish this import.");
                        }
                        apply = crate::design::button(
                            ui,
                            crate::design::Icon::Open,
                            "Import vectors",
                            "",
                            false,
                            same,
                        )
                        .clicked();
                    }
                }
            });
        if apply && let Some(Ok(result)) = dialog.result.take() {
            self.insert_svg(result, dialog.name, dialog.fit);
            return;
        }
        if open {
            self.vector_import = Some(dialog);
        }
    }
    pub(crate) fn insert_svg(&mut self, mut result: Imported, name: String, fit: bool) {
        if self.doc.layers.len() >= 100 {
            self.status = "SVG import requires a free layer slot (maximum 100 layers).".into();
            return;
        }
        if fit {
            let bounds = result.marks.iter().fold(Rect::NOTHING, |r, m| {
                r.union(crate::vector::control_bounds(m))
            });
            if bounds.is_finite() {
                let center = Pos2::new(self.doc.width as f32 / 2., self.doc.height as f32 / 2.);
                let scale = (self.doc.width as f32 * 0.9 / bounds.width().max(1.))
                    .min(self.doc.height as f32 * 0.9 / bounds.height().max(1.));
                result.marks = result
                    .marks
                    .iter()
                    .map(|m| {
                        let mut m =
                            crate::vector::transform(m, |p| center + (p - bounds.center()) * scale);
                        m.width = (m.width * scale).clamp(0.01, 100.);
                        m
                    })
                    .collect();
            }
        }
        self.finish_pending_pen();
        self.checkpoint();
        let count = self.doc.layers[0].frames.len();
        let cel: SharedVec<Mark> = result.marks.into();
        let mut layer = crate::document::Layer {
            name,
            visible: true,
            frames: vec![SharedVec::default(); count].into(),
            options: Default::default(),
        };
        for f in self.frame..count {
            layer.frames[f] = cel.clone();
        }
        layer.options.keys = (0..count)
            .map(|f| f == 0 || f == self.frame)
            .collect::<Vec<_>>()
            .into();
        self.doc.layers.push(layer);
        self.layer = self.doc.layers.len() - 1;
        self.canvas.selected = (0..cel.len()).map(|m| (self.layer, m)).collect();
        self.status = format!(
            "Imported {} editable SVG paths · {} compatibility notices",
            cel.len(),
            result.warnings.len()
        );
        self.recover();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn svg_curves_transforms_opacity_and_arcs_remain_editable_after_save() {
        let imported=parse(r##"<svg xmlns="http://www.w3.org/2000/svg"><g transform="translate(10 20) scale(2)" fill="#ff0000" fill-opacity="0.5" stroke="blue" stroke-width="2"><path d="M0 0 C0 10 10 10 10 0 Z"/><ellipse cx="30" cy="20" rx="10" ry="5"/><rect x="50" y="10" width="20" height="20" rx="3"/></g></svg>"##).unwrap();
        assert_eq!(imported.marks.len(), 3);
        let mark = &imported.marks[0];
        assert_eq!(mark.nodes[0].position, [10., 20.]);
        assert_eq!(mark.nodes[0].out_handle, [10., 40.]);
        assert_eq!(mark.fill, Some([255, 0, 0, 128]));
        assert_eq!(mark.width, 4.);
        assert!(imported.marks.iter().all(|m| !m.nodes.is_empty()));
        let mut doc = crate::Document::default();
        doc.layers[0].frames[0] = imported.marks.into();
        doc.validate().unwrap();
        let bytes = crate::native_project::encode(&doc).unwrap();
        let restored = crate::native_project::decode_if_native(&bytes)
            .unwrap()
            .unwrap();
        assert!(doc.layers[0].frames[0] == restored.layers[0].frames[0]);
    }
    #[test]
    fn compound_holes_and_linked_inkscape_gradients_render() {
        let imported=parse(r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><defs><linearGradient id="colors"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><linearGradient id="paint" xlink:href="#colors"/></defs><path fill="url(#paint)" fill-rule="evenodd" d="M0 0H100V100H0Z M25 25H75V75H25Z"/></svg>"##).unwrap();
        assert_eq!(imported.marks.len(), 1);
        let mark = &imported.marks[0];
        assert!(!mark.holes.is_empty());
        assert!(mark.style.gradient.is_some());
        assert!(crate::vector::contains_fill(mark, Pos2::new(10., 10.)));
        assert!(!crate::vector::contains_fill(mark, Pos2::new(50., 50.)));
        let ctx = egui::Context::default();
        let mut doc = crate::Document::default();
        doc.width = 100;
        doc.height = 100;
        doc.layers[0].frames[0] = imported.marks.into();
        let pixels = crate::exporter::raster(&doc, 0, 100, 100, &ctx, &mut Default::default());
        assert_ne!(
            &pixels[(10 * 100 + 10) * 3..(10 * 100 + 10) * 3 + 3],
            &pixels[(10 * 100 + 90) * 3..(10 * 100 + 90) * 3 + 3]
        );
        assert_eq!(
            &pixels[(50 * 100 + 50) * 3..(50 * 100 + 50) * 3 + 3],
            &[255; 3]
        );
    }
    #[test]
    fn svg_import_rejects_entities_and_preserves_undo() {
        assert!(parse("<!DOCTYPE svg><svg/>").is_err());
        assert!(parse("<svg><path d='M0 0 Cnan 2 3 4 5 6'/></svg>").is_err());
        let result =
            parse("<svg><script>ignored</script><rect width='10' height='10'/></svg>").unwrap();
        assert!(result.warnings.iter().any(|s| s.contains("script")));
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        e.doc = crate::fresh_document();
        let before = serde_json::to_value(&e.doc).unwrap();
        e.insert_svg(result, "SVG art".into(), true);
        e.doc.validate().unwrap();
        assert_eq!(e.undo.len(), 1);
        e.history(false);
        assert_eq!(serde_json::to_value(&e.doc).unwrap(), before);
    }
}
