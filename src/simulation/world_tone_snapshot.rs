//! One fail-closed world snapshot for directional environmental exposure.
//!
//! This assembles only physically realized geometry. Logical material stock has
//! no spatial shape and is not an occluder. The live tick uses the resulting frame
//! to settle direct sunlight into each living organism's existing energy holder.

use crate::energy_ledger::EnergyLedgerAuthority;
use crate::material_storage::StoredMaterial;
use crate::organism_geometry::OrganismBodyGeometry;
use crate::physical_material::PhysicalMaterial;
use crate::resources::{BaseResource, ResourceProperties};
use crate::state::{Organism, Simulation};
use crate::structure::Placement;
use crate::world_tone::{WorldToneCaptureFrame, WorldToneSample};

#[derive(Debug)]
pub(crate) struct WorldToneOccluderSnapshot {
    /// Frame-wide exposure cache; organism indices below map into this frame.
    pub(crate) frame: WorldToneCaptureFrame,
    pub(crate) organism_body_indices: Vec<Option<usize>>,
    /// Material properties keyed by the constituent indices used by geometry.
    pub(crate) body_properties: Vec<Vec<(usize, ResourceProperties)>>,
    /// Number of bodies retained by the conservative receiver-angle broad phase.
    /// Bodies outside every receiver's angular span cannot cast a direct-source shadow.
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
        let receiver_organism_indices: Vec<_> = (0..simulation.organisms.len()).collect();
        Self::from_simulation_for_organisms(sample, simulation, &receiver_organism_indices)
    }

    /// Validate a selected biological receiver subset while building the
    /// world-wide physical exposure frame. All realized bodies are receivers
    /// for energy accounting, regardless of whether a living organism lies
    /// behind them; the selected indices are retained for caller validation.
    pub(crate) fn from_simulation_for_organisms(
        sample: WorldToneSample,
        simulation: &Simulation,
        receiver_organism_indices: &[usize],
    ) -> Option<Self> {
        if receiver_organism_indices
            .iter()
            .any(|index| *index >= simulation.organisms.len())
        {
            return None;
        }
        let catalog = &simulation.environment.catalog;
        let mut bodies = Vec::new();
        let mut body_properties = Vec::new();
        let mut organism_body_indices = vec![None; simulation.organisms.len()];

        // Every organism remains a physical occluder regardless of whether its
        // genome is still intact. Structure placements are already world-space.
        for (organism_index, organism) in simulation.organisms.iter().enumerate() {
            if organism.structure.units.is_empty() {
                continue;
            }
            let body = OrganismBodyGeometry::from_all_realized_units(&organism.structure)?;
            organism_body_indices[organism_index] = Some(bodies.len());
            body_properties.push(structure_material_properties(&organism.structure, catalog)?);
            bodies.push(body);
        }

        let phase = std::f64::consts::TAU * sample.orbital_phase;
        let toward_source = (phase.sin(), -phase.cos());

        // A developing offspring is a separate physical structure even
        // before it is admitted to simulation.organisms. Its committed storage
        // remains physical too and is anchored to the offspring's own origin.
        for organism in &simulation.organisms {
            let Some(construction) = &organism.reproductive_construction else {
                continue;
            };
            if !construction.developing_structure.units.is_empty() {
                body_properties.push(structure_material_properties(
                    &construction.developing_structure,
                    catalog,
                )?);
                bodies.push(OrganismBodyGeometry::from_all_realized_units(
                    &construction.developing_structure,
                )?);
            }
            if !construction.committed_material.entries.is_empty() {
                let anchor = Placement {
                    x: construction.developmental_origin.x,
                    y: construction.developmental_origin.y,
                    rotation_radians: 0.0,
                };
                for entry in &construction.committed_material.entries {
                    let StoredMaterial::Physical(physical) = entry;
                    body_properties.push(physical_material_properties(physical, catalog)?);
                    bodies.push(body_from_owner_relative_material(
                        physical, anchor, catalog,
                    )?);
                }
            }
        }

        // Physical field objects retain their world placements. Do not invent
        // geometry for aggregate logical stock. Every realized field object is
        // also a possible receiver: it must be settled as heat even when no
        // living organism lies behind it, so receiver-relative angular culling
        // cannot discard it here.
        for cell in &simulation.environment.field.cells {
            for physical in &cell.physical_materials {
                let _interval =
                    physical_material_parallel_lane_interval(physical, catalog, toward_source)?;
                body_properties.push(physical_material_properties(physical, catalog)?);
                bodies.push(OrganismBodyGeometry::from_physical_material(
                    physical, catalog,
                )?);
            }
        }

        // Legacy decomposing carriers are still physical structures if present;
        // this does not invoke their separate BREAK pathway.
        for decomposing in &simulation.decomposing_bodies {
            if decomposing.structure.units.is_empty() {
                continue;
            }
            body_properties.push(structure_material_properties(
                &decomposing.structure,
                catalog,
            )?);
            bodies.push(OrganismBodyGeometry::from_all_realized_units(
                &decomposing.structure,
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
                body_properties.push(physical_material_properties(physical, catalog)?);
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
                body_properties.push(physical_material_properties(physical, catalog)?);
                bodies.push(body_from_owner_relative_material(
                    physical,
                    owner_anchor(owner)?,
                    catalog,
                )?);
            }
            if let Some((cell_index, physical)) = &transformation.combine_environmental_source {
                let _ = simulation.environment.field.cells.get(*cell_index)?;
                body_properties.push(physical_material_properties(physical, catalog)?);
                bodies.push(OrganismBodyGeometry::from_physical_material(
                    physical, catalog,
                )?);
            }
        }

        // A body can only block a receiver along directions subtended by that
        // receiver. Keep a conservative angular broad phase so unrelated bodies
        // do not enter every exact ray test.
        let angular_intervals = bodies
            .iter()
            .map(|body| parallel_lane_interval(body, toward_source))
            .collect::<Option<Vec<_>>>()?;
        // Every realized body is a receiver as well as a possible occluder.
        // Keeping the full set is necessary for world energy accounting: a lane
        // that never reaches a living organism still deposits response energy
        // as heat in physical material instead of silently omitting that input.
        let mut remapped_indices = vec![None; bodies.len()];
        let mut relevant_bodies = Vec::with_capacity(bodies.len());
        let mut relevant_properties = Vec::with_capacity(body_properties.len());
        for (original_index, body) in bodies.into_iter().enumerate() {
            let _interval = angular_intervals[original_index];
            remapped_indices[original_index] = Some(relevant_bodies.len());
            relevant_bodies.push(body);
            relevant_properties.push(body_properties[original_index].clone());
        }
        let organism_body_indices: Vec<_> = organism_body_indices
            .into_iter()
            .map(|index| index.and_then(|original| remapped_indices[original]))
            .collect();
        let receiver_body_indices: Vec<_> = (0..relevant_bodies.len()).collect();
        if receiver_body_indices.is_empty() {
            return None;
        }
        let occluder_count = relevant_bodies.len();
        let frame = WorldToneCaptureFrame::from_geometry_for_receivers(
            sample,
            &relevant_bodies,
            &receiver_body_indices,
        )?;
        Some(Self {
            frame,
            organism_body_indices,
            body_properties: relevant_properties,
            occluder_count,
        })
    }

    pub(crate) fn properties_for_body_unit(
        &self,
        body_index: usize,
        unit_index: usize,
    ) -> Option<ResourceProperties> {
        self.body_properties
            .get(body_index)?
            .iter()
            .find_map(|(candidate, properties)| (*candidate == unit_index).then_some(*properties))
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

    /// Settle one tick of direct environmental input into each living organism's
    /// existing usable-energy holder. Geometry is sampled once for the whole
    /// world, while material response uses the actual resource of each unit.
    /// Invalid world geometry fails closed: no organism receives partial credit
    /// from an incomplete occluder set.
    pub(crate) fn settle_living_organism_inputs(
        simulation: &mut Simulation,
        sample: WorldToneSample,
        living_ids: &std::collections::HashSet<String>,
    ) {
        let receiver_organism_indices: Vec<_> = simulation
            .organisms
            .iter()
            .enumerate()
            .filter_map(|(index, organism)| living_ids.contains(&organism.id).then_some(index))
            .collect();
        let Some(snapshot) =
            Self::from_simulation_for_organisms(sample, simulation, &receiver_organism_indices)
        else {
            return;
        };

        // Every lane's first material hit must reproduce the direct geometric
        // exposure widths. Abort the frame rather than settle from inconsistent
        // path and exposure views of the same world.
        let mut path_direct_widths = std::collections::HashMap::<(usize, usize), f64>::new();
        for path in snapshot.frame.interaction_paths() {
            if let Some(&(body_index, unit_index)) = path.hits.first() {
                let width = path_direct_widths
                    .entry((body_index, unit_index))
                    .or_insert(0.0);
                *width += path.projected_width;
                if !width.is_finite() {
                    return;
                }
            }
        }
        for (organism_index, organism) in simulation.organisms.iter().enumerate() {
            if !living_ids.contains(&organism.id) {
                continue;
            }
            let Some(body_index) = snapshot.body_index_for_organism(organism_index) else {
                continue;
            };
            for unit_index in 0..organism.structure.units.len() {
                let Some(expected) = snapshot
                    .frame
                    .projected_width_for_unit(body_index, unit_index)
                else {
                    continue;
                };
                let actual = path_direct_widths
                    .get(&(body_index, unit_index))
                    .copied()
                    .unwrap_or(0.0);
                if !expected.is_finite()
                    || !actual.is_finite()
                    || (expected - actual).abs() > 1e-7 * expected.max(1.0)
                {
                    return;
                }
            }
        }

        let mut body_to_organism = vec![None; snapshot.body_properties.len()];
        for (organism_index, body_index) in snapshot.organism_body_indices.iter().enumerate() {
            if let Some(body_index) = body_index {
                if let Some(slot) = body_to_organism.get_mut(*body_index) {
                    *slot = Some(organism_index);
                } else {
                    return;
                }
            }
        }

        // Work against staged reservoirs. Any invalid geometry, spectrum, or
        // ledger allocation aborts this entire source frame without partial credit.
        let mut staged_reservoirs: Vec<_> = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.clone())
            .collect();
        let catalog = &simulation.environment.catalog;
        let baselines = crate::resources::ResourceBaselines::from_catalog(catalog);
        let mut total_incident = 0.0;
        let mut total_captured = 0.0;
        let mut total_heat = 0.0;
        let mut total_escaped = 0.0;

        for path in snapshot.frame.interaction_paths() {
            if !path.projected_width.is_finite()
                || path.projected_width <= 0.0
                || path.hits.is_empty()
            {
                return;
            }
            let Some(incident_energy) =
                sample.incident_energy_for_projected_width(path.projected_width)
            else {
                return;
            };
            let mut signal = crate::harmonics::ToneSpectrum {
                components: vec![crate::harmonics::ToneComponent {
                    frequency_hz: sample.frequency_hz,
                    amplitude: incident_energy.sqrt(),
                    phase_radians: 0.0,
                }],
            };
            let mut path_captured = 0.0;
            let mut path_heat = 0.0;

            for &(body_index, unit_index) in &path.hits {
                let Some(properties) = snapshot.properties_for_body_unit(body_index, unit_index)
                else {
                    return;
                };
                let incoming_energy = spectrum_energy(&signal);
                let Some(incoming_energy) = incoming_energy else {
                    return;
                };
                if incoming_energy <= f64::EPSILON {
                    signal = crate::harmonics::ToneSpectrum::empty();
                    break;
                }

                let Some(response_spectrum) =
                    crate::harmonics::material_response_spectrum(properties, baselines, &signal)
                else {
                    return;
                };
                let response_energy = spectrum_energy(&response_spectrum);
                let Some(response_energy) = response_energy else {
                    return;
                };
                if response_energy > incoming_energy + 1e-9 {
                    return;
                }
                let damping = crate::harmonics::damping_ratio(properties, baselines);
                if !damping.is_finite() || damping < 0.0 {
                    return;
                }
                let damping_heat = response_energy * damping.clamp(0.0, 1.0);
                let living_organism =
                    body_to_organism
                        .get(body_index)
                        .copied()
                        .flatten()
                        .filter(|organism_index| {
                            living_ids.contains(&simulation.organisms[*organism_index].id)
                        });
                // A living receiver may retain the non-damped response in its
                // resonance reservoir. A physical receiver without a living
                // reservoir instead converts the entire material response to
                // heat; only the unresponded remainder continues downstream.
                let Some(mut interaction_heat) = heat_for_material_response(
                    response_energy,
                    damping_heat,
                    living_organism.is_some(),
                ) else {
                    return;
                };
                let mut captured = 0.0;

                if let Some(organism_index) = living_organism {
                    let retained_budget = response_energy - damping_heat;
                    let Some(retained_spectrum) =
                        crate::harmonics::transform_signal_to_energy_budget(
                            properties,
                            baselines,
                            &signal,
                            retained_budget,
                        )
                    else {
                        return;
                    };
                    let mut candidate = staged_reservoirs[organism_index].clone();
                    let before_energy = candidate.total_energy();
                    let mut deposit_ok = before_energy.is_some();
                    for component in &retained_spectrum.components {
                        let energy = component.amplitude * component.amplitude;
                        if energy > f64::EPSILON
                            && !candidate.deposit(component.frequency_hz, energy)
                        {
                            deposit_ok = false;
                            break;
                        }
                    }
                    let stored_delta = candidate
                        .total_energy()
                        .zip(before_energy)
                        .map(|(after, before)| after - before);
                    if deposit_ok
                        && stored_delta.is_some_and(|stored| {
                            stored.is_finite()
                                && (stored - retained_budget).abs()
                                    <= 1e-9 * retained_budget.max(1.0)
                        })
                    {
                        staged_reservoirs[organism_index] = candidate;
                        captured = retained_budget;
                    } else {
                        // A bounded reservoir must not silently discard a
                        // spectral component; rejected response energy is heat.
                        interaction_heat = response_energy;
                    }
                }

                // Response energy is removed from the travelling signal
                // exactly once, whether it is retained biologically or becomes
                // heat at a nonliving/dead physical receiver.
                let outgoing_budget = incoming_energy - response_energy;
                let Some(outgoing) = crate::harmonics::transform_signal_to_energy_budget(
                    properties,
                    baselines,
                    &signal,
                    outgoing_budget.max(0.0),
                ) else {
                    return;
                };
                path_captured += captured;
                path_heat += interaction_heat;
                if !path_captured.is_finite() || !path_heat.is_finite() {
                    return;
                }
                signal = outgoing;
            }

            let escaped = spectrum_energy(&signal);
            let Some(escaped) = escaped else {
                return;
            };
            let allocated = path_captured + path_heat + escaped;
            if !allocated.is_finite()
                || (incident_energy - allocated).abs() > 1e-8 * incident_energy.max(1.0)
            {
                return;
            }
            total_incident += incident_energy;
            total_captured += path_captured;
            total_heat += path_heat;
            total_escaped += escaped;
            if ![total_incident, total_captured, total_heat, total_escaped]
                .iter()
                .all(|value| value.is_finite())
            {
                return;
            }
        }

        if !simulation.energy_ledger.settle_world_tone_paths(
            total_incident,
            total_captured,
            total_heat,
            total_escaped,
        ) {
            return;
        }
        for (organism, reservoir) in simulation.organisms.iter_mut().zip(staged_reservoirs) {
            organism.resonant_energy = reservoir;
        }
    }
}

