//! Width is sampled by normalized arc length; the editable centerline remains
//! untouched. Variable strokes are outlined only for display and interchange.
use crate::{document::Mark, shared::SharedVec, vector};
use eframe::egui::{Pos2, Vec2};
use geo::{BooleanOps, Buffer, Coord, LineString, MultiPolygon, Polygon};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WidthStop {
    pub t: f32,
    pub width: f32,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Preset {
    #[default]
    Uniform,
    Tapered,
    Start,
    End,
    Bulge,
    Wave,
    Custom,
}
impl Preset {
    pub const ALL: [Self; 7] = [
        Self::Uniform,
        Self::Tapered,
        Self::Start,
        Self::End,
        Self::Bulge,
        Self::Wave,
        Self::Custom,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Uniform => "Uniforme",
            Self::Tapered => "Puntas finas",
            Self::Start => "De fino a grueso",
            Self::End => "De grueso a fino",
            Self::Bulge => "Centro ancho",
            Self::Wave => "Ondulado",
            Self::Custom => "Personalizado",
        }
    }
    pub fn stops(self) -> SharedVec<WidthStop> {
        let values: &[(f32, f32)] = match self {
            Self::Uniform => &[],
            Self::Tapered => &[(0., 0.), (0.5, 1.), (1., 0.)],
            Self::Start => &[(0., 0.03), (1., 1.)],
            Self::End => &[(0., 1.), (1., 0.03)],
            Self::Bulge => &[
                (0., 0.15),
                (0.25, 0.65),
                (0.5, 1.6),
                (0.75, 0.65),
                (1., 0.15),
            ],
            Self::Wave => &[
                (0., 0.2),
                (0.2, 1.25),
                (0.4, 0.35),
                (0.6, 1.25),
                (0.8, 0.35),
                (1., 0.2),
            ],
            Self::Custom => &[(0., 1.), (1., 1.)],
        };
        values
            .iter()
            .map(|&(t, width)| WidthStop { t, width })
            .collect()
    }
}
pub fn sample(stops: &[WidthStop], t: f32) -> f32 {
    if stops.is_empty() {
        return 1.;
    }
    if t <= stops[0].t {
        return stops[0].width;
    }
    for pair in stops.windows(2) {
        if t <= pair[1].t {
            let f = ((t - pair[0].t) / (pair[1].t - pair[0].t).max(1e-6)).clamp(0., 1.);
            let f = f * f * (3. - 2. * f);
            return pair[0].width + (pair[1].width - pair[0].width) * f;
        }
    }
    stops.last().unwrap().width
}
pub fn edit_stop(stops: &mut SharedVec<WidthStop>, t: f32, width: f32) -> usize {
    if stops.is_empty() {
        *stops = Preset::Custom.stops();
    }
    let t = t.clamp(0., 1.);
    let width = width.clamp(0., 4.);
    let closest = stops
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (a.t - t).abs().total_cmp(&(b.t - t).abs()))
        .unwrap()
        .0;
    if (stops[closest].t - t).abs() < 0.015 || stops.len() >= 128 {
        stops[closest].width = width;
        return closest;
    }
    let at = stops.partition_point(|s| s.t < t);
    stops.insert(at, WidthStop { t, width });
    at
}
pub fn validate(stops: &[WidthStop]) -> bool {
    stops.len() <= 128
        && stops.iter().all(|s| {
            s.t.is_finite()
                && s.width.is_finite()
                && (0.0..=1.).contains(&s.t)
                && (0.0..=4.).contains(&s.width)
        })
        && stops.windows(2).all(|p| p[0].t < p[1].t)
}
pub struct Projection {
    pub t: f32,
    pub point: Pos2,
    pub normal: Vec2,
    pub distance: f32,
}
pub fn lengths(points: &[Pos2]) -> (Vec<f32>, f32) {
    let mut lengths = vec![0.; points.len()];
    for i in 1..points.len() {
        lengths[i] = lengths[i - 1] + points[i].distance(points[i - 1]);
    }
    let total = lengths.last().copied().unwrap_or(0.);
    (lengths, total)
}
pub fn nearest(mark: &Mark, p: Pos2) -> Option<Projection> {
    let points = vector::flatten(mark, 0.08);
    let (lengths, total) = lengths(&points);
    let mut best: Option<Projection> = None;
    for (i, pair) in points.windows(2).enumerate() {
        let delta = pair[1] - pair[0];
        let length = delta.length();
        if length < 1e-6 {
            continue;
        }
        let u = ((p - pair[0]).dot(delta) / delta.length_sq()).clamp(0., 1.);
        let point = pair[0] + delta * u;
        let distance = p.distance(point);
        if best.as_ref().is_none_or(|b| distance < b.distance) {
            best = Some(Projection {
                t: (lengths[i] + length * u) / total.max(1e-6),
                point,
                normal: Vec2::new(-delta.y, delta.x) / length,
                distance,
            });
        }
    }
    best
}
pub fn at_length(mark: &Mark, t: f32) -> Option<(Pos2, Vec2)> {
    let points = vector::flatten(mark, 0.08);
    let (lengths, total) = lengths(&points);
    let target = t.clamp(0., 1.) * total;
    for (i, pair) in points.windows(2).enumerate() {
        if lengths[i + 1] >= target {
            let delta = pair[1] - pair[0];
            let length = delta.length();
            if length < 1e-6 {
                continue;
            }
            return Some((
                pair[0] + delta * ((target - lengths[i]) / length).clamp(0., 1.),
                Vec2::new(-delta.y, delta.x) / length,
            ));
        }
    }
    None
}
fn line(points: impl IntoIterator<Item = Pos2>) -> LineString<f64> {
    LineString::new(
        points
            .into_iter()
            .map(|p| Coord {
                x: p.x as f64,
                y: p.y as f64,
            })
            .collect(),
    )
}
fn arc(out: &mut Vec<Pos2>, center: Pos2, angle: f32, radius: f32) {
    let steps = (std::f32::consts::PI * (radius / 0.12).sqrt())
        .ceil()
        .clamp(8., 128.) as usize;
    for i in 1..=steps {
        let a = angle - std::f32::consts::PI * i as f32 / steps as f32;
        out.push(center + Vec2::angled(a) * radius);
    }
}
pub fn outline(mark: &Mark) -> MultiPolygon<f64> {
    let raw = vector::flatten(mark, 0.06);
    if raw.len() == 1 {
        return geo::Point::new(raw[0].x as f64, raw[0].y as f64)
            .buffer((mark.width * sample(&mark.width_profile, 0.5) * 0.5) as f64);
    }
    let (raw_lengths, total) = lengths(&raw);
    if raw.len() < 2 || total < 1e-6 {
        return MultiPolygon(vec![]);
    }
    let mut stops: Vec<f32> = raw_lengths.iter().map(|l| l / total).collect();
    let mut control = vec![WidthStop {
        t: 0.,
        width: sample(&mark.width_profile, 0.),
    }];
    control.extend(
        mark.width_profile
            .iter()
            .copied()
            .filter(|s| s.t > 0. && s.t < 1.),
    );
    control.push(WidthStop {
        t: 1.,
        width: sample(&mark.width_profile, 1.),
    });
    for pair in control.windows(2) {
        let steps = ((pair[1].width - pair[0].width).abs() * mark.width * 0.5 / 0.12)
            .sqrt()
            .ceil()
            .clamp(4., 64.) as usize;
        for i in 0..=steps {
            stops.push(pair[0].t + (pair[1].t - pair[0].t) * i as f32 / steps as f32);
        }
    }
    stops.sort_by(f32::total_cmp);
    stops.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let mut segment = 0;
    let points: Vec<_> = stops
        .into_iter()
        .map(|t| {
            let distance = t * total;
            while segment + 2 < raw.len() && raw_lengths[segment + 1] < distance {
                segment += 1;
            }
            let f = ((distance - raw_lengths[segment])
                / (raw_lengths[segment + 1] - raw_lengths[segment]).max(1e-8))
            .clamp(0., 1.);
            raw[segment].lerp(raw[segment + 1], f)
        })
        .collect();
    let (lengths, total) = lengths(&points);
    if points.len() < 2 || total < 1e-6 {
        return MultiPolygon(vec![]);
    }
    let closed = mark.closed || mark.rectangle;
    let duplicate = closed && points.first() == points.last();
    let n = points.len() - usize::from(duplicate);
    if n < 2 {
        return MultiPolygon(vec![]);
    }
    let mut left = vec![];
    let mut right = vec![];
    for i in 0..n {
        let previous = if i > 0 {
            i - 1
        } else if closed {
            n - 1
        } else {
            0
        };
        let next = if i + 1 < n {
            i + 1
        } else if closed {
            0
        } else {
            n - 1
        };
        let mut a = (points[i] - points[previous]).normalized();
        let mut b = (points[next] - points[i]).normalized();
        if a == Vec2::ZERO {
            a = b;
        }
        if b == Vec2::ZERO {
            b = a;
        }
        let normal_a = Vec2::new(-a.y, a.x);
        let normal_b = Vec2::new(-b.y, b.x);
        let mut normal = (normal_a + normal_b).normalized();
        if normal == Vec2::ZERO {
            normal = normal_b;
        }
        let miter = (1. / normal.dot(normal_b).abs().max(0.4)).min(2.5);
        let radius = mark.width * sample(&mark.width_profile, lengths[i] / total) * 0.5;
        left.push(points[i] + normal * radius * miter);
        right.push(points[i] - normal * radius * miter);
    }
    let poly = if closed {
        left.push(left[0]);
        right.push(right[0]);
        let area = |r: &Vec<Pos2>| {
            r.windows(2)
                .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
                .sum::<f32>()
                .abs()
        };
        if area(&left) < area(&right) {
            std::mem::swap(&mut left, &mut right);
        }
        let signed = |r: &Vec<Pos2>| {
            r.windows(2)
                .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
                .sum::<f32>()
        };
        if signed(&left) * signed(&right) > 0. {
            right.reverse();
        }
        Polygon::new(line(left), vec![line(right)])
    } else {
        let mut ring = left;
        let last = n - 1;
        let end_dir = (points[last] - points[last - 1]).normalized();
        let end_normal = Vec2::new(-end_dir.y, end_dir.x);
        arc(
            &mut ring,
            points[last],
            end_normal.y.atan2(end_normal.x),
            mark.width * sample(&mark.width_profile, 1.) * 0.5,
        );
        ring.extend(right.into_iter().rev().skip(1));
        let start_dir = (points[1] - points[0]).normalized();
        let start_normal = Vec2::new(-start_dir.y, start_dir.x);
        arc(
            &mut ring,
            points[0],
            (-start_normal.y).atan2(-start_normal.x),
            mark.width * sample(&mark.width_profile, 0.) * 0.5,
        );
        if ring.first() != ring.last() {
            ring.push(ring[0]);
        }
        Polygon::new(line(ring), vec![])
    };
    poly.union_with_fill_rule(
        &MultiPolygon::<f64>(vec![]),
        geo::algorithm::bool_ops::FillRule::NonZero,
    )
}
pub fn export_marks(mark: &Mark) -> Vec<Mark> {
    if !mark.stroke || mark.width_profile.is_empty() {
        return vec![mark.clone()];
    }
    let mut output = vec![];
    if mark.fill.is_some() {
        let mut fill = mark.clone();
        fill.stroke = false;
        fill.width_profile = Default::default();
        output.push(fill);
    }
    output.extend(vector::polygons_to_marks(outline(mark), mark.color));
    output
}
/// Preserve the same width distribution when a stroke is cut into a subpath.
pub fn remap(original: &Mark, piece: &mut Mark) {
    if original.width_profile.is_empty() {
        return;
    }
    let original_points = vector::flatten(original, 0.04);
    let (_, total) = lengths(&original_points);
    let piece_points = vector::flatten(piece, 0.04);
    let (_, length) = lengths(&piece_points);
    let Some(first) = piece_points.first() else {
        return;
    };
    let Some(start) = nearest(original, *first) else {
        return;
    };
    let span = (length / total.max(1e-6)).clamp(1e-6, 1.);
    let end = start.t + span;
    let value = |t: f32| sample(&original.width_profile, if t > 1. { t - 1. } else { t });
    let mut profile = vec![WidthStop {
        t: 0.,
        width: value(start.t),
    }];
    for cycle in 0..=1 {
        for stop in &original.width_profile {
            let t = stop.t + cycle as f32;
            if t > start.t + 1e-5 && t < end - 1e-5 {
                profile.push(WidthStop {
                    t: (t - start.t) / span,
                    width: stop.width,
                });
            }
        }
    }
    profile.push(WidthStop {
        t: 1.,
        width: value(end),
    });
    piece.width_profile = profile.into();
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::Area;
    #[test]
    fn presets_and_custom_profile_are_valid_and_interpolate() {
        for preset in Preset::ALL {
            assert!(validate(&preset.stops()));
        }
        let p = Preset::Tapered.stops();
        assert_eq!(sample(&p, 0.), 0.);
        assert_eq!(sample(&p, 0.5), 1.);
        assert_eq!(sample(&p, 1.), 0.);
        let mut p = Preset::Uniform.stops();
        edit_stop(&mut p, 0.3, 2.);
        assert!(validate(&p));
        assert_eq!(sample(&p, 0.3), 2.);
    }
    #[test]
    fn tapered_ink_has_less_area_and_keeps_the_centerline_editable() {
        let mark = Mark {
            points: vec![[0., 0.], [100., 0.]].into(),
            width: 20.,
            width_profile: Preset::Tapered.stops(),
            ..Default::default()
        };
        let area = outline(&mark).unsigned_area();
        assert!(area > 600. && area < 2000., "{area}");
        let exported = export_marks(&mark);
        assert!(
            exported
                .iter()
                .all(|m| m.fill == Some(mark.color) && !m.stroke)
        );
        assert_eq!(mark.points.len(), 2);
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use geo::{Area, Contains};
    #[test]
    fn variable_width_closed_outline_keeps_its_interior_empty() {
        let mark = Mark {
            points: vec![[0., 0.], [100., 100.]].into(),
            rectangle: true,
            closed: true,
            width: 20.,
            width_profile: Preset::Custom.stops(),
            ..Default::default()
        };
        let ink = outline(&mark);
        assert!(!ink.contains(&geo::Point::new(50., 50.)));
        assert!(
            (ink.unsigned_area() - 8000.).abs() < 20.,
            "{}",
            ink.unsigned_area()
        );
    }
    #[test]
    fn cutting_preserves_the_original_width_distribution() {
        let original = Mark {
            points: vec![[0., 0.], [100., 0.]].into(),
            width_profile: Preset::Tapered.stops(),
            ..Default::default()
        };
        let (_, inside) = vector::split_rect(
            &original,
            eframe::egui::Rect::from_min_max(Pos2::new(25., -10.), Pos2::new(75., 10.)),
        );
        assert_eq!(inside.len(), 1);
        let p = &inside[0].width_profile;
        assert!((sample(p, 0.) - sample(&original.width_profile, 0.25)).abs() < 0.01);
        assert!((sample(p, 1.) - sample(&original.width_profile, 0.75)).abs() < 0.01);
        assert!((sample(p, 0.5) - 1.).abs() < 0.01);
    }
    #[test]
    #[ignore = "manual release benchmark"]
    fn benchmark_variable_width_and_bucket() {
        let raw: Vec<_> = (0..2000)
            .map(|i| {
                let x = i as f32 * 0.35;
                [x, (x * 0.02).sin() * 60.]
            })
            .collect();
        let curves = vector::fit(&raw, 1.5);
        let mut mark = vector::from_cubics(
            &curves,
            &Mark {
                width: 12.,
                ..Default::default()
            },
        );
        mark.width_profile = Preset::Wave.stops();
        let start = std::time::Instant::now();
        for _ in 0..100 {
            std::hint::black_box(outline(&mark));
        }
        let stroke = start.elapsed() / 100;
        let rectangle = Mark {
            points: vec![[0., 0.], [400., 300.]].into(),
            rectangle: true,
            closed: true,
            ..Default::default()
        };
        let start = std::time::Instant::now();
        for _ in 0..100 {
            std::hint::black_box(
                crate::bucket::fill(
                    &[rectangle.clone()],
                    Pos2::new(200., 150.),
                    crate::bucket::Gap::None,
                    [0, 0, 0, 255],
                )
                .unwrap(),
            );
        }
        eprintln!(
            "variable outline 2000 samples: {:?}/op (100 runs); bucket rectangle: {:?}/op (100 runs)",
            stroke,
            start.elapsed() / 100
        );
    }
}
