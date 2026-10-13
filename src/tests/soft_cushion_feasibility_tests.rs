//! Diagnostic-only whole-blueprint feasibility experiment.
//! This is intentionally separate from the live constructor: it explores whether
//! the unchanged seed shapes/topology admit a non-overlapping layout when each
//! required connection may use a flexible boundary gap of up to 0.1 units.
#[cfg(test)]
mod tests {
    use crate::material_geometry::{placed_forms_penetrate, PlacedMaterialPart, GEOMETRY_EPSILON};
    use crate::resources::{default_catalog, Form};
    use crate::structure::Placement;
    use crate::structural_blueprint::StructuralBlueprint;

    const MAX_BOND_GAP: f64 = 0.1;
    const ITERATIONS: usize = 30_000;

    fn world_vertices(form: &Form, pose: Placement) -> Option<Vec<(f64, f64)>> {
        Some(form.polygon_vertices()?.into_iter().map(|(x, y)| {
            let (s, c) = pose.rotation_radians.sin_cos();
            (pose.x + x * c - y * s, pose.y + x * s + y * c)
        }).collect())
    }

    fn cross(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
        (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
    }

    fn point_segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let length_sq = dx * dx + dy * dy;
        if length_sq <= f64::EPSILON {
            return (p.0 - a.0).hypot(p.1 - a.1);
        }
        let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_sq).clamp(0.0, 1.0);
        (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
    }

    fn segments_intersect(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
        let eps = 1e-10;
        let ab_c = cross(a, b, c);
        let ab_d = cross(a, b, d);
        let cd_a = cross(c, d, a);
        let cd_b = cross(c, d, b);
        if ((ab_c > eps && ab_d < -eps) || (ab_c < -eps && ab_d > eps))
            && ((cd_a > eps && cd_b < -eps) || (cd_a < -eps && cd_b > eps))
        {
            return true;
        }
        (ab_c.abs() <= eps && point_segment_distance(c, a, b) <= eps)
            || (ab_d.abs() <= eps && point_segment_distance(d, a, b) <= eps)
            || (cd_a.abs() <= eps && point_segment_distance(a, c, d) <= eps)
            || (cd_b.abs() <= eps && point_segment_distance(b, c, d) <= eps)
    }

