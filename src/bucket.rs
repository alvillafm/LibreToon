//! Vector flood fill: the bounded faces of the active frame are extracted from
//! actual ink. Gap bridges are temporary and never change the user's contours.
use crate::{document::Mark, vector, width};
use eframe::egui::{Pos2, Rect};
use geo::{
    Area, BooleanOps, BoundingRect, Buffer, Contains, Coord, LineString, MultiPolygon, Polygon,
    algorithm::buffer::{BufferStyle, LineCap, LineJoin},
};

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
        crate::i18n::tr(match self {
            Self::None => "No cerrar huecos",
            Self::Small => "Cerrar huecos pequeños",
            Self::Medium => "Cerrar huecos medianos",
            Self::Large => "Cerrar huecos grandes",
        })
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
    fill_solution(marks, click, gap, color).map(|(marks, _)| marks)
}
pub(crate) fn fill_preview(
    marks: &[Mark],
    click: Pos2,
    gap: Gap,
    color: [u8; 4],
) -> Result<(Vec<Mark>, Vec<[Pos2; 2]>), String> {
    let (shapes, level) = fill_solution(marks, click, gap, color)?;
    let bridges = cached_faces(marks, level)?.bridges.clone();
    Ok((shapes, bridges))
}
fn fill_solution(
    marks: &[Mark],
    click: Pos2,
    gap: Gap,
    color: [u8; 4],
) -> Result<(Vec<Mark>, Gap), String> {
    let exact = fill_at_level(marks, click, Gap::None, color);
    if gap == Gap::None {
        return exact.map(|marks| (marks, Gap::None));
    }
    if let Ok(original) = exact {
        // A bounded region can still leak into a tiny neighbouring tip.
        // Keep only local closures that trim a small part of the original face.
        if let Ok(closed) = fill_at_level(marks, click, gap, color) {
            let polygons =
                |marks: &[Mark]| MultiPolygon(marks.iter().map(vector::polygon).collect());
            let before = polygons(&original);
            let after = polygons(&closed);
            let area = before.unsigned_area();
            let overlap = before.intersection(&after).unsigned_area();
            if overlap >= area * 0.95 && after.unsigned_area() - overlap <= area * 0.005 {
                return Ok((closed, gap));
            }
        }
        return Ok((original, Gap::None));
    }
    let mut result = exact;
    let mut used = Gap::None;
    for level in Gap::ALL
        .into_iter()
        .skip(1)
        .take_while(|level| level.distance() <= gap.distance())
    {
        used = level;
        result = fill_at_level(marks, click, level, color);
        if result.is_ok() {
            break;
        }
    }
    result.map(|marks| (marks, used))
}

