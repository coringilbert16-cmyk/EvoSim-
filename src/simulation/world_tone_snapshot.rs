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
    /// Attached primary/developing structure belongs to the owner's resonance.
    /// Stored, environmental, and legacy nonliving material has no owner.
    body_resonance_owners: Vec<Option<usize>>,
    /// Bodies backed directly by the environmental material field, which may
    /// retain resonance without a living-organism owner.
    environment_body_indices: std::collections::HashSet<usize>,
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
        let mut body_resonance_owners = Vec::<Option<usize>>::new();
        let mut environment_body_indices = std::collections::HashSet::new();

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
            body_resonance_owners.push(Some(organism_index));
        }

        // Developing members are already part of the parent's one physical
        // aggregate. Only committed physical storage adds separate occluders.
        for organism in &simulation.organisms {
            let Some(construction) = &organism.reproductive_construction else {
                continue;
            };
            // A construction record is valid only as a stable-ID view into
            // the parent's authoritative aggregate. A detached legacy graph
            // would otherwise be rendered as a second physical body/resonance
            // owner, so reject the snapshot rather than double-count it.
            if !construction.is_aggregate_owned_by(&organism.structure) {
                return None;
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
                    body_resonance_owners.push(None);
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
                body_properties.push(physical_material_properties(physical, catalog)?);
                environment_body_indices.insert(bodies.len());
                bodies.push(OrganismBodyGeometry::from_physical_material(
                    physical, catalog,
                )?);
                body_resonance_owners.push(None);
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
            body_resonance_owners.push(None);
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
                body_resonance_owners.push(None);
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
                body_resonance_owners.push(None);
            }
            if let Some((cell_index, physical)) = &transformation.combine_environmental_source {
                let _ = simulation.environment.field.cells.get(*cell_index)?;
                body_properties.push(physical_material_properties(physical, catalog)?);
                bodies.push(OrganismBodyGeometry::from_physical_material(
                    physical, catalog,
                )?);
                body_resonance_owners.push(None);
            }
        }

        // A body can only block a receiver along directions subtended by that
        // receiver. Keep a conservative angular broad phase so unrelated bodies
        // do not enter every exact ray test.
        // Every realized body is a receiver as well as a possible occluder.
        // Keeping the full set is necessary for world energy accounting: a lane
        // that never reaches a living organism still deposits response energy
        // as heat in physical material instead of silently omitting that input.
        let mut remapped_indices = vec![None; bodies.len()];
        let mut relevant_bodies = Vec::with_capacity(bodies.len());
        let mut relevant_properties = Vec::with_capacity(body_properties.len());
        for (original_index, body) in bodies.into_iter().enumerate() {
            remapped_indices[original_index] = Some(relevant_bodies.len());
            relevant_bodies.push(body);
            relevant_properties.push(body_properties[original_index].clone());
        }
        let organism_body_indices: Vec<_> = organism_body_indices
            .into_iter()
            .map(|index| index.and_then(|original| remapped_indices[original]))
            .collect();
        if body_resonance_owners.len() != relevant_bodies.len() {
            return None;
        }
        let receiver_body_indices: Vec<_> = (0..relevant_bodies.len()).collect();
        if receiver_body_indices.is_empty() {
            return None;
        }
        let occluder_count = relevant_bodies.len();
        let frame = WorldToneCaptureFrame::from_geometry_for_receivers_at_tick(
            sample,
            &relevant_bodies,
            &receiver_body_indices,
            simulation.tick,
        )?;
        Some(Self {
            frame,
            organism_body_indices,
            body_resonance_owners,
            environment_body_indices,
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
        let body_to_organism = &snapshot.body_resonance_owners;
        if body_to_organism.len() != snapshot.body_properties.len() {
            return;
        }
        for (body_index, owner) in body_to_organism.iter().enumerate() {
            let Some(organism_index) = *owner else {
                continue;
            };
            let Some(organism) = simulation.organisms.get(organism_index) else {
                return;
            };
            if !living_ids.contains(&organism.id) {
                continue;
            }
            for &(unit_index, _) in &snapshot.body_properties[body_index] {
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

        // Work against staged reservoirs. Any invalid geometry, spectrum, or
        // ledger allocation aborts this entire source frame without partial credit.
        let mut staged_reservoirs: Vec<_> = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.clone())
            .collect();
        let mut staged_environment_resonance = simulation.environment.resonant_energy.clone();
        let initial_resonant_energy = staged_reservoirs
            .iter()
            .map(|reservoir| reservoir.total_energy().unwrap_or(f64::NAN))
            .sum::<f64>()
            + staged_environment_resonance
                .total_energy()
                .unwrap_or(f64::NAN);
        if !initial_resonant_energy.is_finite() || initial_resonant_energy < 0.0 {
            return;
        }
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
                    frequency_cycles_per_day: sample.frequency_cycles_per_day,
                    amplitude: incident_energy.sqrt(),
                    phase_radians: crate::harmonics::phase_at_tick(
                        sample.frequency_cycles_per_day,
                        simulation.tick,
                    ),
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
                // The material response is energy-limited. Clamp only the tiny
                // numerical overshoot already accepted above; never let it
                // become a source of additional captured or dissipated energy.
                let response_energy = response_energy.min(incoming_energy);
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
                // Living structures and environmental field material retain
                // the non-damped response as resonance. Other nonliving material
                // dissipates its response as heat; only the unresponded remainder
                // continues downstream.
                let environmental_receiver =
                    snapshot.environment_body_indices.contains(&body_index);
                let can_retain_resonance = living_organism.is_some() || environmental_receiver;
                let Some(mut interaction_heat) =
                    heat_for_material_response(response_energy, damping_heat, can_retain_resonance)
                else {
                    return;
                };
                let mut captured = 0.0;

                if can_retain_resonance {
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
                    let mut candidate = if let Some(organism_index) = living_organism {
                        staged_reservoirs[organism_index].clone()
                    } else {
                        staged_environment_resonance.clone()
                    };
                    let before_energy = candidate.total_energy();
                    let mut deposit_ok = before_energy.is_some();
                    for component in &retained_spectrum.components {
                        let energy = component.amplitude * component.amplitude;
                        if energy > f64::EPSILON
                            && !candidate.deposit(component.frequency_cycles_per_day, energy)
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
                            stored.is_finite() && stored >= 0.0 && stored <= response_energy
                        })
                    {
                        if let Some(organism_index) = living_organism {
                            staged_reservoirs[organism_index] = candidate;
                        } else {
                            staged_environment_resonance = candidate;
                        }
                        // Count only energy actually retained by the reservoir.
                        // Any rejected spectrum or rounding difference becomes heat.
                        captured = stored_delta.unwrap_or(0.0);
                        interaction_heat = response_energy - captured;
                    } else {
                        interaction_heat = response_energy;
                    }
                }

                // Response energy is removed from the travelling signal
                // exactly once, whether it is retained biologically or becomes
                // heat at a nonliving/dead physical receiver.
                let outgoing_budget = incoming_energy - response_energy;
                let outgoing = match crate::harmonics::transform_signal_to_energy_budget(
                    properties,
                    baselines,
                    &signal,
                    outgoing_budget.max(0.0),
                ) {
                    Some(outgoing) => outgoing,
                    // A numerically tiny remainder can be too small to form a
                    // representable transformed spectrum. Keep the frame
                    // transactional: account that bounded remainder as escaped
                    // energy below instead of discarding all staged captures.
                    None if outgoing_budget <= 1e-8 * incident_energy.max(1.0) => {
                        crate::harmonics::ToneSpectrum::empty()
                    }
                    None => return,
                };
                path_captured += captured;
                path_heat += interaction_heat;
                if !path_captured.is_finite() || !path_heat.is_finite() {
                    return;
                }
                signal = outgoing;
            }

            let escaped_spectrum_energy = spectrum_energy(&signal);
            let Some(escaped_spectrum_energy) = escaped_spectrum_energy else {
                return;
            };
            let mut escaped_budget = incident_energy - path_captured - path_heat;
            if escaped_budget < 0.0 {
                // Full absorption can overshoot the source by one or two ulps
                // when captured and heat allocations are summed independently.
                // Correct only a bounded negative rounding remainder by taking
                // it back from heat; never credit excess capture or invent an
                // outgoing signal to make the transaction balance.
                let overrun = -escaped_budget;
                let tolerance = 1e-8 * incident_energy.max(1.0);
                if overrun <= tolerance && path_heat >= overrun {
                    path_heat -= overrun;
                    escaped_budget = 0.0;
                } else {
                    return;
                }
            }
            if !escaped_budget.is_finite()
                || (escaped_budget - escaped_spectrum_energy).abs()
                    > 1e-8 * incident_energy.max(1.0)
            {
                return;
            }
            // Treat the tiny floating-point discrepancy between the transformed
            // spectrum and the accounted remainder as escaped energy. This
            // keeps allocation conservative without crediting more than arrived.
            let escaped = escaped_budget;
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

        // Per-path reservoir deltas are computed incrementally. Their sum
        // can differ by a few ulps from the actual final reservoir inventory
        // after many deposits into the same spectrum. The final staged holder
        // delta is authoritative: reconcile capture against it and debit the
        // same rounding difference from heat instead of letting the ledger
        // disagree with the energy that is physically stored.
        let staged_resonant_energy = staged_reservoirs
            .iter()
            .map(|reservoir| reservoir.total_energy().unwrap_or(f64::NAN))
            .sum::<f64>()
            + staged_environment_resonance
                .total_energy()
                .unwrap_or(f64::NAN);
        let authoritative_capture = staged_resonant_energy - initial_resonant_energy;
        let capture_correction = authoritative_capture - total_captured;
        let capture_tolerance = 1e-8 * total_incident.max(1.0);
        if !staged_resonant_energy.is_finite()
            || staged_resonant_energy < 0.0
            || !authoritative_capture.is_finite()
            || authoritative_capture < 0.0
            || !capture_correction.is_finite()
            || capture_correction.abs() > capture_tolerance
        {
            return;
        }
        if capture_correction > 0.0 {
            if total_heat < capture_correction {
                return;
            }
            total_heat -= capture_correction;
        } else {
            total_heat += -capture_correction;
        }
        total_captured = authoritative_capture;

        // Reconcile the aggregate totals from one shared residual after
        // summing paths. Summing independently rounded per-path remainders can
        // otherwise make the world total exceed its source by a few ulps.
        let mut aggregate_escaped = total_incident - (total_captured + total_heat);
        if aggregate_escaped < 0.0 {
            // Summing path totals can introduce another ulp-scale overrun even
            // after each path has been reconciled. Debit only that bounded
            // rounding amount from heat; never let aggregate allocation exceed
            // the source or convert a negative residual into captured energy.
            let overrun = -aggregate_escaped;
            let tolerance = 1e-8 * total_incident.max(1.0);
            if overrun <= tolerance && total_heat >= overrun {
                total_heat -= overrun;
                aggregate_escaped = 0.0;
            } else {
                return;
            }
        }
        if !aggregate_escaped.is_finite()
            || (aggregate_escaped - total_escaped).abs() > 1e-8 * total_incident.max(1.0)
        {
            return;
        }
        total_escaped = aggregate_escaped;
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
        simulation.environment.resonant_energy = staged_environment_resonance;
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
        if !component.frequency_cycles_per_day.is_finite()
            || component.frequency_cycles_per_day <= 0.0
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

/// Reconstruct every currently realized physical world body for the
/// independent pair-potential inventory oracle. This intentionally mirrors
/// the authoritative body classes assembled by WorldToneOccluderSnapshot.
fn world_physical_materials_for_pair_potential(
    simulation: &Simulation,
) -> Option<(Vec<PhysicalMaterial>, Vec<usize>, Vec<Option<usize>>)> {
    let catalog = &simulation.environment.catalog;
    let mut bodies = Vec::new();
    let mut resonant_participant_indices = Vec::new();
    // Attached physical bodies are translated with their owner. Their partial
    // gradients must be summed into the owning organism's net force.
    let mut force_owner_indices = Vec::new();

    for (organism_index, organism) in simulation.organisms.iter().enumerate() {
        if !organism.structure.units.is_empty() {
            resonant_participant_indices.push(bodies.len());
            bodies.push(physical_material_from_structure(
                &organism.structure,
                catalog,
            )?);
            force_owner_indices.push(Some(organism_index));
        }
    }

    for (organism_index, organism) in simulation.organisms.iter().enumerate() {
        let Some(construction) = &organism.reproductive_construction else {
            continue;
        };
        if !construction.is_aggregate_owned_by(&organism.structure) {
            return None;
        }
        if !construction.committed_material.entries.is_empty() {
            let anchor = Placement {
                x: construction.developmental_origin.x,
                y: construction.developmental_origin.y,
                rotation_radians: 0.0,
            };
            for entry in &construction.committed_material.entries {
                let StoredMaterial::Physical(physical) = entry;
                resonant_participant_indices.push(bodies.len());
                bodies.push(owner_relative_physical_material(physical, anchor)?);
                force_owner_indices.push(Some(organism_index));
            }
        }
    }

    for cell in &simulation.environment.field.cells {
        for physical in &cell.physical_materials {
            resonant_participant_indices.push(bodies.len());
            bodies.push(physical.clone());
            force_owner_indices.push(None);
        }
    }

    for decomposing in &simulation.decomposing_bodies {
        if !decomposing.structure.units.is_empty() {
            resonant_participant_indices.push(bodies.len());
            bodies.push(physical_material_from_structure(
                &decomposing.structure,
                catalog,
            )?);
            force_owner_indices.push(None);
        }
    }

    for (organism_index, organism) in simulation.organisms.iter().enumerate() {
        if organism.stored_material.entries.is_empty() {
            continue;
        }
        let anchor = owner_anchor(organism)?;
        for entry in &organism.stored_material.entries {
            let StoredMaterial::Physical(physical) = entry;
            // Stored material remains a physical blocker but is deliberately
            // excluded as a resonance source or receiver until it is released.
            bodies.push(owner_relative_physical_material(physical, anchor)?);
            force_owner_indices.push(Some(organism_index));
        }
    }

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
        let (owner_index, owner) = simulation
            .organisms
            .iter()
            .enumerate()
            .find(|(_, organism)| organism.id == transformation.organism_id)?;
        if owner.active_transformation_id != Some(transformation.id) {
            return None;
        }
        if let Some(physical) = &transformation.stored_material {
            bodies.push(owner_relative_physical_material(
                physical,
                owner_anchor(owner)?,
            )?);
            force_owner_indices.push(Some(owner_index));
        }
        if let Some((cell_index, physical)) = &transformation.combine_environmental_source {
            let _ = simulation.environment.field.cells.get(*cell_index)?;
            resonant_participant_indices.push(bodies.len());
            bodies.push(physical.clone());
            force_owner_indices.push(None);
        }
    }

    if bodies.len() != force_owner_indices.len() {
        return None;
    }
    Some((bodies, resonant_participant_indices, force_owner_indices))
}