    fn segments_distance(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> f64 {
        if segments_intersect(a, b, c, d) {
            return 0.0;
        }
        point_segment_distance(a, c, d)
            .min(point_segment_distance(b, c, d))
            .min(point_segment_distance(c, a, b))
            .min(point_segment_distance(d, a, b))
    }

    fn point_in_polygon(point: (f64, f64), vertices: &[(f64, f64)]) -> bool {
        let mut inside = false;
        for i in 0..vertices.len() {
            let a = vertices[i];
            let b = vertices[(i + 1) % vertices.len()];
            if (a.1 > point.1) != (b.1 > point.1)
                && point.0 < (b.0 - a.0) * (point.1 - a.1) / (b.1 - a.1) + a.0
            {
                inside = !inside;
            }
        }
        inside
    }

    fn boundary_gap(a: &Form, pa: Placement, b: &Form, pb: Placement) -> f64 {
        if let (Form::Circle { radius: ar }, Form::Circle { radius: br }) = (a, b) {
            return ((pa.x - pb.x).hypot(pa.y - pb.y) - ar - br).max(0.0);
        }
        let (Some(av), Some(bv)) = (world_vertices(a, pa), world_vertices(b, pb)) else {
            return ((pa.x - pb.x).hypot(pa.y - pb.y) - a.bounding_radius() - b.bounding_radius()).max(0.0);
        };
        if av.iter().any(|&p| point_in_polygon(p, &bv))
            || bv.iter().any(|&p| point_in_polygon(p, &av))
        {
            return 0.0;
        }
        let mut gap = f64::INFINITY;
        for i in 0..av.len() {
            for j in 0..bv.len() {
                gap = gap.min(segments_distance(
                    av[i], av[(i + 1) % av.len()], bv[j], bv[(j + 1) % bv.len()],
                ));
            }
        }
        gap
    }

    fn score(
        forms: &[Form],
        poses: &[Placement],
        connections: &[(usize, usize)],
    ) -> (f64, usize, f64) {
        let mut value = 0.0;
        let mut overlaps = 0;
        let mut max_bond_gap: f64 = 0.0;
        for i in 0..forms.len() {
            let a = PlacedMaterialPart { part_index: i, form: forms[i].clone(), placement: poses[i] };
            for j in (i + 1)..forms.len() {
                let b = PlacedMaterialPart { part_index: j, form: forms[j].clone(), placement: poses[j] };
                if placed_forms_penetrate(&a, &b, GEOMETRY_EPSILON) {
                    overlaps += 1;
                    let center = (poses[i].x - poses[j].x).hypot(poses[i].y - poses[j].y);
                    let depth_proxy = (forms[i].bounding_radius() + forms[j].bounding_radius() - center).max(0.0);
                    value += 1_000.0 + depth_proxy * 20.0;
                }
            }
        }
        for &(i, j) in connections {
            let gap = boundary_gap(&forms[i], poses[i], &forms[j], poses[j]);
            max_bond_gap = max_bond_gap.max(gap);
            if gap > MAX_BOND_GAP {
                value += 500.0 * (gap - MAX_BOND_GAP).powi(2);
            } else {
                // A soft preference for a small, non-mandatory cushion.
                value += 0.5 * (gap - 0.025).powi(2);
            }
        }
        (value, overlaps, max_bond_gap)
    }

    #[test]
    fn soft_cushion_global_layout_feasibility_probe() {
        let catalog = default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let forms: Vec<Form> = blueprint.elements.iter().map(|element| {
            let (name, _) = element.material.parts.first().expect("seed element has a material part");
            catalog.iter().find(|r| r.name == *name).expect("seed resource exists").shape.form.clone()
        }).collect();
        let connections: Vec<(usize, usize)> = blueprint.connections.iter()
            .map(|c| (c.element_a, c.element_b)).collect();
        let mut poses: Vec<Placement> = blueprint.elements.iter().map(|e| Placement {
            x: e.placement.x,
            y: e.placement.y,
            rotation_radians: e.placement.rotation_radians,
        }).collect();

        let (mut current_score, _, _) = score(&forms, &poses, &connections);
        let mut best_poses = poses.clone();
        let mut best_score = current_score;
        let mut best_overlaps = current_overlaps;
        let mut best_gap = current_gap;
        // Fixed-seed LCG keeps the experiment repeatable across CI runs.
        let mut rng: u64 = 0x5eed_c0ff_ee12_3456;
        let mut random = || {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((rng >> 11) as f64) / ((1u64 << 53) as f64)
        };

        for iteration in 0..ITERATIONS {
            let fraction = 1.0 - iteration as f64 / ITERATIONS as f64;
            let temperature = 30.0 * fraction + 0.15;
            let step = 0.35 * fraction + 0.015;
            // Keep the anchor fixed to remove arbitrary global translation.
            let index = 1 + (random() * (poses.len() - 1) as f64) as usize;
            let old = poses[index];
            let axis = random();
            if axis < 0.4 {
                poses[index].x += (random() * 2.0 - 1.0) * step;
            } else if axis < 0.8 {
                poses[index].y += (random() * 2.0 - 1.0) * step;
            } else {
                poses[index].rotation_radians += (random() * 2.0 - 1.0) * step * 0.8;
            }
            let (candidate_score, candidate_overlaps, candidate_gap) = score(&forms, &poses, &connections);
            let accept = candidate_score <= current_score
                || random() < ((current_score - candidate_score) / temperature).exp();
            if accept {
                current_score = candidate_score;
                if candidate_score < best_score {
                    best_score = candidate_score;
                    best_overlaps = candidate_overlaps;
                    best_gap = candidate_gap;
                    best_poses.clone_from(&poses);
                }
            } else {
                poses[index] = old;
            }
            if best_overlaps == 0 && best_gap <= MAX_BOND_GAP {
                break;
            }
        }

        let complete = best_overlaps == 0 && best_gap <= MAX_BOND_GAP;
        let moved = best_poses.iter().zip(blueprint.elements.iter()).filter(|(p, e)| {
            (p.x - e.placement.x).hypot(p.y - e.placement.y) > 1e-3
                || (p.rotation_radians - e.placement.rotation_radians).abs() > 1e-3
        }).count();
        eprintln!(
            "SOFT_CUSHION_GLOBAL_PROBE: elements={} connections={} iterations={} complete={} best_score={:.6} overlaps={} max_required_gap={:.6} moved_elements={}",
            forms.len(), connections.len(), ITERATIONS, complete, best_score, best_overlaps, best_gap, moved
        );
        assert!(best_score.is_finite(), "diagnostic solver must return a finite score");
    }
}
