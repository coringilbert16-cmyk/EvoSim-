//! Physical geometry queries for genome-core construction.
//!
//! The genome core is not identified by a blueprint flag. It is identified by
//! an enclosed empty region whose area is strictly larger than the area of an
//! equilateral triangle with side equal to one carbon diameter.

use std::collections::VecDeque;

use crate::resources::{BaseResource, Form};
use crate::structure::{OrganismStructure, Placement};

const MAX_GRID_AXIS: usize = 256;
const GRID_DIVISIONS_PER_CARBON_RADIUS: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CavityMeasurement {
    pub largest_enclosed_area: f64,
    pub threshold_area: f64,
    pub qualifies: bool,
}

/// Derive the minimum qualifying cavity area from the actual Carbon geometry.
/// No biological cavity-size constant is introduced here.
pub fn cavity_threshold_area(catalog: &[BaseResource]) -> Result<f64, String> {
    let carbon = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .ok_or_else(|| "genome-core construction requires Carbon geometry".to_string())?;

    let radius = carbon.shape.form.bounding_radius();
    if !radius.is_finite() || radius <= 0.0 {
        return Err("Carbon geometry has an invalid bounding radius".to_string());
    }

    let side = 2.0 * radius;
    Ok((3.0_f64.sqrt() / 4.0) * side * side)
}

fn point_inside_form(x: f64, y: f64, form: &Form, placement: Placement) -> bool {
    let (sin, cos) = (-placement.rotation_radians).sin_cos();
    let dx = x - placement.x;
    let dy = y - placement.y;
    let local_x = dx * cos - dy * sin;
    let local_y = dx * sin + dy * cos;

    match form {
        Form::Circle { radius } => local_x.hypot(local_y) <= *radius,
        Form::Rectangle { width, height } => {
            local_x.abs() <= *width / 2.0 && local_y.abs() <= *height / 2.0
        }
        Form::RegularPolygon { sides, radius } => {
            polygon_contains(local_x, local_y, &regular_polygon_vertices(*sides, *radius))
        }
        Form::Polygon { vertices } => polygon_contains(local_x, local_y, vertices),
        Form::Line { .. } | Form::Fluid { .. } => false,
    }
}

