//! One fail-closed world snapshot for directional environmental exposure.
//!
//! This assembles only physically realized geometry. Logical material stock has
//! no spatial shape and is not an occluder. Building this snapshot does not
//! settle energy or connect sunlight to the live tick.

use crate::material_storage::StoredMaterial;
use crate::organism_geometry::OrganismBodyGeometry;
use crate::physical_material::PhysicalMaterial;
use crate::resources::BaseResource;
use crate::state::{Organism, Simulation};
use crate::structure::Placement;
use crate::world_tone::{WorldToneCaptureFrame, WorldToneSample};

#[derive(Debug)]
pub(crate) struct WorldToneOccluderSnapshot {
    /// Frame-wide exposure cache; organism indices below map into this frame.
    pub(crate) frame: WorldToneCaptureFrame,
    pub(crate) organism_body_indices: Vec<Option<usize>>,
    /// Number of physical bodies represented in the frame, useful for auditing
    /// coverage without exposing the internal body array.
    pub(crate) occluder_count: usize,
}

impl WorldToneOccluderSnapshot {
    /// Assemble the same world-space occluder set for every receiver.
    ///
    /// Order is stable: organisms, environmental physical material, legacy
    /// decomposing structures, stored physical material, then physical material
    /// held by active transformations. Stored items are transformed from the
    /// owner's current anchor; field objects and environmental transition
    /// sources already carry world placements.
    pub(crate) fn from_simulation(
        sample: WorldToneSample,
        simulation: &Simulation,
    ) -> Option<Self> {
        let catalog = &simulation.environment.catalog;
        let mut bodies = Vec::new();
        let mut organism_body_indices = vec![None; simulation.organisms.len()];

        // Every organism remains a physical occluder regardless of whether its
        // genome is still intact. Structure placements are already world-space.
        for (organism_index, organism) in simulation.organisms.iter().enumerate() {
            if organism
                .structure
                .structural_unit_indices(catalog)
                .is_empty()
            {
                continue;
            }
            let body = OrganismBodyGeometry::from_structure(&organism.structure, catalog)?;
            organism_body_indices[organism_index] = Some(bodies.len());
            bodies.push(body);
        }

        // Physical field objects retain their world placements. Do not invent
        // geometry for aggregate logical stock.
        for cell in &simulation.environment.field.cells {
            for physical in &cell.physical_materials {
                bodies.push(OrganismBodyGeometry::from_physical_material(
                    physical, catalog,
                )?);
            }
        }

        // Legacy decomposing carriers are still physical structures if present;
        // this does not invoke their separate BREAK pathway.
        for decomposing in &simulation.decomposing_bodies {
            if decomposing
                .structure
                .structural_unit_indices(catalog)
                .is_empty()
            {
                continue;
            }
            bodies.push(OrganismBodyGeometry::from_structure(
                &decomposing.structure,
                catalog,
            )?);
        }

        // Stored objects are physically present at owner-relative positions.
        for organism in &simulation.organisms {
            if organism.stored_material.entries.is_empty() {
                continue;
            }
            let anchor = owner_anchor(organism)?;
            for entry in &organism.stored_material.entries {
                let StoredMaterial::Physical(physical) = entry;
                bodies.push(body_from_owner_relative_material(
                    physical, anchor, catalog,
                )?);
            }
        }

        // A BREAK source has left storage but remains physical while the
        // transformation is pending. Environmental sources held by a pending
        // transaction are likewise retained as occluders. Ambiguous dual-source
        // states fail closed rather than risk counting the same operation twice.
        for transformation in &simulation.active_transformations {
            if transformation.stored_material.is_some()
                && transformation.combine_environmental_source.is_some()
            {
                return None;
            }
            if transformation.stored_material.is_none()
                && transformation.combine_environmental_source.is_none()
            {
                continue;
            }
            let owner = simulation
                .organisms
                .iter()
                .find(|organism| organism.id == transformation.organism_id)?;
            if owner.active_transformation_id != Some(transformation.id) {
                return None;
            }
            if let Some(physical) = &transformation.stored_material {
                bodies.push(body_from_owner_relative_material(
                    physical,
                    owner_anchor(owner)?,
                    catalog,
                )?);
            }
            if let Some((cell_index, physical)) = &transformation.combine_environmental_source {
                if simulation
                    .environment
                    .field
                    .cells
                    .get(*cell_index)
                    .is_none()
                {
                    return None;
                }
                bodies.push(OrganismBodyGeometry::from_physical_material(
                    physical, catalog,
                )?);
            }
        }

        let occluder_count = bodies.len();
        let frame = WorldToneCaptureFrame::from_geometry(sample, &bodies)?;
        Some(Self {
            frame,
            organism_body_indices,
            occluder_count,
        })
    }

    pub(crate) fn body_index_for_organism(&self, organism_index: usize) -> Option<usize> {
        self.organism_body_indices
            .get(organism_index)
            .copied()
            .flatten()
    }

    pub(crate) fn projected_width_for_organism_unit(
        &self,
        organism_index: usize,
        unit_index: usize,
    ) -> Option<f64> {
        self.frame
            .projected_width_for_unit(self.body_index_for_organism(organism_index)?, unit_index)
    }
}

fn owner_anchor(organism: &Organism) -> Option<Placement> {
    let position = organism.occupied_cells.first()?;
    if !position.x.is_finite() || !position.y.is_finite() {
        return None;
    }
    // Storage currently records offsets from the first occupied-cell anchor;
    // organism orientation is not independently rotated by the movement model.
    Some(Placement {
        x: position.x,
        y: position.y,
        rotation_radians: 0.0,
    })
}

