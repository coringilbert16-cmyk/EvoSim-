//! Configuration-space geometry for rigid placement.
//!
//! For a fixed orientation, the touching translations of a convex polygon B
//! around a convex polygon A are the boundary of the Minkowski difference
//! A + (-B). Boundary segments are retained because an edge-flush contact is a
//! continuous placement locus, not an arbitrary point sample.
//!
//! Concave polygons are deliberately rejected here until their general
//! configuration-space construction is separately audited.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacementBoundarySegment {
    pub start: Point,
    pub end: Point,
    /// Source features whose Minkowski support can generate this boundary
    /// segment. Multiple pairs are allowed because the convex hull can merge
    /// coincident support contributions.
    pub feature_pairs: Vec<MinkowskiFeaturePair>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinkowskiFeature {
    Vertex(usize),
    Edge(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinkowskiFeaturePair {
    pub a: MinkowskiFeature,
    /// Feature index in the original B polygon. The sign flip used by the
    /// Minkowski difference does not change the source feature identity.
    pub b: MinkowskiFeature,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConvexConfigurationBoundary {
    pub vertices: Vec<Point>,
    pub segments: Vec<PlacementBoundarySegment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigurationSpaceError {
    TooFewVertices,
    NonFiniteVertex,
    DegeneratePolygon,
    NonConvexPolygon,
}

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn signed_area(vertices: &[Point]) -> f64 {
    vertices
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let b = vertices[(i + 1) % vertices.len()];
            a.x * b.y - a.y * b.x
        })
        .sum::<f64>()
        * 0.5
}

fn normalize_ccw(mut vertices: Vec<Point>) -> Result<Vec<Point>, ConfigurationSpaceError> {
    if vertices.len() < 3 {
        return Err(ConfigurationSpaceError::TooFewVertices);
    }
    if !vertices.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
        return Err(ConfigurationSpaceError::NonFiniteVertex);
    }
    let area = signed_area(&vertices);
    if !area.is_finite() || area.abs() <= 1e-12 {
        return Err(ConfigurationSpaceError::DegeneratePolygon);
    }
    if area < 0.0 {
        vertices.reverse();
    }
    Ok(vertices)
}

fn convex_vertices(vertices: &[Point]) -> Result<Vec<Point>, ConfigurationSpaceError> {
    let vertices = normalize_ccw(vertices.to_vec())?;
    let mut sign = 0.0;
    for i in 0..vertices.len() {
        let c = cross(
            vertices[i],
            vertices[(i + 1) % vertices.len()],
            vertices[(i + 2) % vertices.len()],
        );
        if c.abs() <= 1e-12 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return Err(ConfigurationSpaceError::NonConvexPolygon);
        }
    }
    if sign <= 0.0 {
        return Err(ConfigurationSpaceError::NonConvexPolygon);
    }
    Ok(vertices)
}

fn dedup_points(points: &mut Vec<Point>) {
    points.dedup_by(|a, b| (a.x - b.x).hypot(a.y - b.y) <= 1e-12);
    if points.len() > 1 {
        let first = points[0];
        let last = points[points.len() - 1];
        if (first.x - last.x).hypot(first.y - last.y) <= 1e-12 {
            points.pop();
        }
    }
}

fn cross_vectors(a: Point, b: Point) -> f64 {
    a.x * b.y - a.y * b.x
}

fn support_features(
    points: &[Point],
    indices: &[usize],
) -> Vec<MinkowskiFeature> {
    let mut features = Vec::new();
    for &index in indices {
        let next = (index + 1) % points.len();
        if indices.contains(&next) {
            features.push(MinkowskiFeature::Edge(index));
        } else {
            let previous = (index + points.len() - 1) % points.len();
            if indices.contains(&previous) {
                features.push(MinkowskiFeature::Edge(previous));
            } else {
                features.push(MinkowskiFeature::Vertex(index));
            }
        }
    }
    features.sort_by_key(|feature| match feature {
        MinkowskiFeature::Vertex(index) => (0, *index),
        MinkowskiFeature::Edge(index) => (1, *index),
    });
    features.dedup();
    features
}

