use super::*;

impl DevelopmentalFieldBlueprint {
    pub fn realization(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        developmental_origin: (f64, f64),
        developmental_orientation_radians: f64,
        preferred_mass: f64,
    ) -> DevelopmentalRealization {
        let (seed_mass, seed_length) = crate::juvenile::confirmed_seed_scale_reference(catalog)
            .expect("confirmed seed scale reference must be valid");
        let preferred_length =
            self.preferred_developmental_length(preferred_mass.max(1e-9), seed_mass, seed_length);
        self.realization_at_length(
            structure,
            catalog,
            developmental_origin,
            developmental_orientation_radians,
            preferred_length,
        )
    }

    pub(crate) fn realization_at_length(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        developmental_origin: (f64, f64),
        developmental_orientation_radians: f64,
        preferred_length: f64,
    ) -> DevelopmentalRealization {
        let material_realized = self.material_realization(
            structure,
            catalog,
            developmental_origin,
            developmental_orientation_radians,
            preferred_length,
        );
        let density_realized = self.density_realization(
            structure,
            catalog,
            developmental_origin,
            developmental_orientation_radians,
            preferred_length,
        );

        let connectivity = self.connectivity_realization(
            structure,
            catalog,
            developmental_origin,
            developmental_orientation_radians,
            preferred_length,
        );

        let mut sum = 0.0;
        let mut active = 0usize;
        for value in [material_realized, density_realized, connectivity]
            .into_iter()
            .flatten()
        {
            if value.is_finite() {
                sum += value;
                active += 1;
            }
        }

        DevelopmentalRealization {
            material: material_realized,
            density: density_realized,
            connectivity,
            overall: if active == 0 {
                0.0
            } else {
                (sum / active as f64).clamp(0.0, 1.0)
            },
        }
    }

    pub(crate) fn material_and_density_realization(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        origin: (f64, f64),
        orientation: f64,
        preferred_length: f64,
    ) -> (Option<f64>, Option<f64>) {
        (
            self.material_realization(
                structure,
                catalog,
                origin,
                orientation,
                preferred_length,
            ),
            self.density_realization(
                structure,
                catalog,
                origin,
                orientation,
                preferred_length,
            ),
        )
    }

    pub(crate) fn realization_after_break_with_components(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        origin: (f64, f64),
        orientation: f64,
        preferred_length: f64,
        bond_index: usize,
        material_realized: Option<f64>,
        density_realized: Option<f64>,
    ) -> Option<DevelopmentalRealization> {
        if bond_index >= structure.bonds.len() {
            return None;
        }
        let connectivity = self.connectivity_realization_excluding(
            structure,
            catalog,
            origin,
            orientation,
            preferred_length,
            Some(bond_index),
        );
        let mut sum = 0.0;
        let mut active = 0usize;
        for value in [material_realized, density_realized, connectivity]
            .into_iter()
            .flatten()
        {
            if value.is_finite() {
                sum += value;
                active += 1;
            }
        }
        Some(DevelopmentalRealization {
            material: material_realized,
            density: density_realized,
            connectivity,
            overall: if active == 0 {
                0.0
            } else {
                (sum / active as f64).clamp(0.0, 1.0)
            },
        })
    }

    pub(crate) fn realization_after_break(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        developmental_origin: (f64, f64),
        developmental_orientation_radians: f64,
        preferred_length: f64,
        bond_index: usize,
    ) -> Option<DevelopmentalRealization> {
        let (material_realized, density_realized) = self.material_and_density_realization(
            structure,
            catalog,
            developmental_origin,
            developmental_orientation_radians,
            preferred_length,
        );
        self.realization_after_break_with_components(
            structure,
            catalog,
            developmental_origin,
            developmental_orientation_radians,
            preferred_length,
            bond_index,
            material_realized,
            density_realized,
        )
    }

    fn material_realization(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        origin: (f64, f64),
        orientation: f64,
        preferred_length: f64,
    ) -> Option<f64> {
        let available = self.material_available_value(preferred_length);
        if available <= 0.0 {
            return None;
        }
        let mut realized = 0.0;
        for unit in &structure.units {
            let Some(shape) = unit.shape(catalog) else {
                continue;
            };
            let total_amount = unit
                .material
                .parts
                .iter()
                .map(|(_, amount)| *amount)
                .sum::<f64>();
            if !total_amount.is_finite() || total_amount <= 0.0 {
                continue;
            }
            realized += self.integrate_shape_field(
                &shape.form,
                unit.placement,
                origin,
                orientation,
                |x, y| {
                    unit.material
                        .parts
                        .iter()
                        .map(|(name, amount)| {
                            (amount / total_amount)
                                * self.material_preference_scaled(
                                    name,
                                    x,
                                    y,
                                    preferred_length,
                                )
                        })
                        .sum::<f64>()
                },
            );
        }
        Some((realized / available).clamp(0.0, 1.0))
    }

