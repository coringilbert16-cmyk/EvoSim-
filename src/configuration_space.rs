//! Configuration-space geometry for rigid placement.
//!
//! For a fixed orientation, the touching translations of a convex polygon B
//! around a convex polygon A are the boundary of the Minkowski difference
//! A + (-B). Boundary segments are retained because an edge-flush contact is
//! a continuous placement locus, not an arbitrary point sample.
//!
//! Concave polygons are deliberately rejected here until their general
//! configuration-space construction is separately audited.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementBoundarySegment {
    pub start: Point,
    pub end: Point,
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
            .map(|&(x, y)| Point { x: -x, y: -y })
            .collect::<Vec<_>>(),
    )?;

    let mut sums = Vec::with_capacity(a.len() * b.len());
    for pa in &a {
        for pb in &b {
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
    let segments = vertices
        .iter()
        .enumerate()
        .map(|(i, &start)| PlacementBoundarySegment {
            start,
            end: vertices[(i + 1) % vertices.len()],
        })
        .collect();
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
    fn boundary_segments_preserve_flush_contact_locus() {
        let boundary = convex_minkowski_difference(&square(), &square()).unwrap();
        assert!(boundary.segments.iter().any(|segment| {
            (segment.start.y - segment.end.y).abs() <= 1e-12
                && (segment.start.x - segment.end.x).abs() > 1e-12
        }));
    }
}