fn physical_material_from_structure(
    structure: &crate::structure::OrganismStructure,
    catalog: &[BaseResource],
) -> Option<PhysicalMaterial> {
    if structure.units.is_empty() {
        return None;
    }
    let mut parts = Vec::with_capacity(structure.units.len());
    let mut placements = Vec::with_capacity(structure.units.len());
    let mut realized_shapes = Vec::with_capacity(structure.units.len());
    for unit in &structure.units {
        if unit.material.parts.len() != 1 || !unit.material.is_valid_for_catalog(catalog) {
            return None;
        }
        let (name, amount) = unit.material.parts.first()?;
        parts.push((name.clone(), *amount));
        placements.push(unit.placement);
        realized_shapes.push(unit.shape(catalog)?.clone());
    }
    let material = crate::resources::Material {
        parts,
        internal_bonds: Vec::new(),
    };
    if !material.is_valid_for_catalog(catalog) {
        return None;
    }
    Some(PhysicalMaterial {
        material,
        placements: Some(placements),
        internal_connections: None,
        owner_relative_origin: None,
        realized_shapes: Some(realized_shapes),
    })
}

fn owner_relative_physical_material(
    physical: &PhysicalMaterial,
    owner_anchor: Placement,
) -> Option<PhysicalMaterial> {
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
    world_material.owner_relative_origin = None;
    Some(world_material)
}