    fn density_realization(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        origin: (f64, f64),
        orientation: f64,
        preferred_length: f64,
    ) -> Option<f64> {
        let available = self.density_available_value(preferred_length);
        if available <= 0.0 {
            return None;
        }
        let mut realized = 0.0;
        for unit in &structure.units {
            let Some(shape) = unit.shape(catalog) else {
                continue;
            };
            realized += self.integrate_shape_field(
                &shape.form,
                unit.placement,
                origin,
                orientation,
                |x, y| self.density_preference_scaled(x, y, preferred_length),
            );
        }
        Some((realized / available).clamp(0.0, 1.0))
    }

    fn material_available_value(&self, preferred_length: f64) -> f64 {
        self.material_preferences
            .iter()
            .map(|field| {
                let primary = field.primary_influence().integral(preferred_length);
                let additional = field
                    .additional_influences
                    .iter()
                    .map(|influence| influence.integral(preferred_length))
                    .sum::<f64>();
                let normalization = field.center_preference.max(0.0)
                    + field
                        .additional_influences
                        .iter()
                        .map(|i| i.strength)
                        .sum::<f64>();
                if normalization <= 0.0 {
                    0.0
                } else {
                    (primary + additional) / normalization
                }
            })
            .sum()
    }

    fn density_available_value(&self, preferred_length: f64) -> f64 {
        let primary = RadialInfluence {
            center_x: self.structural_density.center_x,
            center_y: self.structural_density.center_y,
            radial_falloff: self.structural_density.radial_falloff,
            strength: self.structural_density.center_preference.max(0.0),
        };
        let normalization = primary.strength
            + self
                .structural_density
                .additional_influences
                .iter()
                .map(|i| i.strength)
                .sum::<f64>();
        if normalization <= 0.0 {
            0.0
        } else {
            (primary.integral(preferred_length)
                + self
                    .structural_density
                    .additional_influences
                    .iter()
                    .map(|i| i.integral(preferred_length))
                    .sum::<f64>())
                / normalization
        }
    }

    fn connectivity_realization(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        origin: (f64, f64),
        orientation: f64,
        preferred_length: f64,
    ) -> Option<f64> {
        self.connectivity_realization_excluding(
            structure,
            catalog,
            origin,
            orientation,
            preferred_length,
            None,
        )
    }

    fn connectivity_realization_excluding(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        origin: (f64, f64),
        orientation: f64,
        preferred_length: f64,
        excluded_bond: Option<usize>,
    ) -> Option<f64> {
        let mut actual_value = 0.0;
        let mut available_value = 0.0;
        for (bond_index, bond) in structure.bonds.iter().enumerate() {
            if excluded_bond == Some(bond_index) {
                continue;
            }
            let Some(a) = structure.unit_index(bond.endpoint_a.constituent_id) else {
                continue;
            };
            let Some(b) = structure.unit_index(bond.endpoint_b.constituent_id) else {
                continue;
            };
            let Some(wa) = bond
                .endpoint_a
                .location
                .world_point(&structure.units[a], catalog)
            else {
                continue;
            };
            let Some(wb) = bond
                .endpoint_b
                .location
                .world_point(&structure.units[b], catalog)
            else {
                continue;
            };
            let la = developmental_point(wa.x, wa.y, origin, orientation);
            let lb = developmental_point(wb.x, wb.y, origin, orientation);
            actual_value += self.connectivity_score(
                la,
                lb,
                structure,
                a,
                b,
                bond.endpoint_a.location,
                bond.endpoint_b.location,
                catalog,
                preferred_length,
            );
        }

        for a in 0..structure.units.len() {
            for b in (a + 1)..structure.units.len() {
                for candidate in
                    crate::contact::connection_pair_candidates(structure, a, b, catalog)
                        .into_iter()
                        .filter(|candidate| candidate.available_a && candidate.available_b)
                {
                    // The opportunity denominator is E_G ∪ O_new. Exclude only
                    // an opportunity that is the exact already-realized edge;
                    // other available endpoint pairs between the same two
                    // physical units remain legitimate opportunities.
                    let is_existing_edge = structure.bonds.iter().any(|bond| {
                        let Some(id_a) = structure.physical_id(a) else {
                            return false;
                        };
                        let Some(id_b) = structure.physical_id(b) else {
                            return false;
                        };
                        (bond.endpoint_a.constituent_id == id_a
                            && bond.endpoint_a.location == candidate.endpoint_a
                            && bond.endpoint_b.constituent_id == id_b
                            && bond.endpoint_b.location == candidate.endpoint_b)
                            || (bond.endpoint_a.constituent_id == id_b
                                && bond.endpoint_a.location == candidate.endpoint_b
                                && bond.endpoint_b.constituent_id == id_a
                                && bond.endpoint_b.location == candidate.endpoint_a)
                    });
                    if is_existing_edge {
                        continue;
                    }
                    let Some(wa) = candidate
                        .endpoint_a
                        .world_point(&structure.units[a], catalog)
                    else {
                        continue;
                    };
                    let Some(wb) = candidate
                        .endpoint_b
                        .world_point(&structure.units[b], catalog)
                    else {
                        continue;
                    };
                    let la = developmental_point(wa.x, wa.y, origin, orientation);
                    let lb = developmental_point(wb.x, wb.y, origin, orientation);
                    available_value += self.connectivity_score(
                        la,
                        lb,
                        structure,
                        a,
                        b,
                        candidate.endpoint_a,
                        candidate.endpoint_b,
                        catalog,
                        preferred_length,
                    );
                }
            }
        }

        available_value += actual_value;
        if available_value <= 0.0 {
            None
        } else {
            Some((actual_value / available_value).clamp(0.0, 1.0))
        }
    }

