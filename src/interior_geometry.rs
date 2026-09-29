//! General realized interior topology derived from the physical boundary.
use crate::resources::{BaseResource, Form};
use crate::structure::{OrganismStructure, Placement};
use std::collections::HashMap;
use std::f64::consts::TAU;

const EPS: f64 = 1e-8;
const NODE_TOLERANCE: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point { x: f64, y: f64 }
impl Point {
    fn sub(self, o: Self) -> Self { Self { x: self.x-o.x, y: self.y-o.y } }
    fn add(self, o: Self) -> Self { Self { x: self.x+o.x, y: self.y+o.y } }
    fn scale(self, f: f64) -> Self { Self { x: self.x*f, y: self.y*f } }
    fn cross(self, o: Self) -> f64 { self.x*o.y-self.y*o.x }
    fn norm(self) -> f64 { self.x.hypot(self.y) }
}
#[derive(Clone, Copy, Debug)] struct Edge { from: usize, to: usize }

/// A finite region enclosed by the realized structural boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct EnclosedRegion {
    pub area: f64,
    pub boundary_units: Vec<usize>,
    pub sample_point: (f64, f64),
}

/// Find finite enclosed regions of a realized organism.
///
/// This deliberately remains separate from genome-cavity qualification. Fluid
/// forms are not converted into artificial rigid polygons here; connected Water
/// receives context-fitting realization in the next implementation stage.
/// Number of one-unit Water constituents needed to cover an enclosed region
/// at the Water resource's nominal physical area. This is planning information;
/// it does not create Water or mutate the organism.
pub fn required_water_units(region: &EnclosedRegion, water_nominal_area: f64) -> usize {
    if !region.area.is_finite() || region.area <= 0.0 || !water_nominal_area.is_finite() || water_nominal_area <= 0.0 {
        return 0;
    }
    (region.area / water_nominal_area).ceil() as usize
}