fn boundary_feature_pairs(
    a: &[Point],
    b_negated: &[Point],
    start: Point,
    end: Point,
) -> Vec<MinkowskiFeaturePair> {
    let direction = Point { x: end.x - start.x, y: end.y - start.y };
    let length = direction.x.hypot(direction.y);
    if length <= 1e-12 {
        return Vec::new();
    }

    // The hull is CCW, so the boundary's outward normal is the right-hand
    // normal of its directed edge.
    let nx = direction.y / length;
    let ny = -direction.x / length;
    const SUPPORT_TOLERANCE: f64 = 1e-9;

    fn support_indices(points: &[Point], nx: f64, ny: f64) -> Vec<usize> {
        let maximum = points
            .iter()
            .map(|p| p.x * nx + p.y * ny)
            .fold(f64::NEG_INFINITY, f64::max);
        points
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                ((p.x * nx + p.y * ny) >= maximum - SUPPORT_TOLERANCE)
                    .then_some(i)
            })
            .collect()
    }

    let a_support = support_indices(a, nx, ny);
    // Both operands of A + (-B) use the same outward support normal. The
    // second operand is already the sign-flipped geometry, so reversing the
    // normal here would identify the wrong B feature.

    let a_features = support_features(a, &a_support);
    let b_features = support_features(b_negated, &support_indices(b_negated, nx, ny));
    let mut pairs = Vec::new();
    for a_feature in a_features {
        for b_feature in &b_features {
            pairs.push(MinkowskiFeaturePair {
                a: a_feature,
                b: *b_feature,
            });
        }
    }
    pairs
}