fn transform_relative(origin: Placement, relative: Placement) -> Placement {
    let (sin, cos) = origin.rotation_radians.sin_cos();
    Placement {
        x: origin.x + relative.x * cos - relative.y * sin,
        y: origin.y + relative.x * sin + relative.y * cos,
        rotation_radians: origin.rotation_radians + relative.rotation_radians,
    }
}

fn body_from_owner_relative_material(
    physical: &PhysicalMaterial,
    owner_anchor: Placement,
    catalog: &[BaseResource],
) -> Option<OrganismBodyGeometry> {
    let relative_origin = physical.owner_relative_origin?;
    let world_origin = transform_relative(owner_anchor, relative_origin);
    let placements = physical.placements.as_ref()?;
    if placements.is_empty() || placements.len() != physical.material.parts.len() {
        return None;
    }

    let mut world_material = physical.clone();
    world_material.placements = Some(
        placements
            .iter()
            .copied()
            .map(|relative| transform_relative(world_origin, relative))
            .collect(),
    );
    OrganismBodyGeometry::from_physical_material(&world_material, catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::Material;
    use crate::state::{Position, Simulation};
    use crate::structure::{OrganismStructure, StructuralUnit};

    fn sample() -> WorldToneSample {
        WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            signal_energy_level: 8.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.0,
        }
    }

    fn replace_with_nitrogen_square(
        organism: &mut Organism,
        x: f64,
        y: f64,
        catalog: &[BaseResource],
    ) {
        let mut unit = StructuralUnit::from_material(
            Material::free_base("Nitrogen", 1.0),
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        )
        .expect("nitrogen unit should be constructible");
        assert!(unit.realize_default_geometry(catalog));
        let mut structure = OrganismStructure::new();
        structure.add_unit(unit);
        organism.structure = structure;
        organism.occupied_cells = vec![Position { x, y }];
        organism.stored_material.entries.clear();
        organism.mark_structure_changed();
    }

    fn clear_environment_physical_material(simulation: &mut Simulation) {
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
    }

    #[test]
    fn snapshot_includes_environmental_physical_material_in_shadow_geometry() {
        let mut simulation = Simulation::new(71, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);

        let unobstructed = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("one valid organism should produce a snapshot");
        let initial_width = unobstructed
            .projected_width_for_organism_unit(0, 0)
            .expect("organism unit should map to its receiver body");
        assert!(initial_width > 0.0);

        // Hydrogen is a thin, realized rectangle: it casts a partial shadow
        // without approximating its shape or blocking the entire receiver.
        let blocker = PhysicalMaterial::realized(
            Material::free_base("Hydrogen", 1.0),
            vec![Placement {
                x: 2.5,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("hydrogen blocker should be physically realized");
        assert!(simulation
            .environment
            .field
            .deposit_physical(2.5, 0.0, blocker));

        let obstructed = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("physical field objects must join the world snapshot");
        let blocked_width = obstructed
            .projected_width_for_organism_unit(0, 0)
            .expect("organism unit mapping should remain stable");
        assert!(blocked_width > 0.0 && blocked_width < initial_width);
        assert_eq!(obstructed.occluder_count, 2);
    }

    #[test]
    fn stored_physical_material_tracks_its_owner_anchor_translation() {
        let catalog = crate::resources::default_catalog();
        let physical = PhysicalMaterial::realized(
            Material::free_base("Nitrogen", 1.0),
            vec![Placement {
                x: 5.0,
                y: 2.0,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            }],
            &catalog,
        )
        .expect("stored constituent should be physically realized");
        let mut storage = crate::material_storage::MaterialStorage::default();
        assert!(storage.store_physical_instance_at_owner_anchor(
            physical,
            Placement {
                x: 10.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let StoredMaterial::Physical(stored) = &storage.entries[0];

        // The owner moves from x=10 to x=20; the stored item's local offset
        // remains -5 and its original orientation remains ninety degrees.
        let body = body_from_owner_relative_material(
            stored,
            Placement {
                x: 20.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .expect("owner-relative material should transform back to world geometry");
        assert!((body.parts[0].x - 15.0).abs() < 1e-10);
        assert!((body.parts[0].y - 2.0).abs() < 1e-10);
        assert!((body.parts[0].rotation_radians - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn active_transformation_keeps_removed_stored_material_in_snapshot() {
        let mut simulation = Simulation::new(72, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);

        let physical = PhysicalMaterial::realized(
            Material::free_base("Hydrogen", 1.0),
            vec![Placement {
                x: 4.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("in-transition constituent should be physically realized");
        let mut storage = crate::material_storage::MaterialStorage::default();
        assert!(storage.store_physical_instance_at_owner_anchor(
            physical,
            Placement {
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let StoredMaterial::Physical(stored) = storage.entries.into_iter().next().unwrap();
        let owner_id = simulation.organisms[0].id.clone();
        simulation.organisms[0].active_transformation_id = Some(17);

        simulation
            .active_transformations
            .push(crate::state::ActiveTransformation {
                id: 17,
                organism_id: owner_id,
                kind: crate::state::TransformationKind::Break,
                material: Material::free_base("Hydrogen", 1.0),
                bond: None,
                stored_material: Some(stored),
                stored_bond: None,
                environmental_source: false,
                complexity: 1.0,
                duration_ticks: 1,
                remaining_ticks: 1,
                prepared_energy: None,
                pending_experience: None,
                decision_context_key: None,
                prepared_structure: None,
                prepared_stored_material: None,
                prepared_usable_energy: None,
                prepared_stress: None,
                prepared_ledger: None,
                combine_environmental_source: None,
        });

        let snapshot = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("removed material remains a physical occluder during BREAK");
        assert_eq!(snapshot.occluder_count, 2);
        assert_eq!(snapshot.body_index_for_organism(0), Some(0));
    }
}