struct Faces {
    bridges: Vec<[Pos2; 2]>,
    real_ink: MultiPolygon<f64>,
    ink: MultiPolygon<f64>,
    silhouettes: Vec<Polygon<f64>>,
    outer: Polygon<f64>,
    guard: Rect,
    free: MultiPolygon<f64>,
}
struct CachedFaces {
    source: Vec<Mark>,
    gap: Gap,
    faces: std::sync::Arc<Faces>,
}
thread_local! {
    static FACE_CACHE: std::cell::RefCell<Vec<CachedFaces>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn source_geometry(marks: &[Mark]) -> Vec<Mark> {
    marks
        .iter()
        .filter(|m| {
            !m.bucket
                && !m.style.guide
                && m.style.media_image.is_none()
                && m.style.instance.is_none()
                && (m.stroke || m.fill.is_some_and(|c| c[3] > 0))
        })
        .map(|m| {
            let mut geometry = Mark {
                points: m.points.clone(),
                nodes: m.nodes.clone(),
                holes: m.holes.clone(),
                width_profile: m.width_profile.clone(),
                width: m.width,
                rectangle: m.rectangle,
                closed: m.closed,
                stroke: m.stroke,
                fill: m.fill.map(|_| [255; 4]),
                ..Default::default()
            };
            geometry.style.start_cap = m.style.start_cap;
            geometry.style.end_cap = m.style.end_cap;
            geometry
        })
        .collect()
}
fn cached_faces(marks: &[Mark], gap: Gap) -> Result<std::sync::Arc<Faces>, String> {
    let source = source_geometry(marks);
    if let Some(found) = FACE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache
            .iter()
            .position(|entry| entry.gap == gap && entry.source == source)?;
        let entry = cache.remove(index);
        let faces = entry.faces.clone();
        cache.push(entry);
        Some(faces)
    }) {
        return Ok(found);
    }
    let faces = std::sync::Arc::new(build_faces(&source, gap)?);
    use geo::CoordsIter;
    // Four gap levels, at most ~2 MiB of coordinates per entry. Oversized
    // drawings still work, but are not retained by this optimization.
    let source_points: usize = source
        .iter()
        .map(|m| {
            m.points.len() + m.nodes.len() * 3 + m.holes.iter().map(|h| h.len()).sum::<usize>()
        })
        .sum();
    let coordinates = faces.real_ink.coords_count()
        + faces.ink.coords_count()
        + faces.free.coords_count()
        + faces
            .silhouettes
            .iter()
            .map(|p| p.coords_count())
            .sum::<usize>();
    if source.len() <= 2048 && source_points <= 65536 && coordinates <= 65536 {
        FACE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if cache.len() >= 4 {
                cache.remove(0);
            }
            cache.push(CachedFaces {
                source,
                gap,
                faces: faces.clone(),
            });
        });
    }
    Ok(faces)
}
fn fill_at_level(
    marks: &[Mark],
    click: Pos2,
    gap: Gap,
    color: [u8; 4],
) -> Result<Vec<Mark>, String> {
    let faces = cached_faces(marks, gap)?;
    let region = match face_at(&faces.free, faces.guard, click) {
        Ok(region) => region,
        Err(error) if gap != Gap::None && !faces.silhouettes.is_empty() => {
            let radius = f64::from(gap.distance()) / 2.;
            let silhouettes = geo::algorithm::bool_ops::unary_union(&faces.silhouettes);
            let closed = silhouettes
                .buffer_with_style(BufferStyle::new(radius).line_join(LineJoin::Miter(2.)))
                .buffer_with_style(BufferStyle::new(-radius).line_join(LineJoin::Miter(2.)));
            bounded_face(&faces.outer, &faces.ink.union(&closed), faces.guard, click)
                .map_err(|_| error)?
        }
        Err(error) => return Err(error),
    };
    let face = MultiPolygon(vec![region]);
    let underpaint = face.buffer(0.35).intersection(&faces.real_ink);
    let mut shapes = vector::polygons_to_marks(face.union(&underpaint), color);
    for shape in &mut shapes {
        shape.bucket = true;
    }
    Ok(shapes)
}
fn build_faces(marks: &[Mark], gap: Gap) -> Result<Faces, String> {
    let mut paths: Vec<(Vec<Pos2>, f32, bool)> = vec![];
    let mut obstacles: Vec<MultiPolygon<f64>> = vec![];
    let mut silhouettes = Vec::new();
    let mut bounds = Rect::NOTHING;
    for mark in marks.iter().filter(|m| {
        !m.bucket
            && !m.style.guide
            && m.style.media_image.is_none()
            && m.style.instance.is_none()
            && (m.stroke || m.fill.is_some_and(|color| color[3] > 0))
    }) {
        let points = vector::flatten(mark, 0.04);
        if points.len() < 2 {
            continue;
        }
        for p in &points {
            bounds.extend_with(*p);
        }
        let thickness = if mark.stroke {
            mark.width.max(0.01)
        } else {
            0.5
        };
        if mark.stroke
            && (!mark.width_profile.is_empty()
                || mark.style.start_cap != crate::features::Cap::Round
                || mark.style.end_cap != crate::features::Cap::Round)
        {
            obstacles.push(width::outline(mark));
        } else if mark.stroke {
            obstacles.push(line(points.iter().copied()).buffer(thickness as f64 / 2.));
        } else if mark.fill.is_some() {
            // Filled brush silhouettes already describe the real ink boundary.
            // Buffering their edge creates an artificial white gutter inside it.
            let polygon = vector::polygon(mark);
            silhouettes.push(polygon.clone());
            obstacles.push(MultiPolygon(vec![polygon]));
        }
        paths.push((
            points,
            thickness,
            !mark.stroke || mark.closed || mark.rectangle,
        ));
        for hole in &mark.holes {
            let points: Vec<_> = hole.iter().copied().map(vector::pos).collect();
            if points.len() > 2 {
                if mark.stroke {
                    obstacles.push(line(points.iter().copied()).buffer(thickness as f64 / 2.));
                }
                paths.push((points, thickness, true));
            }
        }
    }
    if obstacles.is_empty() {
        return Err("No se encontró una zona cerrada en la capa activa".into());
    }
    let real_ink = geo::algorithm::bool_ops::unary_union(&obstacles);
    let mut bridges = Vec::new();
    let mut bridge_segments = Vec::new();
    if gap != Gap::None {
        let threshold = gap.distance();
        let segments: Vec<_> = paths
            .iter()
            .enumerate()
            .flat_map(|(owner, (p, _, _))| {
                p.windows(2)
                    .enumerate()
                    .map(move |(i, p)| ((owner, i), Rect::from_two_pos(p[0], p[1])))
            })
            .collect();
        let index = crate::spatial::Index::new(segments.iter().map(|(_, r)| *r));
        for (owner, (path, thickness, closed)) in paths.iter().enumerate() {
            if *closed || path.first() == path.last() {
                continue;
            }
            let (lengths, total) = width::lengths(path);
            for (endpoint, at_end) in [(path[0], false), (*path.last().unwrap(), true)] {
                let adjacent = if at_end {
                    path[path.len() - 2]
                } else {
                    path[1]
                };
                let outward = (endpoint - adjacent).normalized();
                let mut nearest: Option<(Pos2, f32)> = None;
                let mut joined = false;
                'neighbors: for candidate in index.query(Rect::from_center_size(
                    endpoint,
                    eframe::egui::Vec2::splat(threshold * 2. + 0.16),
                )) {
                    let (other, segment) = segments[candidate].0;
                    let pair = &paths[other].0[segment..segment + 2];
                    {
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
                        if other != owner && distance <= 0.08 {
                            joined = true;
                            break 'neighbors;
                        }
                        if distance > 0.001
                            && distance <= threshold
                            && (target - endpoint).dot(outward) > distance * 0.12
                            && nearest.as_ref().is_none_or(|(_, best)| distance < *best)
                        {
                            nearest = Some((target, distance));
                        }
                    }
                }
                if !joined && let Some((target, _)) = nearest {
                    bridge_segments.push([endpoint, target]);
                    bridges.push(
                        line([endpoint, target]).buffer_with_style(
                            BufferStyle::new(f64::from(*thickness).min(1.) / 2.)
                                .line_cap(LineCap::Square),
                        ),
                    );
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
    let ink = if bridges.is_empty() {
        real_ink.clone()
    } else {
        real_ink.union(&geo::algorithm::bool_ops::unary_union(&bridges))
    };
    let free = outer.difference(&ink);
    Ok(Faces {
        bridges: bridge_segments,
        real_ink,
        ink,
        silhouettes,
        outer,
        guard,
        free,
    })
}

pub(crate) fn covered_bucket_fills(marks: &[Mark], replacement: &[Mark]) -> Vec<usize> {
    let faces: Vec<_> = replacement.iter().map(vector::polygon).collect();
    let replacement = geo::algorithm::bool_ops::unary_union(&faces);
    marks
        .iter()
        .enumerate()
        .filter_map(|(index, mark)| {
            if !mark.bucket || mark.fill.is_none() {
                return None;
            }
            let old = vector::polygon(mark);
            let area = old.unsigned_area();
            (area > 0. && replacement.intersection(&old).unsigned_area() >= area * 0.8)
                .then_some(index)
        })
        .collect()
}

fn bounded_face(
    outer: &Polygon<f64>,
    ink: &MultiPolygon<f64>,
    guard: Rect,
    click: Pos2,
) -> Result<Polygon<f64>, String> {
    let free = outer.difference(ink);
    face_at(&free, guard, click)
}
fn face_at(free: &MultiPolygon<f64>, guard: Rect, click: Pos2) -> Result<Polygon<f64>, String> {
    let point = geo::Point::new(click.x as f64, click.y as f64);
    let region = free
        .0
        .iter()
        .find(|p| p.contains(&point))
        .cloned()
        .ok_or("Haz clic dentro de una zona, fuera del contorno")?;
    let rect = region.bounding_rect().ok_or("Zona de relleno vacía")?;
    if (rect.min().x - guard.left() as f64).abs() < 0.1
        || (rect.max().x - guard.right() as f64).abs() < 0.1
        || (rect.min().y - guard.top() as f64).abs() < 0.1
        || (rect.max().y - guard.bottom() as f64).abs() < 0.1
    {
        return Err("La zona está abierta. Prueba otro nivel de cierre de huecos".into());
    }
    Ok(region)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_reports_bridges_without_changing_ink_or_fill_geometry() {
        let marks = vec![contour(8.)];
        let before = serde_json::to_vec(&marks).unwrap();
        let (preview, bridges) =
            fill_preview(&marks, Pos2::new(50., 50.), Gap::Medium, [200, 40, 60, 255]).unwrap();
        assert!(!bridges.is_empty());
        let applied = fill(&marks, Pos2::new(50., 50.), Gap::Medium, [200, 40, 60, 255]).unwrap();
        assert!(preview == applied);
        assert_eq!(serde_json::to_vec(&marks).unwrap(), before);
        let closed = vec![contour(0.)];
        assert!(
            fill_preview(&closed, Pos2::new(50., 50.), Gap::None, [255; 4])
                .unwrap()
                .1
                .is_empty()
        );
    }
    #[test]
    fn face_cache_reuses_color_changes_and_invalidates_geometry() {
        FACE_CACHE.with(|cache| cache.borrow_mut().clear());
        let mut mark = contour(0.);
        let first = cached_faces(&[mark.clone()], Gap::None).unwrap();
        mark.color = [240, 40, 10, 255];
        let recolored = cached_faces(&[mark.clone()], Gap::None).unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &recolored));
        let fills = fill(
            &[mark.clone()],
            Pos2::new(20., 20.),
            Gap::None,
            [10, 20, 30, 255],
        )
        .unwrap();
        let mut with_fill = vec![mark.clone()];
        with_fill.extend(fills);
        assert!(std::sync::Arc::ptr_eq(
            &first,
            &cached_faces(&with_fill, Gap::None).unwrap()
        ));
        mark.points[1] = [120., 0.];
        assert!(!std::sync::Arc::ptr_eq(
            &first,
            &cached_faces(&[mark], Gap::None).unwrap()
        ));
        FACE_CACHE.with(|cache| assert!(cache.borrow().len() <= 4));
    }
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
    fn brush_fill_meets_ink_without_a_gutter_or_spilling_outside() {
        let outline = Mark {
            points: vec![[8., 8.], [56., 8.], [56., 56.], [8., 56.], [8., 8.]].into(),
            holes: vec![vec![[12., 12.], [12., 52.], [52., 52.], [52., 12.], [12., 12.]].into()]
                .into(),
            fill: Some([0, 0, 0, 255]),
            stroke: false,
            closed: true,
            ..Default::default()
        };
        let original = serde_json::to_vec(&outline).unwrap();
        let filled = fill(
            &[outline.clone()],
            Pos2::new(32., 32.),
            Gap::None,
            [89, 72, 220, 255],
        )
        .unwrap();
        assert!(
            filled
                .iter()
                .any(|m| vector::contains_fill(m, Pos2::new(12.05, 32.)))
        );
        assert!(
            filled
                .iter()
                .any(|m| vector::contains_fill(m, Pos2::new(11.8, 32.)))
        );
        assert!(
            !filled
                .iter()
                .any(|m| vector::contains_fill(m, Pos2::new(7.9, 32.)))
        );
        assert_eq!(original, serde_json::to_vec(&outline).unwrap());
        let mut doc = crate::Document::default();
        doc.width = 64;
        doc.height = 64;
        doc.layers[0].frames[0] = filled.into();
        doc.layers[0].frames[0].push(outline);
        for scale in [1, 11] {
            let size = 64 * scale;
            let pixels = crate::exporter::raster(
                &doc,
                0,
                size,
                size,
                &eframe::egui::Context::default(),
                &mut Default::default(),
            );
            for x in 10 * scale..15 * scale {
                let at = ((32 * scale * size + x) * 3) as usize;
                assert!(
                    pixels[at] <= 90 && pixels[at + 1] <= 73 && pixels[at + 2] <= 221,
                    "White seam at scale {scale}, x {x}: {:?}",
                    &pixels[at..at + 3]
                );
            }
        }
    }

    #[test]
    fn unrelated_fill_does_not_round_a_narrow_channel_with_a_line_gap() {
        let wall = Mark {
            points: vec![[0., 0.], [80., 0.], [80., 8.], [0., 8.], [0., 5.]].into(),
            width: 1.,
            ..Default::default()
        };
        let decoration = Mark {
            points: vec![[120., 0.], [140., 15.]].into(),
            rectangle: true,
            fill: Some([40, 40, 40, 255]),
            stroke: false,
            ..Default::default()
        };
        let shapes = fill(&[wall, decoration], Pos2::new(20., 4.), Gap::Large, [20; 4]).unwrap();
        for point in [Pos2::new(1., 4.), Pos2::new(40., 1.), Pos2::new(79., 7.)] {
            assert!(
                shapes.iter().any(|mark| vector::contains_fill(mark, point)),
                "{point:?}"
            );
        }
        assert!(
            !shapes
                .iter()
                .any(|mark| vector::contains_fill(mark, Pos2::new(-1., 4.)))
        );
    }
    #[test]
    fn large_gap_does_not_create_a_face_from_open_parallel_strokes() {
        let make = |x| Mark {
            points: vec![[x, 0.], [x, 140.]].into(),
            width: 3.,
            ..Default::default()
        };
        let marks = [make(0.), make(24.)];
        // Both ends line up across the channel; neither stroke points into a gap.
        assert!(fill(&marks, Pos2::new(12., 70.), Gap::Large, [0, 0, 0, 255]).is_err());
    }

    #[test]
    fn closing_gaps_keeps_paint_out_of_a_small_tip_connected_to_a_bounded_face() {
        let marks = [
            Mark {
                closed: true,
                ..contour(0.)
            },
            Mark {
                points: vec![[10., 0.], [10., 8.], [2., 10.]].into(),
                width: 1.,
                ..Default::default()
            },
        ];
        let click = Pos2::new(50., 50.);
        let tip = Pos2::new(5., 4.);
        assert!(
            fill(&marks, click, Gap::None, [255; 4])
                .unwrap()
                .iter()
                .any(|m| vector::contains_fill(m, tip))
        );
        assert!(
            !fill(&marks, click, Gap::Medium, [255; 4])
                .unwrap()
                .iter()
                .any(|m| vector::contains_fill(m, tip))
        );
    }
    #[test]
    fn fill_overlap_covers_ink_without_painting_over_a_virtual_bridge() {
        let shapes = fill(&[contour(8.)], Pos2::new(50., 50.), Gap::Medium, [255; 4]).unwrap();
        assert!(
            shapes
                .iter()
                .any(|mark| vector::contains_fill(mark, Pos2::new(0.75, 4.)))
        );
        assert!(
            !shapes
                .iter()
                .any(|mark| vector::contains_fill(mark, Pos2::new(0.25, 4.)))
        );
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

#[cfg(test)]
mod painted_gap_regressions {
    use super::*;
    #[test]
    fn gaps_between_filled_brush_silhouettes_close_without_rewriting_contours() {
        let axis = Mark {
            points: vec![[0., 8.], [0., 100.], [100., 100.], [100., 0.], [0., 0.]].into(),
            width: 4.,
            ..Default::default()
        };
        let ink = vector::brush_fill(&axis, [0, 0, 0, 255]);
        let original = serde_json::to_vec(&ink).unwrap();
        assert!(fill(&ink, Pos2::new(50., 50.), Gap::None, [200, 0, 0, 255]).is_err());
        let result = fill(&ink, Pos2::new(50., 50.), Gap::Medium, [200, 0, 0, 255]).unwrap();
        assert!(
            result
                .iter()
                .any(|m| vector::contains_fill(m, Pos2::new(50., 50.)))
        );
        assert_eq!(original, serde_json::to_vec(&ink).unwrap());
        assert!(fill(&ink, Pos2::new(200., 200.), Gap::Large, [200, 0, 0, 255]).is_err());
    }
}