fn heat_for_material_response(
    response_energy: f64,
    damping_heat: f64,
    can_retain_resonance: bool,
) -> Option<f64> {
    if !response_energy.is_finite()
        || response_energy < 0.0
        || !damping_heat.is_finite()
        || damping_heat < 0.0
        || damping_heat > response_energy + 1e-9 * response_energy.max(1.0)
    {
        return None;
    }
    Some(if can_retain_resonance {
        damping_heat.min(response_energy)
    } else {
        response_energy
    })
}

fn spectrum_energy(spectrum: &crate::harmonics::ToneSpectrum) -> Option<f64> {
    let mut energy = 0.0;
    for component in &spectrum.components {
        if !component.frequency_hz.is_finite()
            || component.frequency_hz <= 0.0
            || !component.amplitude.is_finite()
            || component.amplitude < 0.0
            || !component.phase_radians.is_finite()
        {
            return None;
        }
        energy += component.amplitude * component.amplitude;
        if !energy.is_finite() {
            return None;
        }
    }
    Some(energy)
}

fn structure_material_properties(
    structure: &crate::structure::OrganismStructure,
    catalog: &[BaseResource],
) -> Option<Vec<(usize, ResourceProperties)>> {
    structure
        .units
        .iter()
        .enumerate()
        .map(|(unit_index, unit)| Some((unit_index, unit.properties(catalog)?)))
        .collect()
}

