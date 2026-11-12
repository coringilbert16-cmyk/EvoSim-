//! Explicit chemical connection faces, separate from physical collision boundaries.
//!
//! A face is an eligible boundary segment. Its two termini are valid discrete
//! connection endpoints; later snapping/occupancy logic may select positions
//! along the face without changing the physical collision shape.
use crate::resources::{Form, Shape};
use crate::structure::Placement;

const GEOMETRY_EPSILON: f64 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceEnd {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionFace {
    /// Stable edge index in the shape's polygon vertex ordering.
    pub face_index: usize,
    pub start: (f64, f64),
    pub end: (f64, f64),
    /// Unit outward normal in the shape's local coordinate frame.
    pub normal: (f64, f64),
    pub length: f64,
}

impl ConnectionFace {
    pub fn endpoint(self, end: FaceEnd) -> (f64, f64) {
        match end {
            FaceEnd::Start => self.start,
            FaceEnd::End => self.end,
        }
    }

    pub fn world_endpoint(self, end: FaceEnd, placement: Placement) -> (f64, f64) {
        let (x, y) = self.endpoint(end);
        let (sin, cos) = placement.rotation_radians.sin_cos();
        (
            placement.x + x * cos - y * sin,
            placement.y + x * sin + y * cos,
        )
    }

    pub fn world_normal(self, placement: Placement) -> (f64, f64) {
        let (sin, cos) = placement.rotation_radians.sin_cos();
        (
            self.normal.0 * cos - self.normal.1 * sin,
            self.normal.0 * sin + self.normal.1 * cos,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionFaceEndpoint {
    pub face_index: usize,
    pub end: FaceEnd,
}

impl ConnectionFaceEndpoint {
    pub fn local_point(self, faces: &[ConnectionFace]) -> Option<(f64, f64)> {
        faces
            .iter()
            .find(|face| face.face_index == self.face_index)
            .map(|face| face.endpoint(self.end))
    }

    pub fn world_point(
        self,
        faces: &[ConnectionFace],
        placement: Placement,
    ) -> Option<(f64, f64)> {
        faces
            .iter()
            .find(|face| face.face_index == self.face_index)
            .map(|face| face.world_endpoint(self.end, placement))
    }
}

/// Return the chemically eligible straight faces of a rigid shape.
///
/// Physical collision boundaries are unchanged. Hydrogen's 1.0 x 0.1
/// rectangle exposes its two longitudinal 1.0-length sides; its two 0.1
/// end faces are deliberately not chemical connection faces. Other polygonal
/// shapes expose each of their boundary edges. Curved and fluid boundaries
/// remain continuous surfaces and are not discretized by this API.
pub fn connection_faces(shape: &Shape, resource_name: &str) -> Vec<ConnectionFace> {
    let Some(vertices) = shape.form.polygon_vertices() else {
        return Vec::new();
    };
    if vertices.len() < 3 {
        return Vec::new();
    }

    let signed_area_twice: f64 = (0..vertices.len())
        .map(|index| {
            let a = vertices[index];
            let b = vertices[(index + 1) % vertices.len()];
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    if !signed_area_twice.is_finite() || signed_area_twice.abs() <= GEOMETRY_EPSILON {
        return Vec::new();
    }
    let winding = signed_area_twice.signum();

    let hydrogen = resource_name == "Hydrogen";
    let longest_hydrogen_face = if hydrogen {
        match shape.form {
            Form::Rectangle { width, height } => Some(width.max(height)),
            _ => None,
        }
    } else {
        None
    };

    (0..vertices.len())
        .filter_map(|face_index| {
            let start = vertices[face_index];
            let end = vertices[(face_index + 1) % vertices.len()];
            let dx = end.0 - start.0;
            let dy = end.1 - start.1;
            let length = dx.hypot(dy);
            if !length.is_finite() || length <= GEOMETRY_EPSILON {
                return None;
            }

            if let Some(longest) = longest_hydrogen_face {
                if (length - longest).abs() > GEOMETRY_EPSILON * longest.max(1.0) {
                    return None;
                }
            }

            // For counter-clockwise polygons, the outward normal is the
            // right-hand perpendicular to each directed edge; reverse it
            // for clockwise vertex orderings.
            let direction = -winding;
            let normal = (direction * dy / length, direction * -dx / length);
            Some(ConnectionFace {
                face_index,
                start,
                end,
                normal,
                length,
            })
        })
        .collect()
}

/// Enumerate only the two termini belonging to each eligible connection face.
pub fn valid_face_endpoints(faces: &[ConnectionFace]) -> Vec<ConnectionFaceEndpoint> {
    let mut endpoints = Vec::with_capacity(faces.len() * 2);
    for face in faces {
        endpoints.push(ConnectionFaceEndpoint {
            face_index: face.face_index,
            end: FaceEnd::Start,
        });
        endpoints.push(ConnectionFaceEndpoint {
            face_index: face.face_index,
            end: FaceEnd::End,
        });
    }
    endpoints
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Form;
    use std::f64::consts::FRAC_PI_2;

    fn rectangle(width: f64, height: f64) -> Shape {
        Shape {
            form: Form::Rectangle { width, height },
        }
    }

    #[test]
    fn hydrogen_exposes_longitudinal_faces_not_short_end_faces() {
        let faces = connection_faces(&rectangle(1.0, 0.1), "Hydrogen");
        assert_eq!(faces.len(), 2);
        assert!(faces.iter().all(|face| (face.length - 1.0).abs() < 1e-12));
        assert_eq!(faces[0].face_index, 0);
        assert_eq!(faces[1].face_index, 2);
        assert!(faces.iter().all(|face| face.normal.1.abs() > 0.999999));
    }

    #[test]
    fn ordinary_rectangle_exposes_all_four_faces() {
        let faces = connection_faces(&rectangle(1.0, 0.5), "Phosphorus");
        assert_eq!(faces.len(), 4);
        assert_eq!(valid_face_endpoints(&faces).len(), 8);
    }

    #[test]
    fn face_endpoints_transform_with_rigid_placement() {
        let faces = connection_faces(&rectangle(1.0, 0.1), "Hydrogen");
        let placement = Placement {
            x: 10.0,
            y: 20.0,
            rotation_radians: FRAC_PI_2,
        };
        let start = ConnectionFaceEndpoint {
            face_index: 0,
            end: FaceEnd::Start,
        }
        .world_point(&faces, placement)
        .unwrap();
        assert!((start.0 - 10.05).abs() < 1e-12);
        assert!((start.1 - 19.5).abs() < 1e-12);
    }

    #[test]
    fn continuous_and_line_shapes_do_not_create_discrete_faces() {
        let line = Shape {
            form: Form::Line { length: 1.0 },
        };
        let circle = Shape {
            form: Form::Circle { radius: 0.5 },
        };
        assert!(connection_faces(&line, "Carbon").is_empty());
        assert!(connection_faces(&circle, "Carbon").is_empty());
    }
}
