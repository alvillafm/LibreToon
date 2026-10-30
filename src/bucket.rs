//! Vector flood fill: the bounded faces of the active frame are extracted from
//! actual ink. Gap bridges are temporary and never change the user's contours.
use crate::{document::Mark, vector, width};
use eframe::egui::{Pos2, Rect};
use geo::{BooleanOps, BoundingRect, Buffer, Contains, Coord, LineString, MultiPolygon, Polygon};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Gap {
    #[default]
    None,
    Small,
    Medium,
    Large,
}
impl Gap {
    pub const ALL: [Self; 4] = [Self::None, Self::Small, Self::Medium, Self::Large];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No cerrar huecos",
            Self::Small => "Cerrar huecos pequeños",
            Self::Medium => "Cerrar huecos medianos",
            Self::Large => "Cerrar huecos grandes",
        }
    }
    pub fn distance(self) -> f32 {
        match self {
            Self::None => 0.,
            Self::Small => 6.,
            Self::Medium => 16.,
            Self::Large => 36.,
        }
    }
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
pub fn fill(marks: &[Mark], click: Pos2, gap: Gap, color: [u8; 4]) -> Result<Vec<Mark>, String> {
    let mut paths: Vec<(Vec<Pos2>, f32, bool)> = vec![];
    let mut obstacles: Vec<MultiPolygon<f64>> = vec![];
    let mut bounds = Rect::from_min_max(click, click);
    for mark in marks.iter().filter(|m| !m.bucket) {
        let points = vector::flatten(mark, 0.12);
        if points.len() < 2 {
            continue;
        }
        for p in &points {
            bounds.extend_with(*p);
        }
        let thickness = if mark.stroke {
            mark.width.max(0.5)
        } else {
            0.5
        };
        if mark.stroke && !mark.width_profile.is_empty() {
            obstacles.push(width::outline(mark));
        } else if mark.stroke || mark.fill.is_some() {
            obstacles.push(line(points.iter().copied()).buffer(thickness as f64 / 2.));
        }
        if mark.stroke || mark.fill.is_some() {
            paths.push((points, thickness, mark.closed || mark.rectangle));
        }
        for hole in &mark.holes {
            let points: Vec<_> = hole.iter().copied().map(vector::pos).collect();
            if points.len() > 2 {
                obstacles.push(line(points.iter().copied()).buffer(thickness as f64 / 2.));
                paths.push((points, thickness, true));
            }
        }
    }
    if obstacles.is_empty() {
        return Err("No se encontró una zona cerrada en la capa activa".into());
    }
    if gap != Gap::None {
        let threshold = gap.distance();
        for (owner, (path, thickness, closed)) in paths.iter().enumerate() {
            if *closed || path.first() == path.last() {
                continue;
            }
            let (lengths, total) = width::lengths(path);
            for (endpoint, at_end) in [(path[0], false), (*path.last().unwrap(), true)] {
                let mut nearest: Option<(Pos2, f32)> = None;
                for (other, (other_path, _, _)) in paths.iter().enumerate() {
                    for (segment, pair) in other_path.windows(2).enumerate() {
                        if other == owner {
                            let distance_along = if at_end {
                                total - lengths[(segment + 1).min(lengths.len() - 1)]
                            } else {
                                lengths[segment]
                            };
                            if distance_along < threshold * 2. {
                                continue;
                            }
                        }
                        let d = pair[1] - pair[0];
                        let t =
                            ((endpoint - pair[0]).dot(d) / d.length_sq().max(1e-12)).clamp(0., 1.);
                        let target = pair[0] + d * t;
                        let distance = endpoint.distance(target);
                        if distance > 0.001
                            && distance <= threshold
                            && nearest.as_ref().is_none_or(|(_, best)| distance < *best)
                        {
                            nearest = Some((target, distance));
                        }
                    }
                }
                if let Some((target, _)) = nearest {
                    obstacles.push(line([endpoint, target]).buffer(*thickness as f64 / 2.));
                }
            }
        }
    }
    let pad = bounds.width().max(bounds.height()).max(256.) * 0.15 + gap.distance() + 16.;
    let guard = bounds.expand(pad);
    let outer = Polygon::new(
        line([
            guard.left_top(),
            guard.right_top(),
            guard.right_bottom(),
            guard.left_bottom(),
            guard.left_top(),
        ]),
        vec![],
    );
    let ink = geo::algorithm::bool_ops::unary_union(&obstacles);
    let free = outer.difference(&ink);
    let point = geo::Point::new(click.x as f64, click.y as f64);
    let region = free
        .0
        .into_iter()
        .find(|p| p.contains(&point))
        .ok_or("Haz clic dentro de una zona, fuera del contorno")?;
    let rect = region.bounding_rect().ok_or("Zona de relleno vacía")?;
    if (rect.min().x - guard.left() as f64).abs() < 0.1
        || (rect.max().x - guard.right() as f64).abs() < 0.1
        || (rect.min().y - guard.top() as f64).abs() < 0.1
        || (rect.max().y - guard.bottom() as f64).abs() < 0.1
    {
        return Err("La zona está abierta. Prueba otro nivel de cierre de huecos".into());
    }
    let mut shapes = vector::polygons_to_marks(MultiPolygon(vec![region]), color);
    for shape in &mut shapes {
        shape.bucket = true;
    }
    Ok(shapes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contour(gap: f32) -> Mark {
        Mark {
            points: vec![[0., 0.], [100., 0.], [100., 100.], [0., 100.], [0., gap]].into(),
            width: 1.,
            ..Default::default()
        }
    }
    #[test]
    fn fills_closed_regions_and_rejects_unbounded_space() {
        let mut m = contour(0.);
        m.closed = true;
        let shapes = fill(
            &[m.clone()],
            Pos2::new(50., 50.),
            Gap::None,
            [20, 80, 200, 255],
        )
        .unwrap();
        assert_eq!(shapes.len(), 1);
        assert!(shapes[0].bucket);
        assert!(vector::contains_fill(&shapes[0], Pos2::new(50., 50.)));
        assert!(fill(&[m], Pos2::new(200., 200.), Gap::Large, [0, 0, 0, 255]).is_err());
    }
    #[test]
    fn gap_levels_close_only_gaps_within_their_threshold() {
        let m = contour(8.);
        assert!(fill(&[m.clone()], Pos2::new(50., 50.), Gap::None, [0, 0, 0, 255]).is_err());
        assert!(
            fill(
                &[m.clone()],
                Pos2::new(50., 50.),
                Gap::Small,
                [0, 0, 0, 255]
            )
            .is_err()
        );
        assert!(
            fill(
                &[m.clone()],
                Pos2::new(50., 50.),
                Gap::Medium,
                [0, 0, 0, 255]
            )
            .is_ok()
        );
        assert_eq!(m.points[4], [0., 8.]);
    }
}