fn physical_material_properties(
    physical: &PhysicalMaterial,
    catalog: &[BaseResource],
) -> Option<Vec<(usize, ResourceProperties)>> {
    physical
        .material
        .parts
        .iter()
        .enumerate()
        .map(|(unit_index, (name, _))| {
            let resource = catalog.iter().find(|resource| resource.name == *name)?;
            Some((unit_index, resource.properties))
        })
        .collect()
}

fn parallel_lane_interval(
    body: &OrganismBodyGeometry,
    toward_source: (f64, f64),
) -> Option<(f64, f64)> {
    parallel_lane_interval_for_bounds(
        body.min_x,
        body.max_x,
        body.min_y,
        body.max_y,
        toward_source,
    )
}

fn physical_material_parallel_lane_interval(
    physical: &PhysicalMaterial,
    catalog: &[BaseResource],
    toward_source: (f64, f64),
) -> Option<(f64, f64)> {
    let placements = physical.placements.as_ref()?;
    if placements.is_empty() || placements.len() != physical.material.parts.len() {
        return None;
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for (index, placement) in placements.iter().enumerate() {
        if !placement.x.is_finite()
            || !placement.y.is_finite()
            || !placement.rotation_radians.is_finite()
        {
            return None;
        }
        let shape = physical.realized_shape_for_part(index, catalog)?;
        let radius = shape.form.bounding_radius();
        if !radius.is_finite() || radius < 0.0 {
            return None;
        }
        min_x = min_x.min(placement.x - radius);
        max_x = max_x.max(placement.x + radius);
        min_y = min_y.min(placement.y - radius);
        max_y = max_y.max(placement.y + radius);
    }
    parallel_lane_interval_for_bounds(min_x, max_x, min_y, max_y, toward_source)
}

fn parallel_lane_interval_for_bounds(
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    toward_source: (f64, f64),
) -> Option<(f64, f64)> {
    if ![min_x, max_x, min_y, max_y, toward_source.0, toward_source.1]
        .iter()
        .all(|value| value.is_finite())
        || min_x > max_x
        || min_y > max_y
        || (toward_source.0.hypot(toward_source.1) - 1.0).abs() > 1e-9
    {
        return None;
    }
    let perpendicular = (-toward_source.1, toward_source.0);
    let corners = [
        (min_x, min_y),
        (min_x, max_y),
        (max_x, min_y),
        (max_x, max_y),
    ];
    let mut minimum = f64::INFINITY;
    let mut maximum = f64::NEG_INFINITY;
    for (x, y) in corners {
        let lane = x * perpendicular.0 + y * perpendicular.1;
        if !lane.is_finite() {
            return None;
        }
        minimum = minimum.min(lane);
        maximum = maximum.max(lane);
    }
    Some((minimum, maximum))
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
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
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
    fn snapshot_keeps_bodies_outside_living_receiver_spans_for_world_accounting() {
        let mut simulation = Simulation::new(76, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);

        let unobstructed = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("one valid receiver should produce a snapshot");
        let initial_width = unobstructed
            .projected_width_for_organism_unit(0, 0)
            .expect("receiver unit should have cached exposure");

        let unrelated = PhysicalMaterial::realized(
            Material::free_base("Hydrogen", 1.0),
            vec![Placement {
                x: 5.0,
                y: 100.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("unrelated physical material should be realized");
        assert!(simulation
            .environment
            .field
            .deposit_physical(5.0, 100.0, unrelated));

        let filtered = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("irrelevant but valid bodies must not invalidate the frame");
        let filtered_width = filtered
            .projected_width_for_organism_unit(0, 0)
            .expect("receiver mapping should survive broad-phase filtering");
        assert_eq!(filtered.occluder_count, 2);
        assert!((filtered_width - initial_width).abs() < 1e-10);
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
    fn snapshot_fails_closed_on_unexpanded_composite_structural_units() {
        use crate::resources::InternalBond;

        let mut simulation = Simulation::new(75, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);
        simulation.organisms[0].structure.units[0].material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        simulation.organisms[0].mark_structure_changed();

        assert!(
            WorldToneOccluderSnapshot::from_simulation(sample(), &simulation).is_none(),
            "a cached single-resource shape must not stand in for a bonded composite material"
        );

        // Multiple constituents are still composite geometry when they have no
        // internal bonds; a single cached shape cannot represent their surfaces.
        simulation.organisms[0].structure.units[0]
            .material
            .internal_bonds
            .clear();
        simulation.organisms[0].mark_structure_changed();
        assert!(
            WorldToneOccluderSnapshot::from_simulation(sample(), &simulation).is_none(),
            "an unbonded multi-constituent material must also fail closed"
        );
    }

    #[test]
    fn developing_offspring_and_its_stored_material_remain_world_occluders() {
        let mut simulation = Simulation::new(73, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);

        let mut developing_structure = OrganismStructure::new();
        let mut anchor_unit = StructuralUnit::from_material(
            Material::free_base("Nitrogen", 1.0),
            Placement {
                x: 20.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        )
        .expect("developing anchor should be constructible");
        assert!(anchor_unit.realize_default_geometry(&catalog));
        developing_structure.add_unit(anchor_unit);

        let stored_physical = PhysicalMaterial::realized(
            Material::free_base("Hydrogen", 1.0),
            vec![Placement {
                x: 21.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("developing stored material should be physically realized");
        let mut committed_material = crate::material_storage::MaterialStorage::default();
        assert!(committed_material.store_physical_instance_at_owner_anchor(
            stored_physical,
            Placement {
                x: 20.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        simulation.organisms[0].reproductive_construction =
            Some(crate::state::ReproductiveConstruction {
                committed_material,
                developing_structure,
                child_genome: simulation.organisms[0].genome.clone(),
                developmental_origin: Position { x: 20.0, y: 0.0 },
                developmental_orientation_radians: 0.0,
                developing_stress: 0.0,
                anchor_unit_index: 0,
                developing_energy: 0.0,
                developing_maintenance_debt: 0.0,
                needs_space: false,
            });

        let snapshot = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("developing physical material must be represented");
        assert_eq!(snapshot.occluder_count, 3);
        assert_eq!(snapshot.body_index_for_organism(0), Some(0));
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
    #[test]
    fn exposed_environmental_material_settles_heat_without_a_living_receiver() {
        let mut simulation = Simulation::new(79, 1.0);
        clear_environment_physical_material(&mut simulation);
        for organism in &mut simulation.organisms {
            organism.structure = OrganismStructure::new();
            organism.stored_material.entries.clear();
            organism.reproductive_construction = None;
        }
        simulation.decomposing_bodies.clear();
        simulation.active_transformations.clear();

        let catalog = simulation.environment.catalog.clone();
        let exposed = PhysicalMaterial::realized(
            Material::free_base("Nitrogen", 1.0),
            vec![Placement {
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("environmental receiver should have realized geometry");
        assert!(simulation
            .environment
            .field
            .deposit_physical(5.0, 0.0, exposed));

        WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample(),
            &std::collections::HashSet::new(),
        );

        assert!(
            simulation.energy_ledger.total_external_energy_input > 0.0,
            "physical material must receive the source even when no living organism is present"
        );
        assert!(
            simulation.energy_ledger.total_heat_dissipated > 0.0,
            "nonliving material response must settle as heat"
        );
        assert_eq!(
            simulation.energy_ledger.total_resonant_energy_held, 0.0,
            "a nonliving receiver must not receive biological resonance storage"
        );
    }

    #[test]
    fn nonliving_material_response_becomes_heat_instead_of_rejoining_signal() {
        let response_energy = 3.25;
        let damping_heat = 0.75;

        assert_eq!(
            heat_for_material_response(response_energy, damping_heat, false),
            Some(response_energy),
            "nonliving physical material has no resonant store, so all absorbed response is heat"
        );
        assert_eq!(
            heat_for_material_response(response_energy, damping_heat, true),
            Some(damping_heat),
            "living receivers retain the non-damped response when reservoir settlement succeeds"
        );
        assert_eq!(
            heat_for_material_response(response_energy, response_energy + 1.0, false),
            None,
            "an impossible heat allocation must fail closed"
        );
    }
}
