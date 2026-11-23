//! Checked physical support-area adapter for distributed in-plane sources.
//!
//! Phase 3A.4d deliberately stops at geometry. It measures the union of the
//! source's actual polygonal support region in world coordinates; it does not
//! register a source, evaluate power density, or transfer energy.

use crate::phase3_union::checked_polygon_union;
use crate::resources::Form;

/// Area and estimated numerical uncertainty for a source's 2D support region.
/// Both values are in squared world units (L²).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SourceFootprintArea {
    pub(crate) square_units: f64,
    pub(crate) uncertainty: f64,
}

/// Measure the union of polygonal source-support pieces expressed in world
/// coordinates. Overlap and coincident pieces are counted once.
///
/// The input is geometry only: each polygon is an ordered list of world-space
/// vertices. Rectangles or other placed forms should be transformed into this
/// common coordinate frame before calling. Unsupported, malformed, concave,
/// degenerate, or numerically unresolved geometry fails closed.
pub(crate) fn checked_source_footprint_area(
    world_polygons: &[Vec<(f64, f64)>],
) -> Option<SourceFootprintArea> {
    if world_polygons.is_empty() {
        return None;
    }

    let forms: Vec<Form> = world_polygons
        .iter()
        .map(|vertices| Form::Polygon {
            vertices: vertices.clone(),
        })
        .collect();

    let union = checked_polygon_union(&forms)?;
    Some(SourceFootprintArea {
        square_units: union.area.square_units,
        uncertainty: union.area.uncertainty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle(x0: f64, y0: f64, width: f64, height: f64) -> Vec<(f64, f64)> {
        vec![
            (x0, y0),
            (x0 + width, y0),
            (x0 + width, y0 + height),
            (x0, y0 + height),
        ]
    }

    #[test]
    fn one_polygon_reports_its_physical_support_area() {
        let area = checked_source_footprint_area(&[rectangle(-2.0, 3.0, 4.0, 3.0)])
            .expect("valid polygon support must have a checked area");
        assert!((area.square_units - 12.0).abs() <= area.uncertainty + 1.0e-10);
    }

    #[test]
    fn disjoint_and_overlapping_support_pieces_use_union_area() {
        let disjoint = checked_source_footprint_area(&[
            rectangle(0.0, 0.0, 2.0, 2.0),
            rectangle(3.0, 0.0, 2.0, 2.0),
        ])
        .expect("disjoint polygon supports must be measurable");
        assert!((disjoint.square_units - 8.0).abs() <= disjoint.uncertainty + 1.0e-9);

        let overlapping = checked_source_footprint_area(&[
            rectangle(0.0, 0.0, 2.0, 2.0),
            rectangle(1.0, 0.0, 2.0, 2.0),
        ])
        .expect("overlapping polygon supports must be unioned");
        assert!((overlapping.square_units - 6.0).abs() <= overlapping.uncertainty + 1.0e-9);
    }

    #[test]
    fn coincident_support_pieces_are_not_double_counted() {
        let polygon = rectangle(-1.0, -1.0, 2.0, 2.0);
        let area = checked_source_footprint_area(&[polygon.clone(), polygon])
            .expect("coincident support must resolve to one geometric region");
        assert!((area.square_units - 4.0).abs() <= area.uncertainty + 1.0e-9);
    }

    #[test]
    fn area_is_translation_and_scale_consistent() {
        let base = rectangle(0.0, 0.0, 4.0, 2.0);
        let translated = rectangle(1.0e8, -1.0e8, 4.0, 2.0);
        let scaled = rectangle(0.0, 0.0, 40.0, 20.0);

        let base_area = checked_source_footprint_area(&[base]).unwrap();
        let translated_area = checked_source_footprint_area(&[translated]).unwrap();
        let scaled_area = checked_source_footprint_area(&[scaled]).unwrap();

        assert!((base_area.square_units - 8.0).abs() <= base_area.uncertainty + 1.0e-9);
        assert!(
            (translated_area.square_units - 8.0).abs()
                <= translated_area.uncertainty + 1.0e-6
        );
        assert!((scaled_area.square_units - 800.0).abs() <= scaled_area.uncertainty + 1.0e-7);
    }

    #[test]
    fn invalid_or_unsupported_support_fails_closed() {
        assert!(checked_source_footprint_area(&[]).is_none());
        assert!(checked_source_footprint_area(&[vec![(0.0, 0.0), (1.0, 0.0)]]).is_none());
        assert!(checked_source_footprint_area(&[vec![
            (0.0, 0.0),
            (2.0, 0.0),
            (1.0, 1.0),
            (2.0, 2.0),
            (0.0, 2.0),
        ]])
        .is_none());
    }
}