fn regular_polygon_vertices(sides: u8, radius: f64) -> Vec<(f64, f64)> {
    (0..sides as usize)
        .map(|index| {
            let angle = index as f64 * std::f64::consts::TAU / sides as f64;
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

fn polygon_contains(x: f64, y: f64, vertices: &[(f64, f64)]) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    let mut inside = false;
    for index in 0..vertices.len() {
        let (ax, ay) = vertices[index];
        let (bx, by) = vertices[(index + 1) % vertices.len()];
        if (ay > y) != (by > y) {
            let intersection_x = (bx - ax) * (y - ay) / (by - ay) + ax;
            if x < intersection_x {
                inside = !inside;
            }
        }
    }
    inside
}

/// Measure enclosed empty area using a resolution derived from Carbon's actual
/// geometry. The grid is a measurement approximation, not an additional
/// biological rule; the authoritative threshold remains geometric.
pub fn measure_largest_cavity(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Result<CavityMeasurement, String> {
    let threshold = cavity_threshold_area(catalog)?;
    if structure.units.is_empty() {
        return Ok(CavityMeasurement {
            largest_enclosed_area: 0.0,
            threshold_area: threshold,
            qualifies: false,
        });
    }

    let carbon_radius = (threshold / (3.0_f64.sqrt())).sqrt();
    let cell_size = (carbon_radius / GRID_DIVISIONS_PER_CARBON_RADIUS).max(1.0e-3);

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for unit in &structure.units {
        let Some(shape) = unit.shape(catalog) else { continue };
        let radius = shape.form.bounding_radius();
        min_x = min_x.min(unit.placement.x - radius);
        max_x = max_x.max(unit.placement.x + radius);
        min_y = min_y.min(unit.placement.y - radius);
        max_y = max_y.max(unit.placement.y + radius);
    }

    if !min_x.is_finite() || !max_x.is_finite() || !min_y.is_finite() || !max_y.is_finite() {
        return Ok(CavityMeasurement {
            largest_enclosed_area: 0.0,
            threshold_area: threshold,
            qualifies: false,
        });
    }

    let margin = 2.0 * cell_size;
    min_x -= margin;
    max_x += margin;
    min_y -= margin;
    max_y += margin;

    let nx = (((max_x - min_x) / cell_size).ceil() as usize).clamp(3, MAX_GRID_AXIS);
    let ny = (((max_y - min_y) / cell_size).ceil() as usize).clamp(3, MAX_GRID_AXIS);
    let sx = (max_x - min_x) / nx as f64;
    let sy = (max_y - min_y) / ny as f64;

    let mut occupied = vec![false; nx * ny];
    for iy in 0..ny {
        for ix in 0..nx {
            let x = min_x + (ix as f64 + 0.5) * sx;
            let y = min_y + (iy as f64 + 0.5) * sy;
            occupied[iy * nx + ix] = structure.units.iter().any(|unit| {
                unit.shape(catalog)
                    .map(|shape| point_inside_form(x, y, &shape.form, unit.placement))
                    .unwrap_or(false)
            });
        }
    }

    let mut exterior = vec![false; nx * ny];
    let mut queue = VecDeque::new();

    for iy in 0..ny {
        for ix in 0..nx {
            if ix != 0 && iy != 0 && ix + 1 != nx && iy + 1 != ny {
                continue;
            }
            let index = iy * nx + ix;
            if !occupied[index] && !exterior[index] {
                exterior[index] = true;
                queue.push_back((ix, iy));
            }
        }
    }

    flood_empty(&occupied, &mut exterior, nx, ny, &mut queue);

    let mut visited = vec![false; nx * ny];
    let mut largest_area = 0.0;

    for iy in 0..ny {
        for ix in 0..nx {
            let start = iy * nx + ix;
            if occupied[start] || exterior[start] || visited[start] {
                continue;
            }

            visited[start] = true;
            let mut component = VecDeque::from([(ix, iy)]);
            let mut cells = 0usize;

            while let Some((x, y)) = component.pop_front() {
                cells += 1;
                for (dx, dy) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
                    let nxp = x as i32 + dx;
                    let nyp = y as i32 + dy;
                    if nxp < 0 || nyp < 0 || nxp >= nx as i32 || nyp >= ny as i32 {
                        continue;
                    }
                    let index = nyp as usize * nx + nxp as usize;
                    if !occupied[index] && !exterior[index] && !visited[index] {
                        visited[index] = true;
                        component.push_back((nxp as usize, nyp as usize));
                    }
                }
            }

            largest_area = largest_area.max(cells as f64 * sx * sy);
        }
    }

    Ok(CavityMeasurement {
        largest_enclosed_area: largest_area,
        threshold_area: threshold,
        qualifies: largest_area > threshold,
    })
}

fn flood_empty(
    occupied: &[bool],
    exterior: &mut [bool],
    width: usize,
    height: usize,
    queue: &mut VecDeque<(usize, usize)>,
) {
    while let Some((x, y)) = queue.pop_front() {
        for (dx, dy) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let index = ny as usize * width + nx as usize;
            if !occupied[index] && !exterior[index] {
                exterior[index] = true;
                queue.push_back((nx as usize, ny as usize));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::OrganismStructure;

    #[test]
    fn threshold_is_derived_from_catalog_carbon_geometry() {
        let catalog = default_catalog();
        let carbon = catalog.iter().find(|resource| resource.name == "Carbon").unwrap();
        let radius = carbon.shape.form.bounding_radius();
        let expected = 3.0_f64.sqrt() * radius * radius;
        assert!((cavity_threshold_area(&catalog).unwrap() - expected).abs() < 1.0e-12);
    }

    #[test]
    fn empty_structure_has_no_cavity() {
        let result = measure_largest_cavity(&OrganismStructure::new(), &default_catalog()).unwrap();
        assert_eq!(result.largest_enclosed_area, 0.0);
        assert!(!result.qualifies);
    }
}
