//! Convex translational configuration space for forward rigid construction.
//!
//! For a fixed relative orientation, the forbidden translation region for a
//! convex candidate B against an existing convex body A is the Minkowski
//! difference A + (-B). Its boundary is the no-fit boundary: translations on
//! that boundary place the bodies in exact non-penetrating contact.
//!
//! This module keeps feature provenance so edge-to-edge contact can be ranked
//! without turning that preference into a topology rule.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Point { pub x: f64, pub y: f64 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundaryFeatureKind { Vertex, Edge }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FeatureRef {
    pub kind: BoundaryFeatureKind,
    pub index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContactFeatureClass {
    VertexVertex,
    VertexEdge,
    EdgeVertex,
    EdgeEdge,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoundaryFeature {
    pub start: Point,
    pub end: Point,
    pub a: FeatureRef,
    pub b: FeatureRef,
    pub class: ContactFeatureClass,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConvexConfigurationBoundary {
    pub vertices: Vec<Point>,
    pub features: Vec<BoundaryFeature>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConfigurationSpaceError {
    TooFewVertices,
    NonFiniteVertex,
    DegeneratePolygon,
    NonConvexPolygon,
}

const EPSILON: f64 = 1.0e-10;

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn signed_area(vertices: &[Point]) -> f64 {
    vertices.iter().enumerate().map(|(i, a)| {
        let b = vertices[(i + 1) % vertices.len()];
        a.x * b.y - a.y * b.x
    }).sum::<f64>() * 0.5
}

fn normalize_ccw(mut vertices: Vec<Point>) -> Result<Vec<Point>, ConfigurationSpaceError> {
    if vertices.len() < 3 { return Err(ConfigurationSpaceError::TooFewVertices); }
    if !vertices.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
        return Err(ConfigurationSpaceError::NonFiniteVertex);
    }
    let area = signed_area(&vertices);
    if !area.is_finite() || area.abs() <= EPSILON {
        return Err(ConfigurationSpaceError::DegeneratePolygon);
    }
    if area < 0.0 { vertices.reverse(); }
    Ok(vertices)
}

fn convex_vertices(vertices: &[(f64, f64)]) -> Result<Vec<Point>, ConfigurationSpaceError> {
    let points = vertices.iter().map(|&(x, y)| Point { x, y }).collect();
    let vertices = normalize_ccw(points)?;
    let mut sign = 0.0;
    for i in 0..vertices.len() {
        let turn = cross(vertices[i], vertices[(i + 1) % vertices.len()], vertices[(i + 2) % vertices.len()]);
        if turn.abs() <= EPSILON { continue; }
        if sign == 0.0 { sign = turn.signum(); }
        else if turn.signum() != sign { return Err(ConfigurationSpaceError::NonConvexPolygon); }
    }
    if sign <= 0.0 { return Err(ConfigurationSpaceError::NonConvexPolygon); }
    Ok(vertices)
}

fn lowest_vertex_index(vertices: &[Point]) -> usize {
    vertices
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.y.partial_cmp(&b.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn cyclic_edges(vertices: &[Point]) -> Vec<Point> {
    (0..vertices.len())
        .map(|i| {
            let a = vertices[i];
            let b = vertices[(i + 1) % vertices.len()];
            Point {
                x: b.x - a.x,
                y: b.y - a.y,
            }
        })
        .collect()
}

fn edge_cross(a: Point, b: Point) -> f64 {
    a.x * b.y - a.y * b.x
}

fn edge_parallel(a: Point, b: Point, c: Point, d: Point) -> bool {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let cdx = d.x - c.x;
    let cdy = d.y - c.y;
    (abx * cdy - aby * cdx).abs() <= EPSILON * (abx.hypot(aby) * cdx.hypot(cdy)).max(1.0)
}

fn nearest_vertex(point: Point, vertices: &[Point]) -> usize {
    vertices.iter().enumerate().min_by(|(_, a), (_, b)| {
        let da = (a.x - point.x).hypot(a.y - point.y);
        let db = (b.x - point.x).hypot(b.y - point.y);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    }).map(|(i, _)| i).unwrap_or(0)
}

fn classify_boundary_feature(
    start: Point,
    end: Point,
    a: &[Point],
    b_negated: &[Point],
) -> BoundaryFeature {
    let a_edge = (0..a.len()).find(|&i| edge_parallel(start, end, a[i], a[(i + 1) % a.len()]));
    let b_edge = (0..b_negated.len()).find(|&i| edge_parallel(start, end, b_negated[i], b_negated[(i + 1) % b_negated.len()]));

    match (a_edge, b_edge) {
        (Some(ai), Some(bi)) => BoundaryFeature {
            start, end,
            a: FeatureRef { kind: BoundaryFeatureKind::Edge, index: ai },
            b: FeatureRef { kind: BoundaryFeatureKind::Edge, index: bi },
            class: ContactFeatureClass::EdgeEdge,
        },
        (Some(ai), None) => BoundaryFeature {
            start, end,
            a: FeatureRef { kind: BoundaryFeatureKind::Edge, index: ai },
            b: FeatureRef { kind: BoundaryFeatureKind::Vertex, index: nearest_vertex(end, b_negated) },
            class: ContactFeatureClass::EdgeVertex,
        },
        (None, Some(bi)) => BoundaryFeature {
            start, end,
            a: FeatureRef { kind: BoundaryFeatureKind::Vertex, index: nearest_vertex(end, a) },
            b: FeatureRef { kind: BoundaryFeatureKind::Edge, index: bi },
            class: ContactFeatureClass::VertexEdge,
        },
        (None, None) => BoundaryFeature {
            start, end,
            a: FeatureRef { kind: BoundaryFeatureKind::Vertex, index: nearest_vertex(start, a) },
            b: FeatureRef { kind: BoundaryFeatureKind::Vertex, index: nearest_vertex(start, b_negated) },
            class: ContactFeatureClass::VertexVertex,
        },
    }
}

/// Return the contact feature represented by a translation on the NFP boundary.
///
/// The translation is the candidate reference point relative to the anchor
/// reference point. A boundary translation is exact non-penetrating contact;
/// interior translations overlap and exterior translations are separated.
pub(crate) fn boundary_feature_at_translation(
    boundary: &ConvexConfigurationBoundary,
    translation: Point,
    tolerance: f64,
) -> Option<ContactFeatureClass> {
    let tolerance = tolerance.max(0.0);
    boundary.features.iter().find_map(|feature| {
        let dx = feature.end.x - feature.start.x;
        let dy = feature.end.y - feature.start.y;
        let length_sq = dx * dx + dy * dy;
        if length_sq <= EPSILON * EPSILON {
            return None;
        }
        let cross = (translation.x - feature.start.x) * dy
            - (translation.y - feature.start.y) * dx;
        let dot = (translation.x - feature.start.x) * dx
            + (translation.y - feature.start.y) * dy;
        let distance = cross.abs() / length_sq.sqrt();
        if distance <= tolerance
            && dot >= -tolerance * length_sq.sqrt()
            && dot <= length_sq + tolerance * length_sq.sqrt()
        {
            Some(feature.class)
        } else {
            None
        }
    })
}



/// Return one deterministic representative translation for each NFP boundary
/// feature. These are contact locations, not a topology prescription.
pub(crate) fn boundary_feature_representatives(
    boundary: &ConvexConfigurationBoundary,
) -> Vec<(Point, ContactFeatureClass)> {
    boundary
        .features
        .iter()
        .map(|feature| {
            (
                Point {
                    x: (feature.start.x + feature.end.x) * 0.5,
                    y: (feature.start.y + feature.end.y) * 0.5,
                },
                feature.class,
            )
        })
        .collect()
}


/// Generate finite candidate rotations from parallel convex boundary edges.
/// This exposes the orientation component of the NFP contact geometry without
/// introducing an angular sampling grid.
pub(crate) fn edge_alignment_rotations(
    existing_vertices: &[(f64, f64)],
    existing_rotation: f64,
    candidate_vertices: &[(f64, f64)],
    candidate_relative_rotation: f64,
) -> Vec<f64> {
    let existing = match convex_vertices(existing_vertices) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let candidate = match convex_vertices(candidate_vertices) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut rotations = Vec::new();
    for i in 0..existing.len() {
        let a = existing[i];
        let b = existing[(i + 1) % existing.len()];
        let ea = (b.y - a.y).atan2(b.x - a.x) + existing_rotation;
        for j in 0..candidate.len() {
            let c = candidate[j];
            let d = candidate[(j + 1) % candidate.len()];
            let eb = (d.y - c.y).atan2(d.x - c.x) + candidate_relative_rotation;
            for offset in [0.0, std::f64::consts::PI] {
                let angle = ea + offset - eb;
                if !rotations.iter().any(|r: &f64| {
                    let mut delta = (*r - angle).rem_euclid(std::f64::consts::TAU);
                    if delta > std::f64::consts::PI { delta -= std::f64::consts::TAU; }
                    delta.abs() <= EPSILON
                }) {
                    rotations.push(angle);
                }
            }
        }
    }
    rotations
}

/// Build A + (-B), the translational no-fit boundary for one fixed orientation.
pub(crate) fn convex_minkowski_difference(
    a: &[(f64, f64)],
    b: &[(f64, f64)],
) -> Result<ConvexConfigurationBoundary, ConfigurationSpaceError> {
    let a = convex_vertices(a)?;
    let b = convex_vertices(b)?;
    let b_negated = b
        .iter()
        .map(|p| Point { x: -p.x, y: -p.y })
        .collect::<Vec<_>>();

    // Both inputs are convex and CCW. Rotating each to its lowest (y, then x)
    // vertex orders its edge vectors by polar angle, so their Minkowski sum
    // can be constructed by a linear merge instead of pairwise sums + hull.
    let a_start = lowest_vertex_index(&a);
    let b_start = lowest_vertex_index(&b_negated);
    let a = a
        .iter()
        .cycle()
        .skip(a_start)
        .take(a.len())
        .copied()
        .collect::<Vec<_>>();
    let b_negated = b_negated
        .iter()
        .cycle()
        .skip(b_start)
        .take(b_negated.len())
        .copied()
        .collect::<Vec<_>>();
    let a_edges = cyclic_edges(&a);
    let b_edges = cyclic_edges(&b_negated);

    let mut vertices = Vec::with_capacity(a.len() + b_negated.len());
    let mut features = Vec::with_capacity(a.len() + b_negated.len());
    let mut i = 0;
    let mut j = 0;
    let mut current = Point {
        x: a[0].x + b_negated[0].x,
        y: a[0].y + b_negated[0].y,
    };

    while i < a_edges.len() || j < b_edges.len() {
        vertices.push(current);
        let parallel = i < a_edges.len()
            && j < b_edges.len()
            && edge_cross(a_edges[i], b_edges[j]).abs()
                <= EPSILON * (a_edges[i].x.hypot(a_edges[i].y)
                    * b_edges[j].x.hypot(b_edges[j].y))
                    .max(1.0);

        if j == b_edges.len()
            || (i < a_edges.len() && (parallel || edge_cross(a_edges[i], b_edges[j]) > 0.0))
        {
            let start = current;
            current = Point {
                x: current.x + a_edges[i].x,
                y: current.y + a_edges[i].y,
            };
            features.push(BoundaryFeature {
                start,
                end: current,
                a: FeatureRef {
                    kind: BoundaryFeatureKind::Edge,
                    index: (a_start + i) % a.len(),
                },
                b: FeatureRef {
                    kind: BoundaryFeatureKind::Vertex,
                    index: (b_start + j) % b_negated.len(),
                },
                class: if parallel {
                    i += 1;
                    j += 1;
                    current = Point {
                        x: current.x + b_edges[j - 1].x,
                        y: current.y + b_edges[j - 1].y,
                    };
                    ContactFeatureClass::EdgeEdge
                } else {
                    i += 1;
                    ContactFeatureClass::EdgeVertex
                },
            });
        } else {
            let start = current;
            current = Point {
                x: current.x + b_edges[j].x,
                y: current.y + b_edges[j].y,
            };
            features.push(BoundaryFeature {
                start,
                end: current,
                a: FeatureRef {
                    kind: BoundaryFeatureKind::Vertex,
                    index: (a_start + i) % a.len(),
                },
                b: FeatureRef {
                    kind: BoundaryFeatureKind::Edge,
                    index: (b_start + j) % b_negated.len(),
                },
                class: ContactFeatureClass::VertexEdge,
            });
            j += 1;
        }
    }

    if vertices.len() < 3 || features.len() != vertices.len() {
        return Err(ConfigurationSpaceError::DegeneratePolygon);
    }

    Ok(ConvexConfigurationBoundary { vertices, features })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<(f64, f64)> {
        vec![(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
    }

    #[test]
    fn equal_squares_have_exact_nfp_extent() {
        let nfp = convex_minkowski_difference(&square(), &square()).unwrap();
        assert_eq!(nfp.vertices.len(), 4);
        assert!(nfp.vertices.iter().all(|p| p.x.abs() == 2.0 || p.y.abs() == 2.0));
    }

    #[test]
    fn equal_squares_preserve_edge_edge_loci() {
        let nfp = convex_minkowski_difference(&square(), &square()).unwrap();
        assert_eq!(nfp.features.len(), 4);
        assert!(nfp.features.iter().all(|f| f.class == ContactFeatureClass::EdgeEdge));
    }

    #[test]
    fn translation_on_nfp_boundary_is_contact() {
        let nfp = convex_minkowski_difference(&square(), &square()).unwrap();
        assert_eq!(
            boundary_feature_at_translation(
                &nfp,
                Point { x: 2.0, y: 0.0 },
                1e-9,
            ),
            Some(ContactFeatureClass::EdgeEdge)
        );
        assert_eq!(
            boundary_feature_at_translation(
                &nfp,
                Point { x: 0.0, y: 0.0 },
                1e-9,
            ),
            None
        );
    }

    #[test]
    fn reversed_convex_input_is_normalized() {
        let mut reversed = square();
        reversed.reverse();
        assert_eq!(
            convex_minkowski_difference(&square(), &square()).unwrap().vertices,
            convex_minkowski_difference(&reversed, &square()).unwrap().vertices
        );
    }

    #[test]
    fn concave_input_is_rejected() {
        let concave = vec![(-1.0,-1.0),(1.0,-1.0),(1.0,1.0),(0.0,1.0),(0.0,0.0),(-1.0,0.0)];
        assert_eq!(
            convex_minkowski_difference(&concave, &square()),
            Err(ConfigurationSpaceError::NonConvexPolygon)
        );
    }
}