pub fn find_enclosed_regions(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Vec<EnclosedRegion> {
    let mut polygons = Vec::<(usize, Vec<Point>)>::new();
    for index in structure.structural_unit_indices(catalog) {
        let unit = &structure.units[index];
        let Some(geometry) = unit.geometry.as_ref() else { continue };
        let Some(polygon) = transformed_polygon(&geometry.shape().form, unit.placement) else { continue };
        if polygon.len() >= 3 { polygons.push((index, polygon)); }
    }
    if polygons.is_empty() { return Vec::new(); }

    let mut points = Vec::new();
    let mut point_index = HashMap::new();
    let mut edges = Vec::new();
    for (_, polygon) in &polygons {
        for i in 0..polygon.len() {
            let a = intern(polygon[i], &mut points, &mut point_index);
            let b = intern(polygon[(i+1)%polygon.len()], &mut points, &mut point_index);
            if a != b {
                edges.push(Edge { from:a, to:b });
                edges.push(Edge { from:b, to:a });
            }
        }
    }
    if edges.is_empty() { return Vec::new(); }

    let mut outgoing = vec![Vec::new(); points.len()];
    for (i,e) in edges.iter().enumerate() { outgoing[e.from].push(i); }
    let mut visited = vec![false; edges.len()];
    let mut regions = Vec::new();

    for start in 0..edges.len() {
        if visited[start] { continue; }
        let mut face = Vec::new();
        let mut current = start;
        let mut closed = false;
        for _ in 0..=edges.len() {
            if current == start && !face.is_empty() { closed=true; break; }
            if visited[current] { break; }
            visited[current]=true;
            face.push(current);
            let edge=edges[current];
            let incoming=points[edge.to].sub(points[edge.from]);
            let reverse_angle=(incoming.y.atan2(incoming.x)+std::f64::consts::PI).rem_euclid(TAU);
            let mut next=None;
            let mut best_turn=f64::INFINITY;
            for &candidate in &outgoing[edge.to] {
                if candidate==(current^1) || (visited[candidate] && candidate!=start) { continue; }
                let e=edges[candidate];
                let direction=points[e.to].sub(points[e.from]);
                let angle=direction.y.atan2(direction.x).rem_euclid(TAU);
                let turn=(reverse_angle-angle).rem_euclid(TAU);
                if turn<best_turn { best_turn=turn; next=Some(candidate); }
            }
            let Some(next)=next else { break };
            current=next;
        }
        if !closed || face.len()<3 { continue; }
        let area=face.iter().map(|&i| points[edges[i].from].cross(points[edges[i].to])).sum::<f64>()*0.5;
        if area<=EPS { continue; }

        let a=points[edges[face[0]].from];
        let b=points[edges[face[0]].to];
        let tangent=b.sub(a);
        let length=tangent.norm();
        if length<=EPS { continue; }
        let sample=a.add(b).scale(0.5).add(Point{x:-tangent.y/length,y:tangent.x/length}.scale(NODE_TOLERANCE*10.0));
        if polygons.iter().any(|(_,p)| point_in_polygon(sample,p)) { continue; }

        let mut boundary_units=Vec::new();
        for &edge_index in &face {
            let a=points[edges[edge_index].from];
            let b=points[edges[edge_index].to];
            if let Some(unit)=polygons.iter().find_map(|(unit,p)| segment_in_polygon_boundary(a,b,p).then_some(*unit)) {
                if !boundary_units.contains(&unit) { boundary_units.push(unit); }
            }
        }
        if boundary_units.is_empty() { continue; }
        regions.push(EnclosedRegion { area, boundary_units, sample_point:(sample.x,sample.y) });
    }
    regions.sort_by(|a,b| b.area.total_cmp(&a.area));
    regions
}

fn intern(point:Point, points:&mut Vec<Point>, index:&mut HashMap<(i64,i64),usize>)->usize {
    let key=((point.x/NODE_TOLERANCE).round() as i64,(point.y/NODE_TOLERANCE).round() as i64);
    if let Some(&i)=index.get(&key) {
        if points[i].sub(point).norm()<=NODE_TOLERANCE { return i; }
    }
    let i=points.len(); points.push(point); index.insert(key,i); i
}
fn transformed_polygon(form:&Form, placement:Placement)->Option<Vec<Point>> {
    let vertices=form.polygon_vertices()?;
    let (s,c)=placement.rotation_radians.sin_cos();
    Some(vertices.into_iter().map(|(x,y)| Point{x:placement.x+x*c-y*s,y:placement.y+x*s+y*c}).collect())
}
fn point_in_polygon(point:Point, polygon:&[Point])->bool {
    let mut inside=false;
    for i in 0..polygon.len() {
        let a=polygon[i]; let b=polygon[(i+1)%polygon.len()];
        if (a.y>point.y)!=(b.y>point.y) && point.x<(b.x-a.x)*(point.y-a.y)/(b.y-a.y)+a.x { inside=!inside; }
    }
    inside
}
fn segment_in_polygon_boundary(a:Point,b:Point,polygon:&[Point])->bool {
    (0..polygon.len()).any(|i| {
        let p=polygon[i]; let q=polygon[(i+1)%polygon.len()];
        (p.sub(a).norm()<=NODE_TOLERANCE && q.sub(b).norm()<=NODE_TOLERANCE)
        || (p.sub(b).norm()<=NODE_TOLERANCE && q.sub(a).norm()<=NODE_TOLERANCE)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Vec<Point>> {
        vec![vec![
            Point { x: -1.0, y: -1.0 },
            Point { x: 1.0, y: -1.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: -1.0, y: 1.0 },
        ]]
    }

    fn region_count(polygons: Vec<Vec<Point>>) -> usize {
        let mut points = Vec::new();
        let mut point_index = HashMap::new();
        let mut edges = Vec::new();
        for (unit, polygon) in polygons.iter().enumerate() {
            for i in 0..polygon.len() {
                let a = intern(polygon[i], &mut points, &mut point_index);
                let b = intern(polygon[(i + 1) % polygon.len()], &mut points, &mut point_index);
                if a != b {
                    edges.push(Edge { from: a, to: b });
                    edges.push(Edge { from: b, to: a });
                }
            }
            let _ = unit;
        }
        let mut outgoing = vec![Vec::new(); points.len()];
        for (i, edge) in edges.iter().enumerate() { outgoing[edge.from].push(i); }
        let mut visited = vec![false; edges.len()];
        let mut count = 0;
        for start in 0..edges.len() {
            if visited[start] { continue; }
            let mut face = Vec::new();
            let mut current = start;
            let mut closed = false;
            for _ in 0..=edges.len() {
                if current == start && !face.is_empty() { closed = true; break; }
                if visited[current] { break; }
                visited[current] = true;
                face.push(current);
                let edge = edges[current];
                let incoming = points[edge.to].sub(points[edge.from]);
                let reverse_angle = (incoming.y.atan2(incoming.x) + std::f64::consts::PI).rem_euclid(TAU);
                let mut next = None;
                let mut best_turn = f64::INFINITY;
                for &candidate in &outgoing[edge.to] {
                    if candidate == (current ^ 1) || (visited[candidate] && candidate != start) { continue; }
                    let e = edges[candidate];
                    let direction = points[e.to].sub(points[e.from]);
                    let angle = direction.y.atan2(direction.x).rem_euclid(TAU);
                    let turn = (reverse_angle - angle).rem_euclid(TAU);
                    if turn < best_turn { best_turn = turn; next = Some(candidate); }
                }
                let Some(next) = next else { break };
                current = next;
            }
            if closed && face.len() >= 3 {
                let area = face.iter().map(|&i| points[edges[i].from].cross(points[edges[i].to])).sum::<f64>() * 0.5;
                if area > EPS {
                    let a = points[edges[face[0]].from];
                    let b = points[edges[face[0]].to];
                    let tangent = b.sub(a);
                    let length = tangent.norm();
                    if length > EPS {
                        let sample = a.add(b).scale(0.5).add(Point { x: -tangent.y / length, y: tangent.x / length }.scale(NODE_TOLERANCE * 10.0));
                        if !polygons.iter().any(|p| point_in_polygon(sample, p)) { count += 1; }
                    }
                }
            }
        }
        count
    }

    #[test]
    fn closed_polygon_has_an_enclosed_region() {
        assert_eq!(region_count(square()), 1);
    }

    #[test]
    fn water_quantity_is_derived_from_enclosed_area() {
        let region = EnclosedRegion {
            area: 2.1,
            boundary_units: vec![],
            sample_point: (0.0, 0.0),
        };
        assert_eq!(required_water_units(&region, 0.5), 5);
        assert_eq!(required_water_units(&region, 1.0), 3);
    }

    #[test]
    fn removing_one_wall_opens_the_region() {
        let mut p = square();
        p[0].pop();
        assert_eq!(region_count(p), 0);
    }
}