fn world_pair_potential_components_for_bodies(
    bodies: &[PhysicalMaterial],
    resonant_indices: &[usize],
    catalog: &[BaseResource],
    tick: u64,
) -> Option<Vec<crate::harmonics::ResonancePairPotentialComponent>> {
    let mut total = Vec::<crate::harmonics::ResonancePairPotentialComponent>::new();
    for (first_position, &first) in resonant_indices.iter().enumerate() {
        for &second in resonant_indices.iter().skip(first_position + 1) {
            let pair = crate::harmonics::resonance_pair_potential_components_with_occluders(
                &bodies[first],
                &bodies[second],
                bodies,
                catalog,
                tick,
            )?;
            for component in pair {
                if let Some(existing) = total.iter_mut().find(|existing| {
                    (existing.frequency_cycles_per_day - component.frequency_cycles_per_day).abs()
                        <= 1e-9
                }) {
                    existing.energy += component.energy;
                    if !existing.energy.is_finite() {
                        return None;
                    }
                } else {
                    total.push(component);
                }
            }
        }
    }
    total.sort_by(|a, b| {
        a.frequency_cycles_per_day
            .partial_cmp(&b.frequency_cycles_per_day)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(total)
}

/// Recompute the signed world pair-potential inventory from the physical
/// world, including every other body as an occluder for each pair. This is an
/// intentionally independent oracle; it is not yet a tick-loop path.
#[allow(dead_code)] // Staged for inventory verification before movement/chemistry use.
pub(crate) fn world_pair_potential_inventory_components(
    simulation: &Simulation,
) -> Option<Vec<crate::harmonics::ResonancePairPotentialComponent>> {
    let (bodies, resonant_indices, _) = world_physical_materials_for_pair_potential(simulation)?;
    world_pair_potential_components_for_bodies(
        &bodies,
        &resonant_indices,
        &simulation.environment.catalog,
        simulation.tick,
    )
}

/// Translate one already world-resolved physical body without changing its
/// material identity or shape. Stored/attached bodies are included because
/// their positions can alter the visibility of other resonant pairs.
fn translate_pair_potential_body(
    body: &PhysicalMaterial,
    dx: f64,
    dy: f64,
) -> Option<PhysicalMaterial> {
    if !dx.is_finite() || !dy.is_finite() || body.owner_relative_origin.is_some() {
        return None;
    }
    let mut translated = body.clone();
    for placement in translated.placements.as_mut()? {
        placement.x += dx;
        placement.y += dy;
        if !placement.x.is_finite() || !placement.y.is_finite() {
            return None;
        }
    }
    Some(translated)
}

/// Complete-world force oracle. Each component is the negative translational
/// gradient of the *same* unordered-pair potential inventory. Forces are
/// returned for every physical body, not just resonant participants, so a
/// non-emitting blocker receives its reaction contribution when moving it
/// changes the visibility of another pair.
///
/// This deliberately uses centered finite differences and rebuilds the world
/// potential for each perturbation. It is a correctness oracle for small
/// worlds, not a production tick-loop implementation; caching and a measured
/// broad phase are required before applying it to the 128-body experiment.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorldPairPotentialGradient {
    pub(crate) components: Vec<crate::harmonics::ResonancePairPotentialComponent>,
    /// One (Fx, Fy) per physical body in the oracle's stable body ordering.
    pub(crate) body_forces: Vec<(f64, f64)>,
    /// Owner organism index for attached/escrowed bodies; sum every partial
    /// force with the same owner index before moving that organism. None means
    /// the body moves independently. This mapping is stable with body_forces.
    pub(crate) body_force_owners: Vec<Option<usize>>,
}

#[allow(dead_code)] // Correctness oracle; production use is gated on caching/benchmarking.
pub(crate) fn world_pair_potential_gradient(
    simulation: &Simulation,
) -> Option<WorldPairPotentialGradient> {
    let (bodies, resonant_indices, force_owners) =
        world_physical_materials_for_pair_potential(simulation)?;
    world_pair_potential_gradient_for_bodies(
        &bodies,
        &resonant_indices,
        &force_owners,
        &simulation.environment.catalog,
        simulation.tick,
    )
}

fn world_pair_potential_gradient_for_bodies(
    bodies: &[PhysicalMaterial],
    resonant_indices: &[usize],
    force_owners: &[Option<usize>],
    catalog: &[BaseResource],
    tick: u64,
) -> Option<WorldPairPotentialGradient> {
    if force_owners.len() != bodies.len() {
        return None;
    }
    const DIFFERENCE_STEP: f64 = 1.0e-4;
    let components =
        world_pair_potential_components_for_bodies(bodies, resonant_indices, catalog, tick)?;
    let base_potential = components
        .iter()
        .map(|component| component.energy)
        .sum::<f64>();
    if !base_potential.is_finite() {
        return None;
    }

    let mut body_forces = vec![(0.0, 0.0); bodies.len()];
    for body_index in 0..bodies.len() {
        let mut translated = bodies.to_vec();
        translated[body_index] =
            translate_pair_potential_body(&bodies[body_index], DIFFERENCE_STEP, 0.0)?;
        let plus_x = world_pair_potential_components_for_bodies(
            &translated,
            resonant_indices,
            catalog,
            tick,
        )?
        .iter()
        .map(|component| component.energy)
        .sum::<f64>();

        translated[body_index] =
            translate_pair_potential_body(&bodies[body_index], -DIFFERENCE_STEP, 0.0)?;
        let minus_x = world_pair_potential_components_for_bodies(
            &translated,
            resonant_indices,
            catalog,
            tick,
        )?
        .iter()
        .map(|component| component.energy)
        .sum::<f64>();

        translated[body_index] =
            translate_pair_potential_body(&bodies[body_index], 0.0, DIFFERENCE_STEP)?;
        let plus_y = world_pair_potential_components_for_bodies(
            &translated,
            resonant_indices,
            catalog,
            tick,
        )?
        .iter()
        .map(|component| component.energy)
        .sum::<f64>();

        translated[body_index] =
            translate_pair_potential_body(&bodies[body_index], 0.0, -DIFFERENCE_STEP)?;
        let minus_y = world_pair_potential_components_for_bodies(
            &translated,
            resonant_indices,
            catalog,
            tick,
        )?
        .iter()
        .map(|component| component.energy)
        .sum::<f64>();

        let force_x = -(plus_x - minus_x) / (2.0 * DIFFERENCE_STEP);
        let force_y = -(plus_y - minus_y) / (2.0 * DIFFERENCE_STEP);
        if !force_x.is_finite() || !force_y.is_finite() {
            return None;
        }
        body_forces[body_index] = (force_x, force_y);
    }

    Some(WorldPairPotentialGradient {
        components,
        body_forces,
        body_force_owners: force_owners.to_vec(),
    })
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
            frequency_cycles_per_day: 440.0,
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
    fn world_pair_potential_oracle_recomputes_all_bodies_with_complete_occlusion() {
        let mut simulation = Simulation::new(77, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);
        let stored = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 5.0,
                y: 2.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("stored blocker should be physically realized");
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance_at_owner_anchor(
                stored,
                Placement {
                    x: 5.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                &catalog,
            ));

        for (name, x) in [("Carbon", 15.0), ("Hydrogen", 20.0), ("Sulfur", 25.0)] {
            let physical = PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("test material should be physically realized");
            assert!(simulation.environment.field.deposit_physical(
                x,
                0.0,
                physical,
                &simulation.environment.catalog,
            ));
        }

        let snapshot = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("the complete physical world should produce a valid exposure snapshot");
        let (bodies, resonant_indices, force_owners) =
            world_physical_materials_for_pair_potential(&simulation)
                .expect("the independent oracle should reconstruct all physical body classes");
        assert_eq!(
            bodies.len(),
            snapshot.occluder_count,
            "the oracle and world-tone snapshot must enumerate the same realized bodies"
        );
        assert_eq!(
            bodies.len(),
            5,
            "one organism, three field bodies, and one stored blocker are expected"
        );
        assert_eq!(
            resonant_indices,
            vec![0, 1, 2, 3],
            "stored material must occlude but must not emit or receive resonance"
        );
        assert_eq!(
            force_owners,
            vec![Some(0), None, None, None, Some(0)],
            "only organism-attached bodies move with their owner"
        );

        let oracle = world_pair_potential_inventory_components(&simulation)
            .expect("every unordered pair should have a finite occlusion-aware potential");
        assert!(
            !oracle.is_empty(),
            "the world pair inventory should retain frequency-resolved components"
        );
        assert!(oracle.iter().all(|component| {
            component.frequency_cycles_per_day.is_finite()
                && component.frequency_cycles_per_day > 0.0
                && component.energy.is_finite()
        }));
        assert!(oracle
            .windows(2)
            .all(|pair| { pair[0].frequency_cycles_per_day < pair[1].frequency_cycles_per_day }));

        let mut independently_summed =
            Vec::<crate::harmonics::ResonancePairPotentialComponent>::new();
        for (first_position, &first) in resonant_indices.iter().enumerate() {
            for &second in resonant_indices.iter().skip(first_position + 1) {
                let pair = crate::harmonics::resonance_pair_potential_components_with_occluders(
                    &bodies[first],
                    &bodies[second],
                    &bodies,
                    &catalog,
                    simulation.tick,
                )
                .expect("each pair should include the remaining bodies as blockers");
                for component in pair {
                    if let Some(existing) = independently_summed.iter_mut().find(|existing| {
                        (existing.frequency_cycles_per_day - component.frequency_cycles_per_day)
                            .abs()
                            <= 1e-9
                    }) {
                        existing.energy += component.energy;
                    } else {
                        independently_summed.push(component);
                    }
                }
            }
        }
        independently_summed.sort_by(|a, b| {
            a.frequency_cycles_per_day
                .partial_cmp(&b.frequency_cycles_per_day)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(oracle, independently_summed);
    }

    #[test]
    fn world_pair_gradient_matches_isolated_pair_and_balances_reaction_forces() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("test material should have valid realized geometry")
        };
        let bodies = vec![
            make_material("Carbon", 0.0, 0.0),
            make_material("Hydrogen", 4.0, 0.0),
        ];
        let gradient =
            world_pair_potential_gradient_for_bodies(&bodies, &[0, 1], &[None, None], &catalog, 0)
                .expect("the isolated world pair should have a finite complete gradient");
        let (force_on_a, force_on_b) =
            crate::harmonics::resonance_pair_force(&bodies[0], &bodies[1], &catalog, 0)
                .expect("the isolated pair should have a gradient-derived force");

        assert_eq!(gradient.body_forces.len(), 2);
        assert!((gradient.body_forces[0].0 - force_on_a.0).abs() < 1e-8);
        assert!((gradient.body_forces[0].1 - force_on_a.1).abs() < 1e-8);
        assert!((gradient.body_forces[1].0 - force_on_b.0).abs() < 1e-8);
        assert!((gradient.body_forces[1].1 - force_on_b.1).abs() < 1e-8);
        assert!((gradient.body_forces[0].0 + gradient.body_forces[1].0).abs() < 1e-8);
        assert!((gradient.body_forces[0].1 + gradient.body_forces[1].1).abs() < 1e-8);
    }

    #[test]
    fn non_resonant_occluding_body_receives_its_world_potential_reaction() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("test material should have valid realized geometry")
        };
        let bodies = vec![
            make_material("Carbon", 0.0, 0.0),
            make_material("Carbon", 8.0, 0.0),
            make_material("Nitrogen", 4.0, 0.4),
        ];
        // The blocker remains physical and occluding but is not a resonance
        // source or receiver until released from storage.
        let gradient = world_pair_potential_gradient_for_bodies(
            &bodies,
            &[0, 1],
            &[None, None, None],
            &catalog,
            0,
        )
        .expect("the complete world potential should have a finite gradient");

        let blocker_force = gradient.body_forces[2];
        assert!(
            blocker_force.0.hypot(blocker_force.1) > 1e-8,
            "moving the blocker changes visibility, so it must receive a reaction force"
        );
        let net_force = gradient
            .body_forces
            .iter()
            .fold((0.0, 0.0), |sum, force| (sum.0 + force.0, sum.1 + force.1));
        assert!(
            net_force.0.abs() < 1e-6 && net_force.1.abs() < 1e-6,
            "complete-world forces, including the blocker, must balance under translation"
        );
    }

    #[test]
    fn pair_potential_oracle_delta_settles_against_the_world_energy_inventory() {
        let mut simulation = Simulation::new(78, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);

        for (name, x) in [("Carbon", 15.0), ("Hydrogen", 20.0), ("Sulfur", 25.0)] {
            let physical = PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("test material should be physically realized");
            assert!(simulation.environment.field.deposit_physical(
                x,
                0.0,
                physical,
                &simulation.environment.catalog,
            ));
        }

        let initial_components = world_pair_potential_inventory_components(&simulation)
            .expect("initial world pair potential should be computable");
        let initial_pair_potential = initial_components
            .iter()
            .map(|component| component.energy)
            .sum::<f64>();

        simulation.organisms[0].structure.units[0].placement.x += 3.0;
        simulation.organisms[0].occupied_cells[0].x += 3.0;
        simulation.organisms[0].mark_structure_changed();
        let current_components = world_pair_potential_inventory_components(&simulation)
            .expect("moved world pair potential should be computable");

        let mut delta_components = current_components.clone();
        for before in &initial_components {
            if let Some(current) = delta_components.iter_mut().find(|current| {
                (current.frequency_cycles_per_day - before.frequency_cycles_per_day).abs() <= 1e-9
            }) {
                current.energy -= before.energy;
            } else {
                delta_components.push(crate::harmonics::ResonancePairPotentialComponent {
                    frequency_cycles_per_day: before.frequency_cycles_per_day,
                    energy: -before.energy,
                });
            }
        }
        delta_components.sort_by(|a, b| {
            a.frequency_cycles_per_day
                .partial_cmp(&b.frequency_cycles_per_day)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let expected_delta = current_components
            .iter()
            .map(|component| component.energy)
            .sum::<f64>()
            - initial_pair_potential;
        assert!(
            expected_delta.abs() > 1e-10,
            "test movement must change pair potential"
        );
        assert!(
            (delta_components
                .iter()
                .map(|component| component.energy)
                .sum::<f64>()
                - expected_delta)
                .abs()
                < 1e-9
        );
        simulation.organisms[0].usable_energy = 1_000.0;

        let initial_usable = simulation
            .organisms
            .iter()
            .map(|organism| organism.usable_energy)
            .sum::<f64>();
        let initial_resonant = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.total_energy().unwrap())
            .sum::<f64>()
            + simulation
                .environment
                .resonant_energy
                .total_energy()
                .unwrap();
        simulation
            .energy_ledger
            .initial_pair_interaction_potential_held = 0.0;
        simulation
            .energy_ledger
            .total_pair_interaction_potential_held = 0.0;
        simulation.energy_ledger.initial_resonant_energy_held = initial_resonant;
        simulation.energy_ledger.total_resonant_energy_held = initial_resonant;

        let ledger = &mut simulation.energy_ledger;
        let mover = &mut simulation.organisms[0];
        assert!(crate::energy_ledger::settle_pair_potential_change(
            ledger,
            &mut mover.usable_energy,
            &mut [&mut mover.resonant_energy],
            &delta_components,
        ));
        assert!((ledger.total_pair_interaction_potential_held - expected_delta).abs() < 1e-9);

        let current_usable = simulation
            .organisms
            .iter()
            .map(|organism| organism.usable_energy)
            .sum::<f64>();
        let current_resonant = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.total_energy().unwrap())
            .sum::<f64>()
            + simulation
                .environment
                .resonant_energy
                .total_energy()
                .unwrap();
        let inventory = crate::energy_ledger::EnergyInventorySnapshot {
            initial_usable_energy: initial_usable,
            current_usable_energy: current_usable,
            initial_resonant_energy: initial_resonant,
            current_resonant_energy: current_resonant,
            initial_structural_energy: 0.0,
            current_structural_energy: 0.0,
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: ledger.total_pair_interaction_potential_held,
            external_energy_input: 0.0,
            external_energy_input_compensation: 0.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 0.0,
            heat_dissipated: ledger.total_heat_dissipated,
        };
        assert!(
            inventory.is_balanced_without_creation(1e-9),
            "the independent world oracle delta must balance against usable energy, compatible resonance, or heat"
        );
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
        assert!(simulation.environment.field.deposit_physical(
            2.5,
            0.0,
            blocker,
            &simulation.environment.catalog,
        ));

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
        assert!(simulation.environment.field.deposit_physical(
            5.0,
            100.0,
            unrelated,
            &simulation.environment.catalog,
        ));

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
            &catalog,
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
    fn snapshot_fails_closed_on_invalid_composite_structural_unit_state() {
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
    fn snapshot_represents_each_constituent_of_composite_physical_material() {
        let mut simulation = Simulation::new(76, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        let composite = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: Vec::new(),
            },
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 1.5,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("composite physical material must carry one realized placement per constituent");
        assert!(simulation.environment.field.deposit_physical(
            0.0,
            0.0,
            composite,
            &simulation.environment.catalog,
        ));

        let snapshot = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("valid composite physical material should not invalidate the world snapshot");
        assert!(
            snapshot
                .body_properties
                .iter()
                .any(|properties| properties.len() == 2),
            "both constituent material responses must be present for composite geometry"
        );
    }

    #[test]
    fn aggregate_owned_developing_member_and_stored_material_remain_world_occluders() {
        let mut simulation = Simulation::new(73, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 20.0, 0.0, &catalog);
        let developing_member_ids = simulation.organisms[0].structure.physical_constituent_ids();
        let anchor_id = developing_member_ids[0];

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
            &catalog,
        ));
        simulation.organisms[0].reproductive_construction =
            Some(crate::state::ReproductiveConstruction {
                committed_material,
                developing_member_ids,
                developing_structure: OrganismStructure::new(),
                child_genome: simulation.organisms[0].genome.clone(),
                developmental_origin: Position { x: 20.0, y: 0.0 },
                developmental_orientation_radians: 0.0,
                developing_stress: 0.0,
                anchor_unit_index: 0,
                anchor_physical_id: Some(anchor_id),
                developing_energy: 0.0,
                developing_maintenance_debt: 0.0,
                needs_space: false,
            });

        let snapshot = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("developing physical material must be represented");
        assert_eq!(snapshot.occluder_count, 2);
        assert_eq!(snapshot.body_index_for_organism(0), Some(0));
        assert_eq!(
            snapshot.body_resonance_owners,
            vec![Some(0), None],
            "the attached developing structure belongs to the parent resonance; committed storage remains a separate nonliving receiver"
        );
    }

    #[test]
    fn nonliving_surface_receives_world_tone_energy_as_heat_without_living_receivers() {
        let mut simulation = Simulation::new(77, 1.0);
        clear_environment_physical_material(&mut simulation);
        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);

        let external_before = simulation.energy_ledger.total_external_energy_input;
        let heat_before = simulation.energy_ledger.total_heat_dissipated;
        let escaped_before = simulation.energy_ledger.total_world_tone_energy_escaped;
        let resonant_before =
            crate::reproduction::total_resonant_energy_held(&simulation.organisms);

        WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample(),
            &std::collections::HashSet::new(),
        );

        let external_delta = simulation.energy_ledger.total_external_energy_input - external_before;
        let heat_delta = simulation.energy_ledger.total_heat_dissipated - heat_before;
        let escaped_delta =
            simulation.energy_ledger.total_world_tone_energy_escaped - escaped_before;
        let resonant_delta = crate::reproduction::total_resonant_energy_held(&simulation.organisms)
            - resonant_before;

        assert!(
            external_delta > 0.0,
            "nonliving physical surfaces still intercept the external world-tone source"
        );
        assert!(
            heat_delta > 0.0,
            "nonliving material response must be accounted as heat"
        );
        assert!(resonant_delta.abs() <= 1e-9);
        assert!(
            (external_delta - heat_delta - escaped_delta).abs() <= 1e-7,
            "the nonliving-only world-tone frame must balance into heat and escaped signal"
        );
    }

    #[test]
    fn attached_developing_structure_can_retain_world_tone_for_its_owner() {
        let mut simulation = Simulation::new(74, 1.0);
        clear_environment_physical_material(&mut simulation);
        simulation.decomposing_bodies.clear();
        simulation.active_transformations.clear();
        let catalog = simulation.environment.catalog.clone();

        // The attached member is represented by the parent's authoritative
        // structure; the construction record stores only its stable IDs.
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);
        let member_ids = simulation.organisms[0].structure.physical_constituent_ids();
        let anchor_id = member_ids[0];
        let owner_id = simulation.organisms[0].id.clone();
        simulation.organisms[0].reproductive_construction =
            Some(crate::state::ReproductiveConstruction {
                committed_material: crate::material_storage::MaterialStorage::default(),
                developing_member_ids: member_ids,
                developing_structure: OrganismStructure::new(),
                child_genome: simulation.organisms[0].genome.clone(),
                developmental_origin: Position { x: 5.0, y: 0.0 },
                developmental_orientation_radians: 0.0,
                developing_stress: 0.0,
                anchor_unit_index: 0,
                anchor_physical_id: Some(anchor_id),
                developing_energy: 0.0,
                developing_maintenance_debt: 0.0,
                needs_space: false,
            });

        let structural = Simulation::structural_energy_held(
            &simulation.organisms,
            &simulation.decomposing_bodies,
        );
        let resonant = crate::reproduction::total_resonant_energy_held(&simulation.organisms);
        simulation.energy_ledger.initial_structural_energy_held = structural;
        simulation.energy_ledger.total_structural_energy_held = structural;
        simulation.energy_ledger.initial_resonant_energy_held = resonant;
        simulation.energy_ledger.total_resonant_energy_held = resonant;
        simulation.energy_ledger.total_usable_energy_held =
            crate::reproduction::total_usable_energy_held(
                &simulation.organisms,
                &simulation.decomposing_bodies,
            );

        let external_before = simulation.energy_ledger.total_external_energy_input;
        let heat_before = simulation.energy_ledger.total_heat_dissipated;
        let escaped_before = simulation.energy_ledger.total_world_tone_energy_escaped;
        WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample(),
            &std::collections::HashSet::from([owner_id]),
        );

        let retained = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("owner resonance reservoir remains valid");
        assert!(
            retained > 0.0,
            "attached developing material must credit its living owner's resonance reservoir"
        );
        let settled =
            crate::reproduction::total_resonant_energy_held(&simulation.organisms) - resonant;
        let ledger_resonant_delta = simulation.energy_ledger.total_resonant_energy_held - resonant;
        assert!(
            (ledger_resonant_delta - settled).abs() <= 1e-7,
            "world-tone ledger must track the resonance actually retained by the owner"
        );
        assert!(
            ((simulation.energy_ledger.total_external_energy_input - external_before)
                - ledger_resonant_delta
                - (simulation.energy_ledger.total_heat_dissipated - heat_before)
                - (simulation.energy_ledger.total_world_tone_energy_escaped - escaped_before))
                .abs()
                <= 1e-7,
            "attached-structure resonance must remain within the source/sink budget"
        );
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
            &catalog,
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
                stored_source_index: None,
                environmental_source_location: None,
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
                prepared_ledger_before: None,
                prepared_usable_energy_before: None,
                combine_environmental_source: None,
                combine_environmental_source_index: None,
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
        assert!(simulation.environment.field.deposit_physical(
            5.0,
            0.0,
            exposed,
            &simulation.environment.catalog,
        ));

        // The fixture intentionally removes the initial organisms' structures
        // above. Rebase the structural inventory baseline so this test measures
        // only the world-tone source/capture/heat/escape flow below.
        simulation.energy_ledger.initial_structural_energy_held =
            crate::reproduction::total_structural_energy_held(
                &simulation.organisms,
                &simulation.decomposing_bodies,
            );

        WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample(),
            &std::collections::HashSet::new(),
        );

        assert!(
            simulation.energy_ledger.total_external_energy_input > 0.0,
            "physical material must receive the source even when no living organism is present"
        );
        // This material may retain the entire non-damped response, so zero
        // incremental heat is valid as long as the captured inventory balances.
        let environmental_resonance = simulation
            .environment
            .resonant_energy
            .total_energy()
            .expect("environment resonance should remain valid");
        assert!(
            environmental_resonance > 0.0,
            "sunlight-exposed environmental material must retain compatible resonance"
        );
        assert!(
            (simulation.energy_ledger.total_resonant_energy_held - environmental_resonance).abs()
                < 1e-9,
            "the ledger must include resonance retained by the environmental field"
        );
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-7),
            "environment resonance capture must conserve the world energy inventory"
        );
    }

    #[test]
    fn ordered_living_and_environmental_receivers_conserve_one_travelling_signal() {
        let mut simulation = Simulation::new(83, 1.0);
        clear_environment_physical_material(&mut simulation);
        for organism in &mut simulation.organisms {
            organism.structure = OrganismStructure::new();
            organism.stored_material.entries.clear();
            organism.reproductive_construction = None;
        }
        simulation.decomposing_bodies.clear();
        simulation.active_transformations.clear();

        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);
        let living_id = simulation.organisms[0].id.clone();

        let downstream = PhysicalMaterial::realized(
            Material::free_base("Nitrogen", 1.0),
            vec![Placement {
                x: 8.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("downstream environmental receiver should have realized geometry");
        assert!(simulation.environment.field.deposit_physical(
            8.0,
            0.0,
            downstream,
            &simulation.environment.catalog,
        ));

        // Rebase the independent inventory oracle on the deliberately
        // reshaped fixture so the following residual measures only world-tone
        // source and sink flows, not test setup edits.
        simulation.energy_ledger.initial_usable_energy_held =
            crate::reproduction::total_usable_energy_held(
                &simulation.organisms,
                &simulation.decomposing_bodies,
            );
        simulation.energy_ledger.initial_resonant_energy_held =
            crate::reproduction::total_resonant_energy_held(&simulation.organisms);
        simulation.energy_ledger.initial_structural_energy_held =
            Simulation::structural_energy_held(
                &simulation.organisms,
                &simulation.decomposing_bodies,
            );
        // Simulation::new may have accumulated setup-time flows. The oracle
        // below is intentionally scoped to this reshaped fixture, so rebase
        // every cumulative flow alongside the three held-energy baselines.
        simulation.energy_ledger.total_external_energy_input = 0.0;
        simulation.energy_ledger.total_potential_energy_released = 0.0;
        simulation.energy_ledger.total_work_expended = 0.0;
        simulation.energy_ledger.total_heat_dissipated = 0.0;
        simulation.energy_ledger.total_world_tone_energy_escaped = 0.0;

        let snapshot = WorldToneOccluderSnapshot::from_simulation(sample(), &simulation)
            .expect("both physical receivers should be represented");
        assert!(
            snapshot.frame.interaction_paths().iter().any(|path| {
                path.hits
                    .windows(2)
                    .any(|hits| hits[0] == (0, 0) && hits[1] == (1, 0))
            }),
            "at least one source lane must hit the living receiver before the environmental receiver"
        );

        let before_retained: f64 = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.total_energy().unwrap())
            .sum();
        let before_external = simulation.energy_ledger.total_external_energy_input;
        let before_heat = simulation.energy_ledger.total_heat_dissipated;
        let before_escaped = simulation.energy_ledger.total_world_tone_energy_escaped;

        WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample(),
            &std::collections::HashSet::from([living_id]),
        );

        let retained: f64 = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.total_energy().unwrap())
            .sum();
        let ledger = &simulation.energy_ledger;
        assert!(
            retained > 0.0,
            "the living receiver should retain compatible response energy"
        );
        let incident_input = ledger.total_external_energy_input - before_external;
        let heat_dissipated = ledger.total_heat_dissipated - before_heat;
        let escaped = ledger.total_world_tone_energy_escaped - before_escaped;
        let retained_delta = retained - before_retained;
        assert!(incident_input > 0.0);
        assert!(
            heat_dissipated > 0.0,
            "damped living response and the nonliving receiver must settle as heat"
        );
        let allocated = retained_delta + heat_dissipated + escaped;
        assert!(
            (incident_input - allocated).abs()
                <= 1e-7 * incident_input.max(1.0),
            "the newly incident source energy must be allocated once across retained resonance, heat, and escape"
        );
        let inventory = simulation.energy_inventory_snapshot();
        assert!(
            inventory.is_balanced(1e-7),
            "world-tone source and sink flows must reconcile with the full world inventory: residual={:?}",
            inventory.residual()
        );
    }

    #[test]
    fn rejected_world_tone_ledger_settlement_does_not_commit_staged_resonance() {
        let mut simulation = Simulation::new(89, 1.0);
        clear_environment_physical_material(&mut simulation);
        for organism in &mut simulation.organisms {
            organism.structure = OrganismStructure::new();
            organism.stored_material.entries.clear();
            organism.reproductive_construction = None;
        }
        simulation.decomposing_bodies.clear();
        simulation.active_transformations.clear();

        let catalog = simulation.environment.catalog.clone();
        replace_with_nitrogen_square(&mut simulation.organisms[0], 5.0, 0.0, &catalog);
        let living_id = simulation.organisms[0].id.clone();
        let before_retained: Vec<f64> = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.total_energy().unwrap())
            .collect();
        let before_external = simulation.energy_ledger.total_external_energy_input;
        let before_escaped = simulation.energy_ledger.total_world_tone_energy_escaped;
        simulation.energy_ledger.total_heat_dissipated = f64::NAN;

        WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample(),
            &std::collections::HashSet::from([living_id]),
        );

        let after_retained: Vec<f64> = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.total_energy().unwrap())
            .collect();
        assert_eq!(
            after_retained, before_retained,
            "staged resonance must not commit if cumulative heat settlement overflows"
        );
        assert_eq!(
            simulation.energy_ledger.total_external_energy_input,
            before_external
        );
        assert_eq!(
            simulation.energy_ledger.total_world_tone_energy_escaped,
            before_escaped
        );
        assert!(simulation.energy_ledger.total_heat_dissipated.is_nan());
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
