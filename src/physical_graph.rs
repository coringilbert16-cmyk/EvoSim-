use crate::resources::{BaseResource, ConnectionSites, Form, Material};
use serde::{Deserialize, Serialize};

/// World placement of one physically instantiated constituent.
/// Physical identity does not depend on the legacy organism-structure layer.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

/// A physical region where a relationship may attach.
/// No authored occupancy limit exists: geometry determines admission.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ConnectionEndpoint {
    Corner { point_index: usize },
    Boundary { angle_radians: f64 },
}

impl ConnectionEndpoint {
    pub fn same_location(self, other: Self) -> bool {
        match (self, other) {
            (Self::Corner { point_index: a }, Self::Corner { point_index: b }) => a == b,
            (Self::Boundary { angle_radians: a }, Self::Boundary { angle_radians: b }) => {
                (a - b).abs() <= 1e-12
            }
            _ => false,
        }
    }

    fn world_point(
        self,
        placement: Placement,
        resource: &BaseResource,
    ) -> Option<crate::connection_geometry::WorldConnectionPoint> {
        let (local_x, local_y, normal_x, normal_y) = match self {
            Self::Corner { point_index } => {
                let ConnectionSites::Corners(points) = resource.shape.connection_sites() else {
                    return None;
                };
                let point = *points.get(point_index)?;
                (
                    point.x,
                    point.y,
                    point.direction_radians.cos(),
                    point.direction_radians.sin(),
                )
            }
            Self::Boundary { angle_radians } => {
                let ConnectionSites::Circumference { radius } = resource.shape.connection_sites() else {
                    return None;
                };
                let (sin, cos) = angle_radians.sin_cos();
                (radius * cos, radius * sin, cos, sin)
            }
        };
        let (sin, cos) = placement.rotation_radians.sin_cos();
        Some(crate::connection_geometry::WorldConnectionPoint {
            x: placement.x + local_x * cos - local_y * sin,
            y: placement.y + local_x * sin + local_y * cos,
            normal_x: normal_x * cos - normal_y * sin,
            normal_y: normal_x * sin + normal_y * cos,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhysicalConstituentId(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalConstituent {
    pub id: PhysicalConstituentId,
    pub resource_name: String,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalRelationship {
    pub constituent_a: PhysicalConstituentId,
    pub constituent_b: PhysicalConstituentId,
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub strength: f64,
    #[serde(default)]
    pub bond_energy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalAttachmentCandidate {
    pub constituent_a: PhysicalConstituentId,
    pub constituent_b: PhysicalConstituentId,
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub distance: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PhysicalConstituentGraph {
    constituents: Vec<PhysicalConstituent>,
    relationships: Vec<PhysicalRelationship>,
    next_id: u64,
}

impl PhysicalConstituentGraph {
    pub fn new() -> Self { Self::default() }

    pub fn constituents(&self) -> &[PhysicalConstituent] { &self.constituents }
    pub fn relationships(&self) -> &[PhysicalRelationship] { &self.relationships }

    pub fn constituent(&self, id: PhysicalConstituentId) -> Option<&PhysicalConstituent> {
        self.constituents.iter().find(|c| c.id == id)
    }

    pub fn constituent_mut(&mut self, id: PhysicalConstituentId) -> Option<&mut PhysicalConstituent> {
        self.constituents.iter_mut().find(|c| c.id == id)
    }

    pub fn add_constituent(
        &mut self,
        resource_name: impl Into<String>,
        placement: Placement,
    ) -> PhysicalConstituentId {
        let id = PhysicalConstituentId(self.next_id);
        self.next_id = self.next_id.checked_add(1).expect("physical constituent id overflow");
        self.constituents.push(PhysicalConstituent { id, resource_name: resource_name.into(), placement });
        id
    }

    /// Convert unit material into individual physical constituents.
    /// Existing legacy material bonds are deliberately not copied: physical
    /// relationships must be realized from actual placement and geometry.
    pub fn materialize_material(
        &mut self,
        material: &Material,
        placement: Placement,
    ) -> Result<Vec<PhysicalConstituentId>, &'static str> {
        if !material.is_valid() || material.is_empty() {
            return Err("material is invalid or empty");
        }
        if material.parts.iter().any(|(_, amount)| (*amount - 1.0).abs() > 1e-9) {
            return Err("physical materialization requires one unit per constituent");
        }
        Ok(material.parts.iter().map(|(name, _)| self.add_constituent(name.clone(), placement)).collect())
    }

    fn resource<'a>(&self, id: PhysicalConstituentId, catalog: &'a [BaseResource]) -> Option<&'a BaseResource> {
        let constituent = self.constituent(id)?;
        catalog.iter().find(|resource| resource.name == constituent.resource_name)
    }

    fn endpoint_candidates(
        a: &PhysicalConstituent,
        b: &PhysicalConstituent,
        resource_a: &BaseResource,
        resource_b: &BaseResource,
    ) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
        match (resource_a.shape.connection_sites(), resource_b.shape.connection_sites()) {
            (ConnectionSites::Corners(points_a), ConnectionSites::Corners(points_b)) => points_a
                .iter().enumerate().flat_map(|(ia, _)| {
                    points_b.iter().enumerate().map(move |(ib, _)| (
                        ConnectionEndpoint::Corner { point_index: ia },
                        ConnectionEndpoint::Corner { point_index: ib },
                    ))
                }).collect(),
            (ConnectionSites::Corners(points_a), ConnectionSites::Circumference { .. }) => points_a
                .iter().enumerate().map(|(ia, _)| {
                    let endpoint = ConnectionEndpoint::Corner { point_index: ia };
                    let world = endpoint.world_point(a.placement, resource_a).expect("valid corner");
                    (endpoint, Self::continuous_endpoint(b.placement, resource_b, world))
                }).collect(),
            (ConnectionSites::Circumference { .. }, ConnectionSites::Corners(points_b)) => points_b
                .iter().enumerate().map(|(ib, _)| {
                    let endpoint = ConnectionEndpoint::Corner { point_index: ib };
                    let world = endpoint.world_point(b.placement, resource_b).expect("valid corner");
                    (Self::continuous_endpoint(a.placement, resource_a, world), endpoint)
                }).collect(),
            (ConnectionSites::Circumference { .. }, ConnectionSites::Circumference { .. }) => {
                let target_b = crate::connection_geometry::WorldConnectionPoint {
                    x: b.placement.x, y: b.placement.y, normal_x: 0.0, normal_y: 0.0,
                };
                let target_a = crate::connection_geometry::WorldConnectionPoint {
                    x: a.placement.x, y: a.placement.y, normal_x: 0.0, normal_y: 0.0,
                };
                vec![(
                    Self::continuous_endpoint(a.placement, resource_a, target_b),
                    Self::continuous_endpoint(b.placement, resource_b, target_a),
                )]
            }
        }
    }

    fn continuous_endpoint(
        placement: Placement,
        resource: &BaseResource,
        target: crate::connection_geometry::WorldConnectionPoint,
    ) -> ConnectionEndpoint {
        let dx = target.x - placement.x;
        let dy = target.y - placement.y;
        let len = dx.hypot(dy);
        let (ux, uy) = if len > 1e-12 { (dx / len, dy / len) } else { (1.0, 0.0) };
        let (sin, cos) = placement.rotation_radians.sin_cos();
        let lx = ux * cos + uy * sin;
        let ly = -ux * sin + uy * cos;
        match resource.shape.connection_sites() {
            ConnectionSites::Circumference { .. } => ConnectionEndpoint::Boundary { angle_radians: ly.atan2(lx) },
            ConnectionSites::Corners(_) => unreachable!("continuous endpoint requested for corner-only resource"),
            ConnectionSites::Undetermined => unreachable!("all physical resources must now expose explicit geometry"),
        }
    }

    pub fn attachment_candidates(
        &self,
        constituent_a: PhysicalConstituentId,
        constituent_b: PhysicalConstituentId,
        catalog: &[BaseResource],
    ) -> Vec<PhysicalAttachmentCandidate> {
        let Some(a) = self.constituent(constituent_a) else { return Vec::new() };
        let Some(b) = self.constituent(constituent_b) else { return Vec::new() };
        let Some(resource_a) = self.resource(constituent_a, catalog) else { return Vec::new() };
        let Some(resource_b) = self.resource(constituent_b, catalog) else { return Vec::new() };
        Self::endpoint_candidates(a, b, resource_a, resource_b).into_iter().filter_map(|(ea, eb)| {
            let wa = ea.world_point(a.placement, resource_a)?;
            let wb = eb.world_point(b.placement, resource_b)?;
            Some(PhysicalAttachmentCandidate {
                constituent_a, constituent_b, endpoint_a: ea, endpoint_b: eb,
                distance: (wa.x - wb.x).hypot(wa.y - wb.y),
            })
        }).collect()
    }

    /// The graph is the physical admission authority. Endpoints must meet,
    /// rigid constituents may not strictly overlap, and relationship geometry
    /// may not cross or overlap another relationship. There is intentionally
    /// no bond-count limit at any endpoint.
    pub fn try_attach(
        &mut self,
        candidate: PhysicalAttachmentCandidate,
        strength: f64,
        bond_energy: f64,
        tolerance: f64,
        catalog: &[BaseResource],
    ) -> Result<usize, &'static str> {
        let tolerance = tolerance.max(0.0);
        if candidate.distance > tolerance { return Err("physical endpoints are not in contact"); }
        if !strength.is_finite() || strength < 0.0 || !bond_energy.is_finite() || bond_energy < 0.0 {
            return Err("attachment contains invalid physical values");
        }
        let Some(a) = self.constituent(candidate.constituent_a).cloned() else { return Err("unknown constituent") };
        let Some(b) = self.constituent(candidate.constituent_b).cloned() else { return Err("unknown constituent") };
        let Some(resource_a) = self.resource(a.id, catalog) else { return Err("unknown resource") };
        let Some(resource_b) = self.resource(b.id, catalog) else { return Err("unknown resource") };
        let Some(world_a) = candidate.endpoint_a.world_point(a.placement, resource_a) else { return Err("invalid endpoint") };
        let Some(world_b) = candidate.endpoint_b.world_point(b.placement, resource_b) else { return Err("invalid endpoint") };
        if (world_a.x - world_b.x).hypot(world_a.y - world_b.y) > tolerance { return Err("physical endpoints are not in contact"); }
        if rigid_constituents_overlap(&a, resource_a, &b, resource_b, 1e-10) {
            return Err("constituents strictly overlap");
        }

        let new_a = (world_a.x, world_a.y);
        let new_b = (world_b.x, world_b.y);
        for existing in &self.relationships {
            let Some(ea) = self.constituent(existing.constituent_a) else { return Err("invalid existing relationship") };
            let Some(eb) = self.constituent(existing.constituent_b) else { return Err("invalid existing relationship") };
            let Some(ra) = self.resource(existing.constituent_a, catalog) else { return Err("unknown resource") };
            let Some(rb) = self.resource(existing.constituent_b, catalog) else { return Err("unknown resource") };
            let Some(pa) = existing.endpoint_a.world_point(ea.placement, ra) else { return Err("invalid existing endpoint") };
            let Some(pb) = existing.endpoint_b.world_point(eb.placement, rb) else { return Err("invalid existing endpoint") };
            if segment_crosses_transversely(new_a, new_b, (pa.x, pa.y), (pb.x, pb.y), 1e-10)
                || collinear_overlap(new_a, new_b, (pa.x, pa.y), (pb.x, pb.y), 1e-10) {
                return Err("attachment conflicts with an existing relationship");
            }
        }
        self.add_relationship(PhysicalRelationship {
            constituent_a: candidate.constituent_a,
            constituent_b: candidate.constituent_b,
            endpoint_a: candidate.endpoint_a,
            endpoint_b: candidate.endpoint_b,
            strength,
            bond_energy,
        })
    }

    pub fn remove_constituent(&mut self, id: PhysicalConstituentId) -> Option<PhysicalConstituent> {
        let index = self.constituents.iter().position(|c| c.id == id)?;
        self.relationships.retain(|r| r.constituent_a != id && r.constituent_b != id);
        Some(self.constituents.remove(index))
    }

    pub fn add_relationship(&mut self, relationship: PhysicalRelationship) -> Result<usize, &'static str> {
        if relationship.constituent_a == relationship.constituent_b { return Err("relationship requires two distinct constituents"); }
        if self.constituent(relationship.constituent_a).is_none() || self.constituent(relationship.constituent_b).is_none() {
            return Err("relationship references an unknown constituent");
        }
        if !relationship.strength.is_finite() || relationship.strength < 0.0 || !relationship.bond_energy.is_finite() || relationship.bond_energy < 0.0 {
            return Err("relationship contains invalid physical values");
        }
        self.relationships.push(relationship);
        Ok(self.relationships.len() - 1)
    }

    pub fn remove_relationship(&mut self, index: usize) -> Option<PhysicalRelationship> {
        (index < self.relationships.len()).then(|| self.relationships.remove(index))
    }

    pub fn relationships_touching(&self, id: PhysicalConstituentId) -> impl Iterator<Item = &PhysicalRelationship> {
        self.relationships.iter().filter(move |r| r.constituent_a == id || r.constituent_b == id)
    }

    pub fn relationship_count_at(&self, id: PhysicalConstituentId, endpoint: ConnectionEndpoint) -> usize {
        self.relationships.iter().filter(|r| {
            (r.constituent_a == id && r.endpoint_a.same_location(endpoint)) ||
            (r.constituent_b == id && r.endpoint_b.same_location(endpoint))
        }).count()
    }

    pub fn connected_components(&self) -> Vec<Vec<PhysicalConstituentId>> {
        let mut components = Vec::new();
        let mut unseen: std::collections::HashSet<_> = self.constituents.iter().map(|c| c.id).collect();
        while let Some(start) = unseen.iter().next().copied() {
            let mut stack = vec![start];
            unseen.remove(&start);
            let mut component = Vec::new();
            while let Some(id) = stack.pop() {
                component.push(id);
                for relationship in self.relationships_touching(id) {
                    let other = if relationship.constituent_a == id { relationship.constituent_b } else { relationship.constituent_a };
                    if unseen.remove(&other) { stack.push(other); }
                }
            }
            component.sort_unstable();
            components.push(component);
        }
        components
    }

    pub fn validate_resource_names(&self, catalog: &[BaseResource]) -> bool {
        self.constituents.iter().all(|c| catalog.iter().any(|r| r.name == c.resource_name))
    }
}

fn rigid_constituents_overlap(
    a: &PhysicalConstituent,
    resource_a: &BaseResource,
    b: &PhysicalConstituent,
    resource_b: &BaseResource,
    epsilon: f64,
) -> bool {
    let ca = (a.placement.x, a.placement.y);
    let cb = (b.placement.x, b.placement.y);
    let distance = (ca.0 - cb.0).hypot(ca.1 - cb.1);
    let ra = resource_a.shape.form.bounding_radius();
    let rb = resource_b.shape.form.bounding_radius();
    if distance >= ra + rb - epsilon { return false; }

    match (&resource_a.shape.form, &resource_b.shape.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => distance < ar + br - epsilon,
        (Form::Circle { radius }, polygon) => circle_polygon_strict_overlap(&a.placement, *radius, &b.placement, polygon, epsilon),
        (polygon, Form::Circle { radius }) => circle_polygon_strict_overlap(&b.placement, *radius, &a.placement, polygon, epsilon),
        _ => polygon_strict_overlap(&a.placement, &resource_a.shape.form, &b.placement, &resource_b.shape.form, epsilon),
    }
}

fn world_vertices(form: &Form, placement: &Placement) -> Option<Vec<(f64, f64)>> {
    let vertices = form.polygon_vertices()?;
    let (sin, cos) = placement.rotation_radians.sin_cos();
    Some(vertices.into_iter().map(|(x, y)| (
        placement.x + x * cos - y * sin,
        placement.y + x * sin + y * cos,
    )).collect())
}

fn polygon_axes(vertices: &[(f64, f64)]) -> Vec<(f64, f64)> {
    vertices.iter().enumerate().map(|(i, &(x1, y1))| {
        let (x2, y2) = vertices[(i + 1) % vertices.len()];
        let ex = x2 - x1;
        let ey = y2 - y1;
        let len = ex.hypot(ey);
        (-ey / len, ex / len)
    }).collect()
}

fn project(vertices: &[(f64, f64)], axis: (f64, f64)) -> (f64, f64) {
    vertices.iter().map(|(x, y)| x * axis.0 + y * axis.1).fold(
        (f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), value| (lo.min(value), hi.max(value))
    )
}

fn polygon_strict_overlap(a: &Placement, form_a: &Form, b: &Placement, form_b: &Form, epsilon: f64) -> bool {
    let Some(va) = world_vertices(form_a, a) else { return false };
    let Some(vb) = world_vertices(form_b, b) else { return false };
    let mut axes = polygon_axes(&va);
    axes.extend(polygon_axes(&vb));
    axes.into_iter().all(|axis| {
        let (amin, amax) = project(&va, axis);
        let (bmin, bmax) = project(&vb, axis);
        amax > bmin + epsilon && bmax > amin + epsilon
    })
}

fn circle_polygon_strict_overlap(circle: &Placement, radius: f64, polygon: &Placement, form: &Form, epsilon: f64) -> bool {
    let Some(vertices) = world_vertices(form, polygon) else { return false };
    let center = (circle.x, circle.y);
    if point_in_polygon_strict(center, &vertices) { return true; }
    let expanded = radius - epsilon;
    if expanded <= 0.0 { return false; }
    vertices.iter().enumerate().any(|(i, &start)| {
        let end = vertices[(i + 1) % vertices.len()];
        point_segment_distance(center, start, end) < expanded
    })
}

fn point_in_polygon_strict(point: (f64, f64), vertices: &[(f64, f64)]) -> bool {
    let (px, py) = point;
    let mut inside = false;
    for i in 0..vertices.len() {
        let (x1, y1) = vertices[i];
        let (x2, y2) = vertices[(i + 1) % vertices.len()];
        if (y1 > py) != (y2 > py) && px < (x2 - x1) * (py - y1) / (y2 - y1) + x1 { inside = !inside; }
    }
    inside
}

fn point_segment_distance(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let (px, py) = point;
    let (sx, sy) = start;
    let (ex, ey) = end;
    let dx = ex - sx;
    let dy = ey - sy;
    let len2 = dx * dx + dy * dy;
    if len2 <= f64::EPSILON { return (px - sx).hypot(py - sy); }
    let t = (((px - sx) * dx + (py - sy) * dy) / len2).clamp(0.0, 1.0);
    (px - (sx + t * dx)).hypot(py - (sy + t * dy))
}

fn segment_crosses_transversely(a0: (f64, f64), a1: (f64, f64), b0: (f64, f64), b1: (f64, f64), eps: f64) -> bool {
    fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 { ax * by - ay * bx }
    let ax = a1.0 - a0.0; let ay = a1.1 - a0.1;
    let bx = b1.0 - b0.0; let by = b1.1 - b0.1;
    let c = cross(ax, ay, bx, by);
    if c.abs() <= eps { return false; }
    let c1 = cross(ax, ay, b0.0 - a0.0, b0.1 - a0.1);
    let c2 = cross(ax, ay, b1.0 - a0.0, b1.1 - a0.1);
    let c3 = cross(bx, by, a0.0 - b0.0, a0.1 - b0.1);
    let c4 = cross(bx, by, a1.0 - b0.0, a1.1 - b0.1);
    ((c1 > eps && c2 < -eps) || (c1 < -eps && c2 > eps)) && ((c3 > eps && c4 < -eps) || (c3 < -eps && c4 > eps))
}

fn collinear_overlap(a0: (f64, f64), a1: (f64, f64), b0: (f64, f64), b1: (f64, f64), eps: f64) -> bool {
    let cross_a = (a1.1 - a0.1) * (b0.0 - a0.0) - (a1.0 - a0.0) * (b0.1 - a0.1);
    let cross_b = (a1.1 - a0.1) * (b1.0 - a0.0) - (a1.0 - a0.0) * (b1.1 - a0.1);
    if cross_a.abs() > eps || cross_b.abs() > eps { return false; }
    let len = (a1.0 - a0.0).hypot(a1.1 - a0.1);
    if len <= eps { return false; }
    let ux = (a1.0 - a0.0) / len;
    let uy = (a1.1 - a0.1) / len;
    let t0 = (b0.0 - a0.0) * ux + (b0.1 - a0.1) * uy;
    let t1 = (b1.0 - a0.0) * ux + (b1.1 - a0.1) * uy;
    t1.max(t0).min(len) - t0.min(t1).max(0.0) > eps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::InternalBond;

    fn placement(x: f64, y: f64) -> Placement { Placement { x, y, rotation_radians: 0.0 } }

    #[test]
    fn graph_nodes_are_individual_constituents() {
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Hydrogen", placement(1.0, 0.0));
        assert_ne!(a, b);
        assert_eq!(graph.constituents().len(), 2);
    }

    #[test]
    fn materialization_creates_constituents_without_inventing_structure() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let mut graph = PhysicalConstituentGraph::new();
        let ids = graph.materialize_material(&material, placement(0.0, 0.0)).unwrap();
        assert_eq!(ids.len(), 2);
        assert!(graph.relationships().is_empty());
    }

    #[test]
    fn aggregate_material_cannot_become_one_constituent() {
        let material = Material::free_base("Carbon", 2.0);
        let mut graph = PhysicalConstituentGraph::new();
        assert!(graph.materialize_material(&material, placement(0.0, 0.0)).is_err());
    }

    #[test]
    fn constituent_identity_survives_unrelated_removal() {
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Hydrogen", placement(1.0, 0.0));
        let c = graph.add_constituent("Carbon", placement(2.0, 0.0));
        graph.remove_constituent(b);
        assert!(graph.constituent(a).is_some());
        assert!(graph.constituent(c).is_some());
        assert_eq!(graph.constituents().iter().map(|x| x.id).collect::<Vec<_>>(), vec![a, c]);
    }

    #[test]
    fn relationship_does_not_impose_endpoint_occupancy_limit() {
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Hydrogen", placement(1.0, 0.0));
        let c = graph.add_constituent("Hydrogen", placement(-1.0, 0.0));
        let endpoint = ConnectionEndpoint::Corner { point_index: 0 };
        for other in [b, c] {
            graph.add_relationship(PhysicalRelationship { constituent_a: a, constituent_b: other, endpoint_a: endpoint, endpoint_b: endpoint, strength: 1.0, bond_energy: 1.0 }).unwrap();
        }
        assert_eq!(graph.relationship_count_at(a, endpoint), 2);
    }

    #[test]
    fn connected_components_follow_physical_relationships() {
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Hydrogen", placement(1.0, 0.0));
        let c = graph.add_constituent("Water", placement(5.0, 0.0));
        graph.add_relationship(PhysicalRelationship { constituent_a: a, constituent_b: b, endpoint_a: ConnectionEndpoint::Corner { point_index: 0 }, endpoint_b: ConnectionEndpoint::Corner { point_index: 0 }, strength: 1.0, bond_energy: 1.0 }).unwrap();
        let components = graph.connected_components();
        assert_eq!(components.len(), 2);
        assert!(components.contains(&vec![a, b]));
        assert!(components.contains(&vec![c]));
    }

    #[test]
    fn attachment_discovers_actual_corner_pair_from_geometry() {
        let catalog = crate::resources::default_catalog();
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Carbon", placement(1.0, 0.0));
        let candidates = graph.attachment_candidates(a, b, &catalog);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|candidate| candidate.constituent_a == a && candidate.constituent_b == b));
    }

    #[test]
    fn attachment_admission_does_not_count_existing_bonds() {
        let catalog = crate::resources::default_catalog();
        let mut graph = PhysicalConstituentGraph::new();
        let radius = match &catalog.iter().find(|r| r.name == "Carbon").unwrap().shape.form { Form::RegularPolygon { radius, .. } => *radius, _ => unreachable!() };
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Carbon", placement(2.0 * radius, 0.0));
        let candidates = graph.attachment_candidates(a, b, &catalog);
        let candidate = candidates.into_iter().min_by(|x, y| x.distance.total_cmp(&y.distance)).unwrap();
        graph.try_attach(candidate, 1.0, 1.0, 1e-9, &catalog).unwrap();
        assert!(graph.try_attach(candidate, 1.0, 1.0, 1e-9, &catalog).is_ok());
        assert_eq!(graph.relationships().len(), 2);
    }

    #[test]
    fn circular_water_uses_the_same_circumference_physics_as_other_circles() {
        let catalog = crate::resources::default_catalog();
        let water = catalog.iter().find(|r| r.name == "Water").unwrap();
        assert!(matches!(water.shape.form, Form::Circle { .. }));
        assert!(matches!(water.shape.connection_sites(), ConnectionSites::Circumference { .. }));
    }
}
