//! Editable cubic paths, adaptive flattening and geometric (not object) cuts.
use crate::document::{Mark, Node};
use eframe::egui::{Pos2, Rect, Vec2};
use geo::{BooleanOps, Buffer, Coord, LineString, MultiPolygon, Polygon};

pub fn pos(p: [f32; 2]) -> Pos2 {
    Pos2::new(p[0], p[1])
}
pub fn xy(p: Pos2) -> [f32; 2] {
    [p.x.clamp(-1e7, 1e7), p.y.clamp(-1e7, 1e7)]
}
#[derive(Clone, Copy, Debug)]
pub struct Cubic(pub [Pos2; 4]);
impl Cubic {
    pub fn at(self, t: f32) -> Pos2 {
        let [a, b, c, d] = self.0;
        let ab = a.lerp(b, t);
        let bc = b.lerp(c, t);
        let cd = c.lerp(d, t);
        ab.lerp(bc, t).lerp(bc.lerp(cd, t), t)
    }
    pub fn split(self, t: f32) -> (Self, Self) {
        let [a, b, c, d] = self.0;
        let ab = a.lerp(b, t);
        let bc = b.lerp(c, t);
        let cd = c.lerp(d, t);
        let abc = ab.lerp(bc, t);
        let bcd = bc.lerp(cd, t);
        let mid = abc.lerp(bcd, t);
        (Self([a, ab, abc, mid]), Self([mid, bcd, cd, d]))
    }
    pub fn sub(self, a: f32, b: f32) -> Self {
        if a <= 0. && b >= 1. {
            return self;
        }
        self.split(b).0.split(if b > 0. { a / b } else { 0. }).1
    }
    fn flat(self, tolerance: f32, depth: u32, out: &mut Vec<Pos2>) {
        let [a, b, c, d] = self.0;
        let chord = d - a;
        let distance = |p: Pos2| {
            let t = if chord.length_sq() > 1e-12 {
                ((p - a).dot(chord) / chord.length_sq()).clamp(0., 1.)
            } else {
                0.
            };
            p.distance(a + chord * t)
        };
        if depth >= 18 || distance(b).max(distance(c)) <= tolerance {
            out.push(d);
        } else {
            let (left, right) = self.split(0.5);
            left.flat(tolerance, depth + 1, out);
            right.flat(tolerance, depth + 1, out);
        }
    }
}
pub fn cubics(mark: &Mark) -> Vec<Cubic> {
    if !mark.nodes.is_empty() {
        let n = mark.nodes.len();
        let count = n.saturating_sub(1) + usize::from(mark.closed && n > 1);
        (0..count)
            .map(|i| {
                let a = &mark.nodes[i];
                let b = &mark.nodes[(i + 1) % n];
                Cubic([
                    pos(a.position),
                    pos(a.out_handle),
                    pos(b.in_handle),
                    pos(b.position),
                ])
            })
            .collect()
    } else {
        let p = if mark.rectangle && mark.points.len() > 1 {
            let a = pos(mark.points[0]);
            let b = pos(*mark.points.last().unwrap());
            vec![a, Pos2::new(b.x, a.y), b, Pos2::new(a.x, b.y), a]
        } else {
            let mut p: Vec<_> = mark.points.iter().copied().map(pos).collect();
            if mark.closed && p.len() > 1 && p.first() != p.last() {
                p.push(p[0]);
            }
            p
        };
        p.windows(2)
            .map(|w| Cubic([w[0], w[0], w[1], w[1]]))
            .collect()
    }
}
pub fn flatten(mark: &Mark, tolerance: f32) -> Vec<Pos2> {
    if mark.nodes.is_empty() && !mark.rectangle {
        let mut p: Vec<_> = mark.points.iter().copied().map(pos).collect();
        if mark.closed && p.len() > 1 && p.first() != p.last() {
            p.push(p[0]);
        }
        return p;
    }
    let curves = cubics(mark);
    let mut out = curves.first().map(|c| vec![c.0[0]]).unwrap_or_else(|| {
        mark.nodes
            .first()
            .map(|n| vec![pos(n.position)])
            .unwrap_or_default()
    });
    for c in curves {
        c.flat(tolerance.max(0.001), 0, &mut out);
    }
    out
}
pub fn control_bounds(mark: &Mark) -> Rect {
    let mut r = Rect::NOTHING;
    for p in &mark.points {
        r.extend_with(pos(*p));
    }
    for n in &mark.nodes {
        for p in [n.position, n.in_handle, n.out_handle] {
            r.extend_with(pos(p));
        }
    }
    r
}
pub fn from_cubics(curves: &[Cubic], style: &Mark) -> Mark {
    let mut out = style.clone();
    out.points = Default::default();
    out.rectangle = false;
    out.holes = Default::default();
    out.closed = false;
    let mut nodes = Vec::new();
    if let Some(first) = curves.first() {
        nodes.push(Node::corner(xy(first.0[0])));
        for c in curves {
            nodes.last_mut().unwrap().out_handle = xy(c.0[1]);
            let mut node = Node::corner(xy(c.0[3]));
            node.in_handle = xy(c.0[2]);
            nodes.push(node);
        }
        if nodes.len() > 2
            && pos(nodes[0].position).distance(pos(nodes.last().unwrap().position)) < 0.0001
        {
            nodes[0].in_handle = nodes.pop().unwrap().in_handle;
            out.closed = true;
        }
    }
    out.nodes = nodes.into();
    out
}
/// Schneider-style least-squares fitting with chord parameters and bounded
/// subdivision. This runs once at stroke completion, not on every input event.
pub fn fit(points: &[[f32; 2]], error: f32) -> Vec<Cubic> {
    let mut p: Vec<Pos2> = Vec::new();
    for v in points {
        let v = pos(*v);
        if p.last().is_none_or(|q| q.distance(v) > 0.001) {
            p.push(v);
        }
    }
    if p.len() < 2 {
        return vec![];
    }
    let mut out = vec![];
    fit_part(
        &p,
        (p[1] - p[0]).normalized(),
        (p[p.len() - 2] - p[p.len() - 1]).normalized(),
        error.max(0.05).powi(2),
        0,
        &mut out,
    );
    out
}
fn fit_part(p: &[Pos2], left: Vec2, right: Vec2, error: f32, depth: u32, out: &mut Vec<Cubic>) {
    let last = p.len() - 1;
    if last == 1 || depth >= 32 {
        for w in p.windows(2) {
            let d = w[0].distance(w[1]) / 3.;
            out.push(Cubic([w[0], w[0] + left * d, w[1] + right * d, w[1]]));
        }
        return;
    }
    let mut u = vec![0.; p.len()];
    for i in 1..p.len() {
        u[i] = u[i - 1] + p[i].distance(p[i - 1]);
    }
    let total = u[last];
    for t in &mut u {
        *t /= total;
    }
    let generate = |u: &[f32]| {
        let mut c00 = 0.;
        let mut c01 = 0.;
        let mut c11 = 0.;
        let mut x0 = 0.;
        let mut x1 = 0.;
        for (&point, &t) in p.iter().zip(u) {
            let b0 = (1. - t).powi(3);
            let b1 = 3. * t * (1. - t).powi(2);
            let b2 = 3. * t * t * (1. - t);
            let b3 = t * t * t;
            let a0 = left * b1;
            let a1 = right * b2;
            let residual =
                point - (Pos2::ZERO + p[0].to_vec2() * (b0 + b1) + p[last].to_vec2() * (b2 + b3));
            c00 += a0.dot(a0);
            c01 += a0.dot(a1);
            c11 += a1.dot(a1);
            x0 += a0.dot(residual);
            x1 += a1.dot(residual);
        }
        let determinant = c00 * c11 - c01 * c01;
        let mut a = if determinant.abs() > 1e-12 {
            (x0 * c11 - x1 * c01) / determinant
        } else {
            0.
        };
        let mut b = if determinant.abs() > 1e-12 {
            (x1 * c00 - x0 * c01) / determinant
        } else {
            0.
        };
        let length = p[0].distance(p[last]);
        if a < length * 1e-4 || b < length * 1e-4 || a > total * 2. || b > total * 2. {
            a = length / 3.;
            b = a;
        }
        Cubic([p[0], p[0] + left * a, p[last] + right * b, p[last]])
    };
    let mut curve = generate(&u);
    let mut split = last / 2;
    for attempt in 0..5 {
        let mut maximum = 0.;
        for i in 1..last {
            let e = curve.at(u[i]).distance_sq(p[i]);
            if e > maximum {
                maximum = e;
                split = i;
            }
        }
        if maximum <= error {
            out.push(curve);
            return;
        }
        if attempt == 4 || maximum > error * 16. {
            break;
        }
        let [a, b, c, d] = curve.0;
        let mut previous = 0.;
        let mut valid = true;
        for i in 1..last {
            let t = u[i];
            let s = 1. - t;
            let first = (b - a) * (3. * s * s) + (c - b) * (6. * s * t) + (d - c) * (3. * t * t);
            let second = ((c - b) - (b - a)) * (6. * s) + ((d - c) - (c - b)) * (6. * t);
            let residual = curve.at(t) - p[i];
            let denom = first.dot(first) + residual.dot(second);
            let next = if denom.abs() > 1e-10 {
                (t - residual.dot(first) / denom).clamp(0., 1.)
            } else {
                t
            };
            if next <= previous || next >= 1. {
                valid = false;
                break;
            }
            u[i] = next;
            previous = next;
        }
        if !valid {
            break;
        }
        curve = generate(&u);
    }
    split = split.clamp(1, last - 1);
    let tangent = (p[split - 1] - p[split + 1]).normalized();
    fit_part(&p[..=split], left, tangent, error, depth + 1, out);
    fit_part(&p[split..], -tangent, right, error, depth + 1, out);
}
fn ring(points: impl IntoIterator<Item = Pos2>) -> LineString<f64> {
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
pub fn polygon(mark: &Mark) -> Polygon<f64> {
    Polygon::new(
        ring(flatten(mark, 0.04)),
        mark.holes
            .iter()
            .map(|h| ring(h.iter().copied().map(pos)))
            .collect(),
    )
}
pub fn polygons_to_marks(polys: MultiPolygon<f64>, color: [u8; 4]) -> Vec<Mark> {
    polys
        .0
        .into_iter()
        .filter(|p| p.exterior().0.len() >= 4)
        .map(|p| {
            let points = p
                .exterior()
                .0
                .iter()
                .map(|p| [p.x as f32, p.y as f32])
                .collect();
            let holes = p
                .interiors()
                .iter()
                .map(|r| r.0.iter().map(|p| [p.x as f32, p.y as f32]).collect())
                .collect();
            Mark {
                points,
                fill: Some(color),
                stroke: false,
                closed: true,
                holes,
                ..Mark::default()
            }
        })
        .collect()
}
pub fn brush_fill(center: &Mark, color: [u8; 4]) -> Vec<Mark> {
    let points = flatten(center, 0.06);
    let polys = if points.len() == 1 {
        geo::Point::new(points[0].x as f64, points[0].y as f64).buffer(center.width as f64 / 2.)
    } else {
        ring(points).buffer(center.width as f64 / 2.)
    };
    polygons_to_marks(polys, color)
}
pub fn inside_ring(points: &[Pos2], p: Pos2) -> bool {
    let mut inside = false;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}
pub fn contains_fill(mark: &Mark, p: Pos2) -> bool {
    mark.fill.is_some()
        && inside_ring(&flatten(mark, 0.15), p)
        && !mark
            .holes
            .iter()
            .any(|h| inside_ring(&h.iter().copied().map(pos).collect::<Vec<_>>(), p))
}
fn axis_roots(curve: Cubic, axis: usize, boundary: f32) -> Vec<f32> {
    let q = curve
        .0
        .map(|p| if axis == 0 { p.x as f64 } else { p.y as f64 });
    let a = -q[0] + 3. * q[1] - 3. * q[2] + q[3];
    let b = 3. * q[0] - 6. * q[1] + 3. * q[2];
    let c = 3. * (q[1] - q[0]);
    let d = q[0] - boundary as f64;
    let eval = |t: f64| ((a * t + b) * t + c) * t + d;
    let mut cuts = vec![0., 1.];
    if a.abs() < 1e-12 {
        if b.abs() > 1e-12 {
            let t = -c / (2. * b);
            if t > 0. && t < 1. {
                cuts.push(t);
            }
        }
    } else {
        let disc = 4. * b * b - 12. * a * c;
        if disc >= 0. {
            for t in [
                (-2. * b - disc.sqrt()) / (6. * a),
                (-2. * b + disc.sqrt()) / (6. * a),
            ] {
                if t > 0. && t < 1. {
                    cuts.push(t);
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    let mut roots = vec![];
    for &t in &cuts {
        if eval(t).abs() < 1e-8 {
            roots.push(t as f32);
        }
    }
    for pair in cuts.windows(2) {
        let (mut l, mut r) = (pair[0], pair[1]);
        let sign = eval(l);
        if sign * eval(r) >= 0. {
            continue;
        }
        for _ in 0..48 {
            let m = (l + r) * 0.5;
            if eval(m) * sign > 0. {
                l = m;
            } else {
                r = m;
            }
        }
        roots.push(((l + r) * 0.5) as f32);
    }
    roots
}
fn clip_stroke(mark: &Mark, rect: Rect) -> (Vec<Mark>, Vec<Mark>) {
    let mut groups: Vec<(bool, Vec<Cubic>)> = vec![];
    for curve in cubics(mark) {
        let mut cuts = vec![0., 1.];
        for (axis, bound) in [
            (0, rect.left()),
            (0, rect.right()),
            (1, rect.top()),
            (1, rect.bottom()),
        ] {
            cuts.extend(axis_roots(curve, axis, bound));
        }
        cuts.sort_by(f32::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for pair in cuts.windows(2) {
            if pair[1] - pair[0] < 1e-6 {
                continue;
            }
            let inside = rect.contains(curve.at((pair[0] + pair[1]) * 0.5));
            let segment = curve.sub(pair[0], pair[1]);
            if let Some((kind, list)) = groups.last_mut() {
                if *kind == inside {
                    list.push(segment);
                    continue;
                }
            }
            groups.push((inside, vec![segment]));
        }
    }
    if groups.len() > 1
        && groups.first().unwrap().0 == groups.last().unwrap().0
        && (mark.closed || mark.rectangle)
    {
        let mut last = groups.pop().unwrap().1;
        last.extend(groups[0].1.iter().copied());
        groups[0].1 = last;
    }
    let mut outside = vec![];
    let mut inside = vec![];
    for (selected, curves) in groups {
        let mut m = from_cubics(&curves, mark);
        crate::width::remap(mark, &mut m);
        m.fill = None;
        if selected {
            inside.push(m)
        } else {
            outside.push(m)
        }
    }
    if cubics(mark).is_empty() && !mark.points.is_empty() {
        if rect.contains(pos(mark.points[0])) {
            inside.push(mark.clone())
        } else {
            outside.push(mark.clone())
        }
    }
    (outside, inside)
}
/// Cut fills by area, strokes by exact cubic subsegments. Newly cut fill edges
/// do not acquire a contour. Fully contained paths retain their original nodes.
pub fn split_rect(mark: &Mark, rect: Rect) -> (Vec<Mark>, Vec<Mark>) {
    let bounds = control_bounds(mark);
    if !bounds.intersects(rect) {
        return (vec![mark.clone()], vec![]);
    }
    if rect.contains_rect(bounds) {
        return (vec![], vec![mark.clone()]);
    }
    if mark.style.instance.is_some() {
        return (vec![], vec![mark.clone()]);
    }
    let mut outside = vec![];
    let mut inside = vec![];
    if let Some(color) = mark.fill {
        let area = polygon(mark);
        let cutter = Polygon::new(
            ring([
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
                rect.left_top(),
            ]),
            vec![],
        );
        outside.extend(crate::features::styled(
            area.difference(&cutter),
            color,
            &mark.style,
        ));
        inside.extend(crate::features::styled(
            area.intersection(&cutter),
            color,
            &mark.style,
        ));
    }
    if mark.stroke {
        let (a, b) = clip_stroke(mark, rect);
        outside.extend(a);
        inside.extend(b);
    }
    (outside, inside)
}
pub fn transform(mark: &Mark, mut f: impl FnMut(Pos2) -> Pos2) -> Mark {
    let mut out = mark.clone();
    if let Some(g) = out.style.gradient.as_mut() {
        g.start = xy(f(pos(g.start)));
        g.end = xy(f(pos(g.end)));
    }
    if let Some(i) = out.style.instance.as_mut() {
        let origin = f(Pos2::new(i.matrix[4], i.matrix[5]));
        let x = f(Pos2::new(
            i.matrix[4] + i.matrix[0],
            i.matrix[5] + i.matrix[1],
        )) - origin;
        let y = f(Pos2::new(
            i.matrix[4] + i.matrix[2],
            i.matrix[5] + i.matrix[3],
        )) - origin;
        i.matrix = [x.x, x.y, y.x, y.y, origin.x, origin.y];
    }
    if mark.rectangle {
        out.points = flatten(mark, 0.1).into_iter().map(|p| xy(f(p))).collect();
        out.rectangle = false;
        out.closed = true;
    } else {
        out.points = mark.points.iter().map(|p| xy(f(pos(*p)))).collect();
    }
    out.nodes = mark
        .nodes
        .iter()
        .map(|n| Node {
            position: xy(f(pos(n.position))),
            in_handle: xy(f(pos(n.in_handle))),
            out_handle: xy(f(pos(n.out_handle))),
        })
        .collect();
    out.holes = mark
        .holes
        .iter()
        .map(|r| r.iter().map(|p| xy(f(pos(*p)))).collect())
        .collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::Area;
    fn rectangle() -> Mark {
        Mark {
            points: vec![[0., 0.], [100., 100.]].into(),
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some([90, 50, 200, 255]),
            ..Default::default()
        }
    }
    #[test]
    fn selecting_middle_of_fill_leaves_real_hole_and_conserves_area() {
        let mark = rectangle();
        let rect = Rect::from_min_max(Pos2::new(30., 30.), Pos2::new(70., 70.));
        let (outside, inside) = split_rect(&mark, rect);
        assert_eq!(outside.len(), 1);
        assert_eq!(inside.len(), 1);
        assert_eq!(outside[0].holes.len(), 1);
        assert!(!contains_fill(&outside[0], Pos2::new(50., 50.)));
        assert!(contains_fill(&inside[0], Pos2::new(50., 50.)));
        let area: f64 = outside
            .iter()
            .chain(&inside)
            .map(|m| polygon(m).unsigned_area())
            .sum();
        assert!((area - 10_000.).abs() < 0.01);
        assert!(outside.iter().chain(&inside).all(|m| !m.stroke));
        let moved = transform(&inside[0], |p| p + Vec2::new(200., 0.));
        assert!(contains_fill(&moved, Pos2::new(250., 50.)));
        assert!(!contains_fill(&outside[0], Pos2::new(50., 50.)));
    }
    #[test]
    fn cutting_cubic_preserves_control_handles_and_exact_curve_pieces() {
        let curve = Cubic([
            Pos2::new(-30., 0.),
            Pos2::new(0., 100.),
            Pos2::new(100., 100.),
            Pos2::new(130., 0.),
        ]);
        let mark = from_cubics(&[curve], &Mark::default());
        let rect = Rect::from_min_max(Pos2::new(20., -10.), Pos2::new(80., 100.));
        let (outside, inside) = split_rect(&mark, rect);
        assert_eq!(outside.len(), 2);
        assert_eq!(inside.len(), 1);
        assert_eq!(inside[0].nodes.len(), 2);
        assert_ne!(inside[0].nodes[0].out_handle, inside[0].nodes[0].position);
        for p in flatten(&inside[0], 0.02) {
            assert!(rect.expand(0.001).contains(p));
        }
        let start = pos(inside[0].nodes[0].position);
        let end = pos(inside[0].nodes[1].position);
        assert!((start.x - 20.).abs() < 0.001);
        assert!((end.x - 80.).abs() < 0.001);
        for m in outside.iter().chain(&inside) {
            for p in flatten(m, 0.05) {
                let error = (0..=4000)
                    .map(|i| curve.at(i as f32 / 4000.).distance(p))
                    .fold(f32::INFINITY, f32::min);
                assert!(error < 0.06, "distance={error}");
            }
        }
    }
    #[test]
    fn fitted_brush_has_continuous_tangents_and_fewer_editable_nodes() {
        let raw: Vec<_> = (0..400)
            .map(|i| {
                let x = i as f32;
                [x, (x * 0.02).sin() * 45. + (x * 0.7).sin() * 0.4]
            })
            .collect();
        let curves = fit(&raw, 1.);
        assert!(curves.len() < 30, "curves={}", curves.len());
        for pair in curves.windows(2) {
            assert_eq!(pair[0].0[3], pair[1].0[0]);
            let a = (pair[0].0[3] - pair[0].0[2]).normalized();
            let b = (pair[1].0[1] - pair[1].0[0]).normalized();
            assert!(a.dot(b) > 0.999);
        }
        let mark = from_cubics(&curves, &Mark::default());
        let flat = flatten(&mark, 0.02);
        for raw in raw {
            let p = pos(raw);
            let error = flat
                .windows(2)
                .map(|w| {
                    let d = w[1] - w[0];
                    let t = ((p - w[0]).dot(d) / d.length_sq().max(1e-12)).clamp(0., 1.);
                    p.distance(w[0] + d * t)
                })
                .fold(f32::INFINITY, f32::min);
            assert!(error < 1.1, "error={error}");
        }
    }
    #[test]
    fn classic_brush_is_fill_geometry_and_vector_brush_keeps_centerline() {
        let center = from_cubics(
            &[Cubic([
                Pos2::ZERO,
                Pos2::new(30., 30.),
                Pos2::new(60., -30.),
                Pos2::new(90., 0.),
            ])],
            &Mark {
                width: 12.,
                ..Default::default()
            },
        );
        let shapes = brush_fill(&center, [255, 40, 50, 255]);
        assert!(!shapes.is_empty());
        assert!(
            shapes
                .iter()
                .all(|m| m.fill == Some([255, 40, 50, 255]) && !m.stroke && m.closed)
        );
        assert!(shapes.iter().any(|m| contains_fill(m, Pos2::new(45., 0.))));
        assert!(center.stroke && center.fill.is_none());
    }
    #[test]
    fn transforming_curves_moves_anchors_handles_and_holes_without_flattening() {
        let mut mark = from_cubics(
            &[Cubic([
                Pos2::ZERO,
                Pos2::new(10., 20.),
                Pos2::new(30., 40.),
                Pos2::new(50., 60.),
            ])],
            &Mark::default(),
        );
        mark.holes = vec![vec![[1., 2.], [3., 4.], [5., 6.]].into()].into();
        let transformed = transform(&mark, |p| p + Vec2::new(-200., 800.));
        assert_eq!(transformed.nodes.len(), 2);
        assert_eq!(transformed.nodes[0].out_handle, [-190., 820.]);
        assert_eq!(transformed.holes[0][0], [-199., 802.]);
        assert!(mark.points.is_empty());
    }
    #[test]
    #[ignore = "manual release benchmark"]
    fn benchmark_vector_fit_and_partial_fill() {
        let raw: Vec<_> = (0..5000)
            .map(|i| {
                let t = i as f32 * 0.005;
                [480. + t.cos() * 300., 270. + t.sin() * 180.]
            })
            .collect();
        let start = std::time::Instant::now();
        let curves = fit(&raw, 2.);
        let fit_time = start.elapsed();
        let mark = from_cubics(
            &curves,
            &Mark {
                width: 8.,
                ..Default::default()
            },
        );
        let start = std::time::Instant::now();
        let shapes = brush_fill(&mark, [0, 0, 0, 255]);
        let fill_time = start.elapsed();
        let start = std::time::Instant::now();
        for shape in &shapes {
            std::hint::black_box(split_rect(
                shape,
                Rect::from_min_max(Pos2::new(300., 200.), Pos2::new(600., 400.)),
            ));
        }
        eprintln!(
            "vector 5000 samples: fit={fit_time:?}, segments={}, brush fill={fill_time:?}, area split={:?}",
            curves.len(),
            start.elapsed()
        );
    }
}