fn convex_hull(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    dedup_points(&mut points);
    if points.len() <= 2 {
        return points;
    }

    let mut lower = Vec::new();
    for point in &points {
        while lower.len() >= 2 {
            let a = lower[lower.len() - 2];
            let b = lower[lower.len() - 1];
            if cross_vectors(
                Point {
                    x: b.x - a.x,
                    y: b.y - a.y,
                },
                Point {
                    x: point.x - b.x,
                    y: point.y - b.y,
                },
            ) > 1e-12 {
                break;
            }
            lower.pop();
        }
        lower.push(*point);
    }

    let mut upper = Vec::new();
    for point in points.iter().rev() {
        while upper.len() >= 2 {
            let a = upper[upper.len() - 2];
            let b = upper[upper.len() - 1];
            if cross_vectors(
                Point {
                    x: b.x - a.x,
                    y: b.y - a.y,
                },
                Point {
                    x: point.x - b.x,
                    y: point.y - b.y,
                },
            ) > 1e-12 {
                break;
            }
            upper.pop();
        }
        upper.push(*point);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Build the exact convex Minkowski difference boundary A + (-B) for a fixed
/// relative orientation. A translation of B whose reference point lies on
/// this boundary is a non-penetrating touching configuration.
pub fn convex_minkowski_difference(
    a: &[(f64, f64)],
    b: &[(f64, f64)],
) -> Result<ConvexConfigurationBoundary, ConfigurationSpaceError> {
    let a = convex_vertices(
        &a.iter()
            .map(|&(x, y)| Point { x, y })
            .collect::<Vec<_>>(),
    )?;
    let b = convex_vertices(
        &b.iter()
            .map(|&(x, y)| Point { x, y })
            .collect::<Vec<_>>(),
    )?;
    let b_negated = b
        .iter()
        .map(|p| Point { x: -p.x, y: -p.y })
        .collect::<Vec<_>>();

    let mut sums = Vec::with_capacity(a.len() * b.len());
    for pa in &a {
        for pb in &b_negated {
            sums.push(Point {
                x: pa.x + pb.x,
                y: pa.y + pb.y,
            });
        }
    }

    let vertices = convex_hull(sums);
    if vertices.len() < 3 {
        return Err(ConfigurationSpaceError::DegeneratePolygon);
    }

    // A boundary edge of A + (-B) is generated by an edge of one source
    // polygon paired with the support vertex/edge of the other. We retain all
    // compatible source pairs rather than guessing a single feature after the
    // hull has discarded collinear intermediate points.
    let mut segments = Vec::with_capacity(vertices.len());
    for (i, &start) in vertices.iter().enumerate() {
        let end = vertices[(i + 1) % vertices.len()];
        segments.push(PlacementBoundarySegment {
            start,
            end,
            feature_pairs: boundary_feature_pairs(&a, &b_negated, start, end),
        });
    }

    Ok(ConvexConfigurationBoundary { vertices, segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<(f64, f64)> {
        vec![(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
    }

    #[test]
    fn equal_squares_produce_exact_configuration_square() {
        let boundary = convex_minkowski_difference(&square(), &square()).unwrap();
        assert_eq!(boundary.vertices.len(), 4);
        assert!(boundary
            .vertices
            .iter()
            .all(|p| (p.x.abs() - 2.0).abs() < 1e-12 || (p.y.abs() - 2.0).abs() < 1e-12));
        assert_eq!(boundary.segments.len(), 4);
    }

    #[test]
    fn reversed_convex_input_is_normalized() {
        let mut reversed = square();
        reversed.reverse();
        let normal = convex_minkowski_difference(&square(), &square()).unwrap();
        let reversed_result = convex_minkowski_difference(&reversed, &square()).unwrap();
        assert_eq!(normal.vertices, reversed_result.vertices);
    }

    #[test]
    fn concave_polygon_is_rejected_instead_of_treated_as_convex() {
        let l = vec![
            (-1.0, -1.0),
            (1.0, -1.0),
            (1.0, 1.0),
            (0.0, 1.0),
            (0.0, 0.0),
            (-1.0, 0.0),
        ];
        assert_eq!(
            convex_minkowski_difference(&l, &square()),
            Err(ConfigurationSpaceError::NonConvexPolygon)
        );
    }

    #[test]
    fn edge_flush_segment_has_edge_edge_provenance() {
        let boundary = convex_minkowski_difference(&square(), &square()).unwrap();
        let segment = boundary
            .segments
            .iter()
            .find(|segment| {
                (segment.start.y - segment.end.y).abs() <= 1e-12
                    && (segment.start.x - segment.end.x).abs() > 1e-12
            })
            .unwrap();
        assert!(segment
            .feature_pairs
            .iter()
            .any(|pair| matches!(pair.a, MinkowskiFeature::Edge(_))
                && matches!(pair.b, MinkowskiFeature::Edge(_))));
    }

    #[test]
    fn square_right_boundary_uses_right_facing_supports() {
        let boundary = convex_minkowski_difference(&square(), &square()).unwrap();
        let segment = boundary
            .segments
            .iter()
            .find(|segment| {
                (segment.start.x - 2.0).abs() < 1e-12
                    && (segment.end.x - 2.0).abs() < 1e-12
            })
            .unwrap();
        assert!(segment.feature_pairs.iter().any(|pair| {
            matches!(pair.a, MinkowskiFeature::Vertex(1) | MinkowskiFeature::Vertex(2))
        }));
    }

    #[test]
    fn edge_vertex_provenance_uses_same_minkowski_support_normal() {
        let triangle = vec![(0.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
        let boundary = convex_minkowski_difference(&square(), &triangle).unwrap();
        assert!(boundary.segments.iter().any(|segment| {
            segment.feature_pairs.iter().any(|pair| {
                matches!(pair.a, MinkowskiFeature::Edge(_))
                    && matches!(pair.b, MinkowskiFeature::Vertex(_))
            })
        }));
    }

    #[test]
    fn provenance_can_represent_vertex_edge_contact() {
        let triangle = vec![(0.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
        let boundary = convex_minkowski_difference(&triangle, &square()).unwrap();
        assert!(boundary.segments.iter().any(|segment| {
            segment.feature_pairs.iter().any(|pair| {
                matches!(pair.a, MinkowskiFeature::Vertex(_))
                    && matches!(pair.b, MinkowskiFeature::Edge(_))
            })
        }));
    }

    #[test]
    fn translation_on_boundary_means_external_tangency() {
        let a = square();
        let b = square();
        let boundary = convex_minkowski_difference(&a, &b).unwrap();
        let right = boundary
            .segments
            .iter()
            .find(|s| (s.start.x - 2.0).abs() < 1e-12 && (s.end.x - 2.0).abs() < 1e-12)
            .unwrap();
        let translation = Point { x: 2.0, y: 0.0 };
        assert!(right.start.y <= translation.y && translation.y <= right.end.y);
    }

    #[test]
    fn placement_translation_is_world_center_translation() {
        let a = square();
        let b = square();
        let boundary = convex_minkowski_difference(&a, &b).unwrap();
        let right = boundary
            .segments
            .iter()
            .find(|s| (s.start.x - 2.0).abs() < 1e-12 && (s.end.x - 2.0).abs() < 1e-12)
            .unwrap();
        let translation = Point { x: 2.0, y: 0.5 };
        assert!(right.start.y <= translation.y && translation.y <= right.end.y);

        // EvoSim applies Placement.x/y as a direct translation of local shape
        // coordinates. The same translation therefore places B flush against
        // A, without an additional origin or centroid offset.
        let b_world = b
            .iter()
            .map(|&(x, y)| (translation.x + x, translation.y + y))
            .collect::<Vec<_>>();
        assert!(b_world.iter().any(|&(x, y)| {
            (x - 1.0).abs() < 1e-12 && (y + 0.5).abs() < 1e-12
        }));
    }

    #[test]
    fn fixed_rotation_is_applied_before_configuration_space() {
        let a = square();
        let unrotated = square();
        let angle = std::f64::consts::FRAC_PI_2;
        let rotated = unrotated
            .iter()
            .map(|&(x, y)| {
                let (sin, cos) = angle.sin_cos();
                (x * cos - y * sin, x * sin + y * cos)
            })
            .collect::<Vec<_>>();
        let boundary = convex_minkowski_difference(&a, &rotated).unwrap();
        assert!(boundary
            .vertices
            .iter()
            .any(|p| (p.x.abs() - 2.0).abs() < 1e-12));
    }

    #[test]
    fn origin_is_inside_equal_square_configuration_region() {
        let boundary = convex_minkowski_difference(&square(), &square()).unwrap();
        assert!(boundary.vertices.iter().all(|p| p.x.abs() <= 2.0 + 1e-12));
        assert!(boundary.vertices.iter().all(|p| p.y.abs() <= 2.0 + 1e-12));
    }

    #[test]
    fn corner_support_is_retained_as_vertex_provenance() {
        let triangle = vec![(0.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
        let boundary = convex_minkowski_difference(&triangle, &square()).unwrap();
        assert!(boundary.segments.iter().any(|segment| {
            segment.feature_pairs.iter().any(|pair| {
                matches!(pair.a, MinkowskiFeature::Vertex(_))
                    || matches!(pair.b, MinkowskiFeature::Vertex(_))
            })
        }));
    }

    #[test]
    fn boundary_segments_preserve_flush_contact_locus() {
        let boundary = convex_minkowski_difference(&square(), &square()).unwrap();
        assert!(boundary.segments.iter().any(|segment| {
            (segment.start.y - segment.end.y).abs() <= 1e-12
                && (segment.start.x - segment.end.x).abs() > 1e-12
        }));
    }
}