    fn connectivity_score(
        &self,
        a: (f64, f64),
        b: (f64, f64),
        structure: &crate::structure::OrganismStructure,
        unit_a: usize,
        unit_b: usize,
        endpoint_a: crate::structure::ConnectionEndpoint,
        endpoint_b: crate::structure::ConnectionEndpoint,
        catalog: &[BaseResource],
        preferred_length: f64,
    ) -> f64 {
        let ka = self.connectivity_preference_scaled(a.0, a.1, preferred_length);
        let kb = self.connectivity_preference_scaled(b.0, b.1, preferred_length);
        let qa = endpoint_opportunity_count(structure, unit_a, endpoint_a, catalog);
        let qb = endpoint_opportunity_count(structure, unit_b, endpoint_b, catalog);
        let qreal_a = endpoint_realized_count(structure, unit_a, endpoint_a);
        let qreal_b = endpoint_realized_count(structure, unit_b, endpoint_b);
        let n = 0.5 * (qreal_a as f64 / qa.max(1) as f64 + qreal_b as f64 / qb.max(1) as f64);
        let lambda = crate::developmental_blueprint::CANDIDATE_CONNECTIVITY_WEIGHT;
        ((ka + kb) * 0.5 + lambda * n).max(0.0)
    }

    fn integrate_shape_field<F>(
        &self,
        form: &crate::resources::Form,
        placement: crate::structure::Placement,
        origin: (f64, f64),
        orientation: f64,
        field: F,
    ) -> f64
    where
        F: Fn(f64, f64) -> f64,
    {
        const NODES: [f64; 8] = [
            -0.9602898564975363,
            -0.7966664774136267,
            -0.525532409916329,
            -0.1834346424956498,
            0.1834346424956498,
            0.525532409916329,
            0.7966664774136267,
            0.9602898564975363,
        ];
        const WEIGHTS: [f64; 8] = [
            0.1012285362903763,
            0.2223810344533745,
            0.3137066458778873,
            0.362683783378362,
            0.362683783378362,
            0.3137066458778873,
            0.2223810344533745,
            0.1012285362903763,
        ];
        let radius = form.bounding_radius().max(0.0);
        if !radius.is_finite() || radius <= 0.0 {
            return 0.0;
        }
        let min_x = placement.x - radius;
        let max_x = placement.x + radius;
        let min_y = placement.y - radius;
        let max_y = placement.y + radius;
        let hx = (max_x - min_x) * 0.5;
        let hy = (max_y - min_y) * 0.5;
        let cx = (max_x + min_x) * 0.5;
        let cy = (max_y + min_y) * 0.5;
        let mut total = 0.0;
        for i in 0..8 {
            for j in 0..8 {
                let world_x = cx + hx * NODES[i];
                let world_y = cy + hy * NODES[j];
                if !point_in_form(form, placement, world_x, world_y) {
                    continue;
                }
                let (x, y) = developmental_point(world_x, world_y, origin, orientation);
                total += WEIGHTS[i] * WEIGHTS[j] * field(x, y);
            }
        }
        total * hx * hy
    }
}

fn gaussian_plane_integral(amplitude: f64, radial_falloff: f64) -> Option<f64> {
    if !amplitude.is_finite()
        || amplitude <= 0.0
        || !radial_falloff.is_finite()
        || radial_falloff <= 0.0
    {
        None
    } else {
        Some(amplitude * std::f64::consts::PI / radial_falloff)
    }
}

pub(crate) fn developmental_point(
    x: f64,
    y: f64,
    origin: (f64, f64),
    orientation: f64,
) -> (f64, f64) {
    let dx = x - origin.0;
    let dy = y - origin.1;
    let (s, c) = orientation.sin_cos();
    (dx * c + dy * s, -dx * s + dy * c)
}

fn point_in_form(
    form: &crate::resources::Form,
    placement: crate::structure::Placement,
    x: f64,
    y: f64,
) -> bool {
    match form {
        crate::resources::Form::Circle { radius } => {
            (x - placement.x).hypot(y - placement.y) <= *radius
        }
        crate::resources::Form::Rectangle { .. }
        | crate::resources::Form::RegularPolygon { .. }
        | crate::resources::Form::Polygon { .. } => {
            let Some(local_vertices) = form.polygon_vertices() else {
                return false;
            };
            let (s, c) = placement.rotation_radians.sin_cos();
            let local_x = (x - placement.x) * c + (y - placement.y) * s;
            let local_y = -(x - placement.x) * s + (y - placement.y) * c;
            let mut inside = false;
            for i in 0..local_vertices.len() {
                let a = local_vertices[i];
                let b = local_vertices[(i + 1) % local_vertices.len()];
                if (a.1 > local_y) != (b.1 > local_y)
                    && local_x < (b.0 - a.0) * (local_y - a.1) / (b.1 - a.1) + a.0
                {
                    inside = !inside;
                }
            }
            inside
        }
        crate::resources::Form::Line { .. } | crate::resources::Form::Fluid { .. } => false,
    }
}

fn endpoint_realized_count(
    structure: &crate::structure::OrganismStructure,
    unit: usize,
    endpoint: crate::structure::ConnectionEndpoint,
) -> usize {
    let Some(id) = structure.physical_id(unit) else {
        return 0;
    };
    structure
        .bonds
        .iter()
        .filter(|bond| bond.touches(id, endpoint))
        .count()
}

fn endpoint_opportunity_count(
    structure: &crate::structure::OrganismStructure,
    unit: usize,
    endpoint: crate::structure::ConnectionEndpoint,
    catalog: &[BaseResource],
) -> usize {
    let mut count = endpoint_realized_count(structure, unit, endpoint);
    for other in 0..structure.units.len() {
        if other == unit {
            continue;
        }
        if crate::contact::connection_pair_candidates(structure, unit, other, catalog)
            .into_iter()
            .any(|candidate| {
                candidate.available_a
                    && candidate.available_b
                    && candidate.endpoint_a.same_location(endpoint)
            })
        {
            count += 1;
        }
    }
    count.max(1)
}

pub fn default_developmental_blueprint() -> DevelopmentalFieldBlueprint {
    DevelopmentalFieldBlueprint {
        material_preferences: vec![
            ("Carbon", 1.0),
            ("Nitrogen", 0.0),
            ("Phosphorus", 0.0),
            ("Sulfur", 0.0),
            ("Hydrogen", 0.0),
            ("Methane", 0.0),
            ("Water", 0.0),
        ]
        .into_iter()
        .map(
            |(resource_name, center_preference)| MaterialPreferenceField {
                resource_name: resource_name.into(),
                center_preference,
                radial_falloff: 2.0, // EXPERIMENTAL: alpha=0.5 initial Gaussian width.
                center_x: 0.0,       // EXPERIMENTAL: initial influence center.
                center_y: 0.0,       // EXPERIMENTAL: initial influence center.
                additional_influences: vec![
                    RadialInfluence {
                        center_x: 0.0,
                        center_y: 0.0,
                        radial_falloff: 2.0,
                        strength: center_preference,
                    };
                    3
                ],
            },
        )
        .collect(),
        structural_density: StructuralDensityField {
            center_preference: 0.5,
            radial_falloff: 2.0, // EXPERIMENTAL: alpha=0.5 initial Gaussian width.
            center_x: 0.0,       // EXPERIMENTAL: initial influence center.
            center_y: 0.0,       // EXPERIMENTAL: initial influence center.
            additional_influences: vec![
                RadialInfluence {
                    center_x: 0.0,
                    center_y: 0.0,
                    radial_falloff: 2.0,
                    strength: 0.5,
                };
                3
            ],
        },
        connectivity: ConnectivityField {
            strength: 0.0,
            center_x: 0.0,       // EXPERIMENTAL: initial influence center.
            center_y: 0.0,       // EXPERIMENTAL: initial influence center.
            radial_falloff: 2.0, // EXPERIMENTAL: alpha=0.5 initial Gaussian width.
            additional_influences: vec![
                RadialInfluence {
                    center_x: 0.0,
                    center_y: 0.0,
                    radial_falloff: 2.0,
                    strength: 0.0,
                };
                3
            ],
        },
    }
}
