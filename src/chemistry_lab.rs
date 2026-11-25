//! Dedicated experimental workbench backed by the shared EvoSim engine.

use crate::{
    environment::ActiveMaterialField,
    physical_material::PhysicalMaterial,
    resources::{InternalBond, Material},
    state::{EnergyLedger, Environment, Simulation},
    structure::Placement,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LaboratoryFlowConfig {
    /// Mean current direction in degrees; zero points right and 90 points down.
    pub direction_degrees: f64,
    /// Flow speed multiplier. Zero disables ambient transport.
    pub speed: f64,
    /// Blend between a uniform current (0) and the native S-shaped field (1).
    pub strength: f64,
    /// Local recirculation intensity around the two S-bend vortices.
    pub vortex_strength: f64,
}

impl Default for LaboratoryFlowConfig {
    fn default() -> Self {
        Self {
            direction_degrees: 0.0,
            speed: 1.0,
            strength: 1.0,
            vortex_strength: 1.0,
        }
    }
}

impl LaboratoryFlowConfig {
    fn validate(self) -> Result<(), &'static str> {
        if !self.direction_degrees.is_finite()
            || !(-360.0..=360.0).contains(&self.direction_degrees)
        {
            return Err("flow direction must be in [-360, 360] degrees");
        }
        if !self.speed.is_finite() || !(0.0..=10.0).contains(&self.speed) {
            return Err("flow speed must be in [0, 10]");
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return Err("flow strength must be in [0, 1]");
        }
        if !self.vortex_strength.is_finite() || !(0.0..=2.0).contains(&self.vortex_strength) {
            return Err("local vortex strength must be in [0, 2]");
        }
        Ok(())
    }

    fn engine_config(self) -> crate::environment_flow::FlowFieldConfig {
        crate::environment_flow::FlowFieldConfig {
            direction_degrees: self.direction_degrees,
            speed: self.speed,
            strength: self.strength,
            vortex_strength: self.vortex_strength,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaboratoryConfig {
    pub width: f64,
    pub height: f64,
    pub cell_size: f64,
    pub ticks_per_second: f64,
    pub seed: u64,
    #[serde(default)]
    pub flow: LaboratoryFlowConfig,
    #[serde(default)]
    pub world_tone_enabled: bool,
    #[serde(default)]
    pub hotspot_enabled: bool,
}

impl Default for LaboratoryConfig {
    fn default() -> Self {
        Self {
            width: 100.0,
            height: 100.0,
            cell_size: 5.0,
            ticks_per_second: 20.0,
            seed: 42,
            flow: LaboratoryFlowConfig::default(),
            world_tone_enabled: false,
            hotspot_enabled: false,
        }
    }
}

impl LaboratoryConfig {
    fn validate(&self) -> Result<(), &'static str> {
        if !self.width.is_finite() || self.width <= 0.0 || self.width > 10_000.0 {
            return Err("width must be in (0, 10000]");
        }
        if !self.height.is_finite() || self.height <= 0.0 || self.height > 10_000.0 {
            return Err("height must be in (0, 10000]");
        }
        if !self.cell_size.is_finite() || self.cell_size < 1.0 || self.cell_size > 500.0 {
            return Err("cell_size must be in [1, 500]");
        }
        if !self.ticks_per_second.is_finite()
            || self.ticks_per_second <= 0.0
            || self.ticks_per_second > 10_000.0
        {
            return Err("ticks_per_second must be in (0, 10000]");
        }
        let width_cells = (self.width / self.cell_size).ceil();
        let height_cells = (self.height / self.cell_size).ceil();
        if !width_cells.is_finite()
            || !height_cells.is_finite()
            || width_cells * height_cells > 1_000_000.0
        {
            return Err("laboratory grid cannot exceed 1000000 cells");
        }
        self.flow.validate()?;
        if self.world_tone_enabled
            && crate::world_tone::WorldToneRuntimeConfig::default_for_world(self.width, self.height)
                .is_none()
        {
            return Err("world-tone source is invalid for these dimensions");
        }
        if self.hotspot_enabled
            && crate::hotspot::RadialHotspot::right_bend_for_world(self.width, self.height)
                .is_none()
        {
            return Err("hotspot source is invalid for these dimensions");
        }
        Ok(())
    }
}

pub struct Laboratory {
    pub config: LaboratoryConfig,
    simulation: Simulation,
    run_generation: u64,
    initial_placements: Vec<LaboratoryInitialPlacement>,
    reaction_history: Vec<ReactionEventSnapshot>,
}

impl Laboratory {
    pub fn new(config: LaboratoryConfig) -> Result<Self, &'static str> {
        config.validate()?;
        let environment = Environment {
            width: config.width,
            height: config.height,
            field: ActiveMaterialField::new(config.width, config.height, config.cell_size),
            catalog: crate::resources::default_catalog(),
            chemistry_usable_energy: 0.0,
            resonant_energy: Default::default(),
        };
        let ambient_config = crate::world_generated_ambient_mode::AmbientModeConfig::new(
            config.width * config.height,
            1.0,
            0.0,
            0.0,
        )
        .expect("laboratory ambient energy configuration must be valid");
        let ambient_energy =
            crate::world_generated_ambient_mode::WorldGeneratedAmbientMode::new(ambient_config, 0)
                .expect("laboratory ambient energy must initialize at tick zero");
        let opening_ambient_energy = ambient_energy.sample().total_energy;
        let resonance_field = crate::live_resonance_field::LiveResonanceField::new(
            config.width,
            config.height,
            ambient_energy
                .sample_midpoint_before_current()
                .map_err(|_| "initial ambient carrier midpoint is unavailable")?,
        )
        .map_err(|_| "live spatial resonance field could not initialize")?;
        let opening_spatial_field_energy = resonance_field.initial_total_energy();
        let spatial_energy_state = resonance_field.energy_state();
        let energy_ledger = EnergyLedger {
            total_ambient_energy_held: opening_ambient_energy,
            initial_ambient_energy_held: opening_ambient_energy,
            total_spatial_disturbance_energy_held: spatial_energy_state.disturbance_energy,
            total_spatial_cross_energy_held: spatial_energy_state.cross_energy,
            total_spatial_field_energy_held: spatial_energy_state.total_energy,
            initial_spatial_field_energy_held: opening_spatial_field_energy,
            ..EnergyLedger::default()
        };
        let simulation = Simulation {
            tick: 0,
            ticks_per_second: config.ticks_per_second,
            running: false,
            organisms: Vec::new(),
            environment,
            active_transformations: Vec::new(),
            chemical_reaction_accumulation: Default::default(),
            chemistry_library: crate::chemistry_library::open_default_library()
                .expect("chemistry library must open"),
            decomposing_bodies: Vec::new(),
            energy_ledger,
            next_organism_id: 1,
            next_transformation_id: 1,
            rng: ChaCha8Rng::seed_from_u64(config.seed),
            decision_parameters: crate::decision::DecisionParameters::default(),
            seed_scale_reference: None,
            world_tone: if config.world_tone_enabled {
                crate::world_tone::WorldToneRuntimeConfig::default_for_world(
                    config.width,
                    config.height,
                )
            } else {
                None
            },
            world_tone_cycle: if config.world_tone_enabled {
                crate::world_tone::WorldToneRuntimeConfig::default_for_world(
                    config.width,
                    config.height,
                )
                .and_then(|source| {
                    crate::world_tone::WorldToneCycle::precompute(
                        source,
                        config.width,
                        config.height,
                    )
                })
            } else {
                None
            },
            hotspot: if config.hotspot_enabled {
                crate::hotspot::RadialHotspot::right_bend_for_world(config.width, config.height)
            } else {
                None
            },
            environmental_chemistry_diagnostics: Default::default(),
            environmental_chemistry_events: Vec::new(),
            environmental_local_resonance_samples: Vec::new(),
            record_local_resonance_diagnostics: false,
            record_environmental_molecular_history: true,
            flow_field: config.flow.engine_config(),
            ambient_energy,
            resonance_field,
        };
        Ok(Self {
            config,
            simulation,
            run_generation: 0,
            initial_placements: Vec::new(),
            reaction_history: Vec::new(),
        })
    }

    fn capture_current_tick_events(&mut self) {
        for event in &self.simulation.environmental_chemistry_events {
            self.reaction_history.push(ReactionEventSnapshot {
                tick: event.tick,
                reaction: event.reaction.clone(),
                source_cells: event.source_cells.clone(),
                source_materials: event.source_materials.clone(),
                work_required: event.work_required,
                reaction_frequency_hz: event.reaction_frequency_hz,
                activation_signal: event.activation_signal,
                accumulated_reaction: event.accumulated_reaction,
                outcome: event.outcome.clone(),
                failure_reason: event.failure_reason.clone(),
                product_material: event.product_material.clone(),
                product_internal_bond_count: event.product_internal_bond_count,
                molecular_lifecycle: event
                    .molecular_lifecycle
                    .iter()
                    .map(|lifecycle| MolecularLifecycleSnapshot {
                        action: lifecycle.action.clone(),
                        tick: lifecycle.tick,
                        molecule_id: lifecycle.molecule_id,
                        parent_ids: lifecycle.parent_ids.clone(),
                        reaction_id: lifecycle.reaction_id.clone(),
                        reaction: lifecycle.reaction.clone(),
                        molecule: lifecycle.molecule.clone(),
                    })
                    .collect(),
            });
        }
        const MAX_REACTION_HISTORY: usize = 2_000;
        if self.reaction_history.len() > MAX_REACTION_HISTORY {
            let excess = self.reaction_history.len() - MAX_REACTION_HISTORY;
            self.reaction_history.drain(..excess);
        }
    }

    pub fn document(&self) -> LaboratoryDocument {
        LaboratoryDocument {
            config: self.config.clone(),
            initial_placements: self.initial_placements.clone(),
        }
    }

    pub fn from_document(document: LaboratoryDocument) -> Result<Self, &'static str> {
        if document.initial_placements.len() > 10_000 {
            return Err("experiment setup cannot contain more than 10000 placements");
        }
        let mut laboratory = Self::new(document.config)?;
        for placement in document.initial_placements {
            match placement {
                LaboratoryInitialPlacement::Element { resource, x, y } => {
                    laboratory.place_element(&resource, x, y)?;
                }
                LaboratoryInitialPlacement::Molecule {
                    parts,
                    bonds,
                    placements,
                } => {
                    laboratory.place_molecule(parts, bonds, placements)?;
                }
            }
        }
        Ok(laboratory)
    }

    /// Enable or disable the existing, ledger-settled external energy sources.
    /// This selects established source models; it does not create a lab-only energy grant.
    pub fn configure_energy_sources(
        &mut self,
        world_tone_enabled: bool,
        hotspot_enabled: bool,
    ) -> Result<(), &'static str> {
        let world_tone = if world_tone_enabled {
            Some(
                crate::world_tone::WorldToneRuntimeConfig::default_for_world(
                    self.config.width,
                    self.config.height,
                )
                .ok_or("world-tone source is invalid for these dimensions")?,
            )
        } else {
            None
        };
        let world_tone_cycle = world_tone.and_then(|source| {
            crate::world_tone::WorldToneCycle::precompute(
                source,
                self.config.width,
                self.config.height,
            )
        });
        if world_tone.is_some() && world_tone_cycle.is_none() {
            return Err("world-tone cycle could not be prepared");
        }
        let hotspot = if hotspot_enabled {
            Some(
                crate::hotspot::RadialHotspot::right_bend_for_world(
                    self.config.width,
                    self.config.height,
                )
                .ok_or("hotspot source is invalid for these dimensions")?,
            )
        } else {
            None
        };
        self.simulation.world_tone = world_tone;
        self.simulation.world_tone_cycle = world_tone_cycle;
        self.simulation.hotspot = hotspot;
        self.config.world_tone_enabled = world_tone_enabled;
        self.config.hotspot_enabled = hotspot_enabled;
        Ok(())
    }

    /// Replace the ambient vector field immediately, including during a live run.
    pub fn configure_flow(&mut self, flow: LaboratoryFlowConfig) -> Result<(), &'static str> {
        flow.validate()?;
        self.config.flow = flow;
        self.simulation.flow_field = flow.engine_config();
        Ok(())
    }

    pub fn place_element(&mut self, resource: &str, x: f64, y: f64) -> Result<u64, &'static str> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= self.config.width
            || y >= self.config.height
        {
            return Err("placement must be inside laboratory bounds");
        }
        let identity = self.place_material(
            Material::free_base(resource.to_string(), 1.0),
            vec![Placement {
                x,
                y,
                rotation_radians: 0.0,
            }],
            x,
            y,
        )?;
        self.initial_placements
            .push(LaboratoryInitialPlacement::Element {
                resource: resource.to_string(),
                x,
                y,
            });
        Ok(identity)
    }

    pub fn place_molecule(
        &mut self,
        parts: Vec<String>,
        bonds: Vec<[usize; 2]>,
        placements: Vec<[f64; 3]>,
    ) -> Result<u64, &'static str> {
        if parts.is_empty() || parts.len() > 64 || placements.len() != parts.len() {
            return Err("molecule needs 1–64 parts and one placement per part");
        }
        if parts.iter().any(|name| {
            !self
                .simulation
                .environment
                .catalog
                .iter()
                .any(|resource| resource.name == *name)
        }) {
            return Err("molecule contains an unknown resource type");
        }
        if placements.iter().any(|placement| {
            !placement[0].is_finite()
                || !placement[1].is_finite()
                || !placement[2].is_finite()
                || placement[0] < 0.0
                || placement[1] < 0.0
                || placement[0] >= self.config.width
                || placement[1] >= self.config.height
        }) {
            return Err("all constituent placements must be inside laboratory bounds");
        }

        let saved_parts = parts.clone();
        let saved_bonds = bonds.clone();
        let saved_placements = placements.clone();
        let mut internal_bonds = Vec::with_capacity(bonds.len());
        for [part_a, part_b] in bonds {
            let bond = InternalBond { part_a, part_b };
            if !bond.is_valid_for(parts.len())
                || internal_bonds.contains(&bond)
                || internal_bonds
                    .iter()
                    .any(|prior: &InternalBond| prior.part_a == part_b && prior.part_b == part_a)
            {
                return Err("molecule contains an invalid or duplicate bond");
            }
            internal_bonds.push(bond);
        }

        let material = Material {
            parts: parts.into_iter().map(|name| (name, 1.0)).collect(),
            internal_bonds,
        };
        let placements = placements
            .into_iter()
            .map(|placement| Placement {
                x: placement[0],
                y: placement[1],
                rotation_radians: placement[2],
            })
            .collect::<Vec<_>>();
        let anchor = placements.first().ok_or("molecule has no placement")?;
        let (anchor_x, anchor_y) = (anchor.x, anchor.y);
        let identity = self.place_material(material, placements, anchor_x, anchor_y)?;
        self.initial_placements
            .push(LaboratoryInitialPlacement::Molecule {
                parts: saved_parts,
                bonds: saved_bonds,
                placements: saved_placements,
            });
        Ok(identity)
    }

    fn place_material(
        &mut self,
        material: Material,
        placements: Vec<Placement>,
        x: f64,
        y: f64,
    ) -> Result<u64, &'static str> {
        if self.simulation.tick != 0 || self.simulation.running {
            return Err("initial material placement requires a paused tick-zero experiment");
        }
        let identity = {
            let catalog = &self.simulation.environment.catalog;
            let physical = PhysicalMaterial::realized(material, placements, catalog)
                .ok_or("physical realization or bond geometry validation failed")?;
            let candidate_placements = physical
                .placements
                .as_ref()
                .ok_or("realized material has no constituent placements")?;
            let candidate_shapes = physical
                .realized_shapes_for_parts(catalog)
                .ok_or("realized material has invalid constituent geometry")?;
            let candidate_parts = candidate_placements
                .iter()
                .copied()
                .zip(candidate_shapes)
                .enumerate()
                .map(|(part_index, (placement, shape))| {
                    crate::material_geometry::PlacedMaterialPart {
                        part_index,
                        form: shape.form,
                        placement,
                    }
                })
                .collect::<Vec<_>>();
            for candidate_part in &candidate_parts {
                let radius = candidate_part.form.bounding_radius();
                if !radius.is_finite()
                    || radius < 0.0
                    || candidate_part.placement.x - radius < 0.0
                    || candidate_part.placement.x + radius > self.config.width
                    || candidate_part.placement.y - radius < 0.0
                    || candidate_part.placement.y + radius > self.config.height
                {
                    return Err("material geometry must remain inside laboratory bounds");
                }
            }
            let field = &self.simulation.environment.field;
            for existing in field
                .cells
                .iter()
                .flat_map(|cell| cell.physical_materials.iter())
            {
                if !existing.is_realized() {
                    return Err("existing material has invalid physical geometry");
                }
                let existing_placements = existing
                    .placements
                    .as_ref()
                    .ok_or("existing material has no constituent placements")?;
                let existing_shapes = existing
                    .realized_shapes_for_parts(catalog)
                    .ok_or("existing material has invalid constituent geometry")?;
                for (part_index, (placement, shape)) in existing_placements
                    .iter()
                    .copied()
                    .zip(existing_shapes)
                    .enumerate()
                {
                    let existing_part = crate::material_geometry::PlacedMaterialPart {
                        part_index,
                        form: shape.form,
                        placement,
                    };
                    if candidate_parts.iter().any(|candidate_part| {
                        crate::material_geometry::placed_forms_penetrate(
                            candidate_part,
                            &existing_part,
                            crate::material_geometry::GEOMETRY_EPSILON,
                        )
                    }) {
                        return Err("material placement would penetrate existing material");
                    }
                }
            }
            let field = &mut self.simulation.environment.field;
            if !field.deposit_physical(x, y, physical, catalog) {
                return Err("environment rejected material");
            }
            field.assign_missing_material_identities();
            let index = field
                .index_for_position(x, y)
                .ok_or("invalid destination")?;
            field.cells[index]
                .physical_materials
                .last()
                .map(|material| material.environmental_identity)
                .ok_or("identity assignment failed")?
        };

        // Placed bonded molecules are part of the experiment's initial
        // condition, not a reaction. Move the tick-zero structural baseline
        // with that explicitly supplied initial material so its bond potential
        // is not mistaken for energy created by the simulator.
        let initial_structural_energy = self
            .simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(|material| {
                material
                    .internal_connections
                    .as_ref()
                    .map(|bonds| -bonds.iter().map(|bond| bond.bond_energy).sum::<f64>())
                    .unwrap_or(0.0)
            })
            .sum::<f64>();
        if !initial_structural_energy.is_finite() {
            return Err("initial structural energy is not finite");
        }
        self.simulation.energy_ledger.total_structural_energy_held = initial_structural_energy;
        self.simulation.energy_ledger.initial_structural_energy_held = initial_structural_energy;
        Ok(identity)
    }

    pub fn step(&mut self, count: u32) -> Result<(), &'static str> {
        if count == 0 || count > 100_000 {
            return Err("count must be in [1, 100000]");
        }
        for _ in 0..count {
            self.simulation.step();
            self.capture_current_tick_events();
        }
        Ok(())
    }

    pub fn snapshot(&self) -> LaboratorySnapshot {
        let materials = self
            .simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(|material| {
                let shapes = (0..material.material.parts.len())
                    .map(|part_index| {
                        material
                            .realized_shape_for_part(
                                part_index,
                                &self.simulation.environment.catalog,
                            )
                            .map(|shape| shape.form)
                    })
                    .collect();
                MaterialSnapshot {
                    identity: material.environmental_identity,
                    parts: material.material.parts.clone(),
                    placements: material
                        .placements
                        .clone()
                        .unwrap_or_default()
                        .iter()
                        .map(|placement| [placement.x, placement.y, placement.rotation_radians])
                        .collect(),
                    shapes,
                    bond_count: material.internal_connections.as_ref().map_or(0, Vec::len),
                    bonds: material
                        .material
                        .internal_bonds
                        .iter()
                        .map(|bond| [bond.part_a, bond.part_b])
                        .collect(),
                }
            })
            .collect();
        LaboratorySnapshot {
            tick: self.simulation.tick,
            running: self.simulation.running,
            resources: self
                .simulation
                .environment
                .catalog
                .iter()
                .map(|resource| resource.name.clone())
                .collect(),
            width: self.config.width,
            height: self.config.height,
            material_count: self
                .simulation
                .environment
                .field
                .cells
                .iter()
                .map(|cell| cell.physical_materials.len())
                .sum(),
            materials,
            chemistry_usable_energy: self.simulation.environment.chemistry_usable_energy,
            resonant_energy: self.simulation.environment.resonant_energy.total_energy(),
            organisms: self.simulation.organisms.len(),
            flow: self.config.flow,
            world_tone_enabled: self.config.world_tone_enabled,
            hotspot_enabled: self.config.hotspot_enabled,
            reaction_history: self.reaction_history.clone(),
            molecular_network: molecular_network_snapshot(&self.simulation, &self.reaction_history),
            energy: EnergyLedgerSnapshot {
                external_input: self.simulation.energy_ledger.total_external_energy_input,
                usable_energy_gained: self.simulation.energy_ledger.total_usable_energy_gained,
                work_expended: self.simulation.energy_ledger.total_work_expended,
                heat_dissipated: self.simulation.energy_ledger.total_heat_dissipated,
                usable_energy_held: self.simulation.energy_ledger.total_usable_energy_held,
                resonant_energy_held: self.simulation.energy_ledger.total_resonant_energy_held,
                structural_energy_held: self.simulation.energy_ledger.total_structural_energy_held,
                pair_interaction_potential_held: self
                    .simulation
                    .energy_ledger
                    .total_pair_interaction_potential_held,
                world_tone_unabsorbed: self
                    .simulation
                    .energy_ledger
                    .total_world_tone_energy_unabsorbed,
                world_tone_escaped: self
                    .simulation
                    .energy_ledger
                    .total_world_tone_energy_escaped,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MolecularNetworkSnapshot {
    pub molecule_nodes: usize,
    pub lineage_edges: usize,
    pub family_count: usize,
    pub largest_family_size: usize,
    pub reaction_types: Vec<String>,
    pub edges: Vec<[u64; 2]>,
}

fn molecular_network_snapshot(
    simulation: &Simulation,
    history: &[ReactionEventSnapshot],
) -> MolecularNetworkSnapshot {
    let mut nodes = std::collections::BTreeSet::new();
    let mut edges = std::collections::BTreeSet::new();
    let mut reaction_types = std::collections::BTreeSet::new();

    for material in simulation
        .environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
    {
        nodes.insert(material.environmental_identity);
    }

    for event in history {
        reaction_types.insert(event.reaction.clone());
        for lifecycle in &event.molecular_lifecycle {
            nodes.insert(lifecycle.molecule_id);
            for parent_id in &lifecycle.parent_ids {
                nodes.insert(*parent_id);
                edges.insert((*parent_id, lifecycle.molecule_id));
            }
        }
    }

    let mut adjacency: std::collections::BTreeMap<u64, Vec<u64>> =
        std::collections::BTreeMap::new();
    for (parent, child) in &edges {
        adjacency.entry(*parent).or_default().push(*child);
        adjacency.entry(*child).or_default().push(*parent);
    }
    let mut visited = std::collections::BTreeSet::new();
    let mut family_count = 0;
    let mut largest_family_size = 0;
    for node in &nodes {
        if !visited.insert(*node) {
            continue;
        }
        family_count += 1;
        let mut pending = vec![*node];
        let mut family_size = 0;
        while let Some(current) = pending.pop() {
            family_size += 1;
            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if visited.insert(*neighbor) {
                        pending.push(*neighbor);
                    }
                }
            }
        }
        largest_family_size = largest_family_size.max(family_size);
    }

    MolecularNetworkSnapshot {
        molecule_nodes: nodes.len(),
        lineage_edges: edges.len(),
        family_count,
        largest_family_size,
        reaction_types: reaction_types.into_iter().collect(),
        edges: edges
            .into_iter()
            .map(|(parent, child)| [parent, child])
            .collect(),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnergyLedgerSnapshot {
    pub external_input: f64,
    pub usable_energy_gained: f64,
    pub work_expended: f64,
    pub heat_dissipated: f64,
    pub usable_energy_held: f64,
    pub resonant_energy_held: f64,
    pub structural_energy_held: f64,
    pub pair_interaction_potential_held: f64,
    pub world_tone_unabsorbed: f64,
    pub world_tone_escaped: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReactionEventSnapshot {
    pub tick: u64,
    pub reaction: String,
    pub source_cells: Vec<usize>,
    pub source_materials: Vec<Vec<(String, f64)>>,
    pub work_required: f64,
    pub reaction_frequency_hz: Option<f64>,
    pub activation_signal: Option<f64>,
    pub accumulated_reaction: Option<f64>,
    pub outcome: String,
    pub failure_reason: Option<String>,
    pub product_material: Option<Vec<(String, f64)>>,
    pub product_internal_bond_count: Option<usize>,
    pub molecular_lifecycle: Vec<MolecularLifecycleSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MolecularLifecycleSnapshot {
    pub action: String,
    pub tick: u64,
    pub molecule_id: u64,
    pub parent_ids: Vec<u64>,
    pub reaction_id: String,
    pub reaction: String,
    pub molecule: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaboratoryDocument {
    pub config: LaboratoryConfig,
    pub initial_placements: Vec<LaboratoryInitialPlacement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LaboratoryInitialPlacement {
    Element {
        resource: String,
        x: f64,
        y: f64,
    },
    Molecule {
        parts: Vec<String>,
        bonds: Vec<[usize; 2]>,
        placements: Vec<[f64; 3]>,
    },
}

#[derive(Serialize)]
pub struct LaboratorySnapshot {
    pub tick: u64,
    pub running: bool,
    pub resources: Vec<String>,
    pub width: f64,
    pub height: f64,
    pub material_count: usize,
    pub materials: Vec<MaterialSnapshot>,
    pub chemistry_usable_energy: f64,
    pub resonant_energy: Option<f64>,
    pub organisms: usize,
    pub flow: LaboratoryFlowConfig,
    pub world_tone_enabled: bool,
    pub hotspot_enabled: bool,
    pub reaction_history: Vec<ReactionEventSnapshot>,
    pub molecular_network: MolecularNetworkSnapshot,
    pub energy: EnergyLedgerSnapshot,
}

#[derive(Serialize)]
pub struct MaterialSnapshot {
    pub identity: u64,
    pub parts: Vec<(String, f64)>,
    pub placements: Vec<[f64; 3]>,
    /// Authoritative realized shape for each constituent, serialized from the engine.
    pub shapes: Vec<Option<crate::resources::Form>>,
    pub bond_count: usize,
    pub bonds: Vec<[usize; 2]>,
}

#[derive(Clone)]
struct LabState(Arc<Mutex<Laboratory>>);

#[derive(Deserialize)]
struct PlaceRequest {
    resource: String,
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
struct MoleculeRequest {
    parts: Vec<String>,
    bonds: Vec<[usize; 2]>,
    placements: Vec<[f64; 3]>,
}

#[derive(Deserialize)]
struct EnergySourcesRequest {
    world_tone_enabled: bool,
    hotspot_enabled: bool,
}

#[derive(Deserialize)]
struct StepRequest {
    count: Option<u32>,
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn state(State(state): State<LabState>) -> Json<LaboratorySnapshot> {
    Json(state.0.lock().await.snapshot())
}

async fn export_experiment(State(state): State<LabState>) -> Json<LaboratoryDocument> {
    Json(state.0.lock().await.document())
}

async fn import_experiment(
    State(state): State<LabState>,
    Json(document): Json<LaboratoryDocument>,
) -> Result<Json<LaboratorySnapshot>, (StatusCode, String)> {
    let mut replacement = Laboratory::from_document(document)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    let mut current = state.0.lock().await;
    replacement.run_generation = current.run_generation.wrapping_add(1);
    *current = replacement;
    Ok(Json(current.snapshot()))
}

async fn reset(
    State(state): State<LabState>,
    Json(config): Json<LaboratoryConfig>,
) -> Result<Json<LaboratorySnapshot>, (StatusCode, String)> {
    let mut replacement =
        Laboratory::new(config).map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    let snapshot = replacement.snapshot();
    let mut current = state.0.lock().await;
    replacement.run_generation = current.run_generation.wrapping_add(1);
    *current = replacement;
    Ok(Json(snapshot))
}

async fn place(
    State(state): State<LabState>,
    Json(request): Json<PlaceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let identity = state
        .0
        .lock()
        .await
        .place_element(&request.resource, request.x, request.y)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(serde_json::json!({ "identity": identity })))
}

async fn molecule(
    State(state): State<LabState>,
    Json(request): Json<MoleculeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let identity = state
        .0
        .lock()
        .await
        .place_molecule(request.parts, request.bonds, request.placements)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(serde_json::json!({ "identity": identity })))
}

async fn start(State(state): State<LabState>) -> Json<LaboratorySnapshot> {
    let generation = {
        let mut laboratory = state.0.lock().await;
        if laboratory.simulation.running {
            None
        } else {
            laboratory.simulation.running = true;
            laboratory.run_generation = laboratory.run_generation.wrapping_add(1);
            Some(laboratory.run_generation)
        }
    };
    if let Some(generation) = generation {
        let shared = state.0.clone();
        tokio::spawn(async move {
            loop {
                let interval = {
                    let laboratory = shared.lock().await;
                    if !laboratory.simulation.running || laboratory.run_generation != generation {
                        break;
                    }
                    std::time::Duration::from_secs_f64(1.0 / laboratory.config.ticks_per_second)
                };
                tokio::time::sleep(interval).await;
                let mut laboratory = shared.lock().await;
                if !laboratory.simulation.running || laboratory.run_generation != generation {
                    break;
                }
                laboratory
                    .step(1)
                    .expect("one real-time laboratory tick must be valid");
            }
        });
    }
    Json(state.0.lock().await.snapshot())
}

async fn pause(State(state): State<LabState>) -> Json<LaboratorySnapshot> {
    let mut laboratory = state.0.lock().await;
    laboratory.simulation.running = false;
    laboratory.run_generation = laboratory.run_generation.wrapping_add(1);
    Json(laboratory.snapshot())
}

async fn configure_energy_sources(
    State(state): State<LabState>,
    Json(request): Json<EnergySourcesRequest>,
) -> Result<Json<LaboratorySnapshot>, (StatusCode, String)> {
    let mut laboratory = state.0.lock().await;
    laboratory
        .configure_energy_sources(request.world_tone_enabled, request.hotspot_enabled)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(laboratory.snapshot()))
}

async fn configure_flow(
    State(state): State<LabState>,
    Json(flow): Json<LaboratoryFlowConfig>,
) -> Result<Json<LaboratorySnapshot>, (StatusCode, String)> {
    let mut laboratory = state.0.lock().await;
    laboratory
        .configure_flow(flow)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(laboratory.snapshot()))
}

async fn step(
    State(state): State<LabState>,
    Json(request): Json<StepRequest>,
) -> Result<Json<LaboratorySnapshot>, (StatusCode, String)> {
    let mut laboratory = state.0.lock().await;
    laboratory
        .step(request.count.unwrap_or(1))
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(laboratory.snapshot()))
}

pub async fn run() {
    let laboratory =
        Laboratory::new(LaboratoryConfig::default()).expect("default lab config valid");
    let app_state = LabState(Arc::new(Mutex::new(laboratory)));
    let app = Router::new()
        .route("/", get(index))
        .route("/api/state", get(state))
        .route(
            "/api/experiment",
            get(export_experiment).post(import_experiment),
        )
        .route("/api/reset", post(reset))
        .route("/api/place", post(place))
        .route("/api/molecule", post(molecule))
        .route("/api/run", post(start))
        .route("/api/pause", post(pause))
        .route("/api/step", post(step))
        .route("/api/flow", post(configure_flow))
        .route("/api/sources", post(configure_energy_sources))
        .with_state(app_state);
    let address = std::env::var("EVOSIM_LAB_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:3030".into())
        .parse::<SocketAddr>()
        .expect("valid EVOSIM_LAB_ADDRESS");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("lab listener binds");
    println!("EvoSim Chemical Laboratory: http://{address}");
    axum::serve(listener, app).await.expect("lab server failed");
}

const INDEX_HTML: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>EvoSim Chemical Laboratory</title><style>
:root{color-scheme:dark;font:15px system-ui;background:#081b2a;color:#e7f3fa}body{margin:0;display:grid;grid-template-rows:auto 1fr;min-height:100vh}header{padding:12px 18px;background:#102b40;border-bottom:1px solid #28506a;display:flex;gap:18px;align-items:center;flex-wrap:wrap}main{display:grid;grid-template-columns:minmax(0,1fr) 300px;min-height:0}canvas{width:100%;height:100%;background:#0b3147;display:block;touch-action:none}aside{padding:16px;background:#102638;border-left:1px solid #28506a}button,select,input,textarea{background:#173c54;color:#e7f3fa;border:1px solid #46718a;border-radius:5px;padding:8px}#status{color:#9ec8df}#materials{font-size:13px;line-height:1.7;white-space:pre-wrap;overflow-wrap:anywhere}textarea{width:95%}@media(max-width:650px){main{grid-template-columns:1fr;grid-template-rows:minmax(55vh,1fr) auto}aside{border-left:0;border-top:1px solid #28506a}}
</style></head><body><header><strong>EvoSim · Chemical Laboratory</strong><button id="run">Run</button><button id="pause">Pause</button><button id="step">Step ×1</button><button id="step100">Step ×100</button><label>Width <input id="width" type="number" value="100" min="1" max="10000" style="width:65px"></label><label>Height <input id="height" type="number" value="100" min="1" max="10000" style="width:65px"></label><label>Seed <input id="seed" type="number" value="42" style="width:75px"></label><button id="reset">Reset world</button><button id="zoomIn">Zoom +</button><button id="zoomOut">Zoom −</button><button id="fitView">Fit world</button><button id="saveSetup">Save setup</button><label>Load setup <input id="loadSetup" type="file" accept="application/json,.json"></label><label>Element <select id="element"></select></label><span>Wheel zoom · drag to pan · tap molecules to inspect · tap empty space to place</span><span id="status">Loading…</span></header><main><canvas id="world"></canvas><aside><h3>Experiment state</h3><div id="summary"></div><h3>External energy sources</h3><label><input id="worldTone" type="checkbox"> World tone (440 Hz)</label><br><label><input id="hotspot" type="checkbox"> Local hotspot (880 Hz)</label><p>Both use the engine's existing source and energy-ledger settlement. Enabling a source does not guarantee capture; physical material and its response determine absorption.</p><h3>Energy ledger</h3><div id="energy"></div><h3>Ambient flow field</h3><label>Direction (degrees) <input id="flowDirection" type="number" min="-360" max="360" step="1" value="0" style="width:90px"><output id="flowDirectionValue">°</output></label><br><label>Speed <input id="flowSpeed" type="range" min="0" max="5" step="0.1" value="1"><output id="flowSpeedValue">1.0</output></label><br><label>Flow structure <input id="flowStrength" type="range" min="0" max="1" step="0.05" value="1"><output id="flowStrengthValue">1.00</output></label><br><label>Local vortices <input id="flowVortices" type="range" min="0" max="2" step="0.1" value="1"><output id="flowVorticesValue">1.0</output></label><p>Changes apply immediately, including during a running experiment. Strength 0 is a uniform current; strength 1 restores the S-shaped field.</p><h3>Selected molecule</h3><div id="selected">Tap a molecule to inspect it.</div><h3>Reaction history and lineage</h3><div id="reactions">No reactions recorded yet.</div><h3>Molecular network</h3><div id="network"></div><h3>Physical materials</h3><div id="materials"></div><h3>Place molecule (JSON)</h3><textarea id="molecule" rows="5">{"parts":["Carbon","Oxygen"],"bonds":[[0,1]],"placements":[[45,50,0],[47,50,0]]}</textarea><button id="placeMolecule">Place molecule</button><p>Tap the field to place the selected element. Molecules use the exact constituent coordinates in the editor above.</p></aside></main><script>
const canvas=document.querySelector('#world'),ctx=canvas.getContext('2d');let state=null,selectedIdentity=null,suppressClick=false;const camera={zoom:1,dx:0,dy:0};const palette={Carbon:'#9aa5ae',Oxygen:'#e04444',Hydrogen:'#dce8ee',Water:'#328ed0',Nitrogen:'#9b7045',Phosphorus:'#e3a34c',Sulfur:'#e2d34b'};
function resize(){const r=canvas.getBoundingClientRect(),d=devicePixelRatio||1;canvas.width=r.width*d;canvas.height=r.height*d;draw()}
function transform(){const base=Math.min(canvas.width/state.width,canvas.height/state.height),scale=base*camera.zoom;return {scale,ox:(canvas.width-state.width*base)/2+camera.dx,oy:(canvas.height-state.height*base)/2+camera.dy}}
function flowVector(x,y){const f=state.flow,phase=Math.PI*2*x/state.width,center=state.height*.5+state.height*.23*Math.sin(phase),slope=state.height*.23*Math.PI*2/state.width*Math.cos(phase),dy=((y-center+state.height*.5)%state.height+state.height)%state.height-state.height*.5,scale=Math.max(state.height*.16,1),envelope=Math.exp(-.5*(dy/scale)**2),tl=Math.hypot(1,slope),a=f.direction_degrees*Math.PI/180,ca=Math.cos(a),sa=Math.sin(a),curved={x:(ca-slope*sa)/tl*(.12+.88*envelope),y:(sa+slope*ca)/tl*(.12+.88*envelope)};let vx=ca*(1-f.strength)+curved.x*f.strength,vy=sa*(1-f.strength)+curved.y*f.strength;const radius=Math.max(state.height*.13,1);for(const [fraction,sign] of [[.25,1],[.75,-1]]){const cx=state.width*fraction,cy=state.height*.5+state.height*.23*Math.sin(Math.PI*2*fraction),dx=x-cx,ddy=((y-cy+state.height*.5)%state.height+state.height)%state.height-state.height*.5,w=Math.exp(-(dx*dx+ddy*ddy)/(2*radius*radius))*sign*1.2*f.vortex_strength*f.strength;vx+=(-ddy/radius*w)*ca-(dx/radius*w)*sa;vy+=(-ddy/radius*w)*sa+(dx/radius*w)*ca}return {x:vx*f.speed,y:vy*f.speed}}
function drawFlow(){if(!state.flow.speed)return;ctx.strokeStyle='rgba(117,200,232,.48)';ctx.fillStyle='rgba(117,200,232,.58)';ctx.lineWidth=.12;for(let y=state.height/8;y<state.height;y+=state.height/4)for(let x=state.width/10;x<state.width;x+=state.width/5){const v=flowVector(x,y),l=Math.hypot(v.x,v.y);if(l<.02)continue;const ux=v.x/l,uy=v.y/l,len=Math.min(1.2,.35+l*.22),x2=x+ux*len,y2=y+uy*len;ctx.beginPath();ctx.moveTo(x,y);ctx.lineTo(x2,y2);ctx.stroke();ctx.beginPath();ctx.moveTo(x2,y2);ctx.lineTo(x2-ux*.3-uy*.18,y2-uy*.3+ux*.18);ctx.lineTo(x2-ux*.3+uy*.18,y2-uy*.3-ux*.18);ctx.closePath();ctx.fill()}}
function drawPart(form,x,y,rotation,color,selected){
 if(!form)return;
 const kind=Object.keys(form)[0],data=form[kind]||{};
 ctx.save();ctx.translate(x,y);ctx.rotate(rotation);ctx.beginPath();
 let fill=true;
 if(kind==='Circle'){ctx.arc(0,0,data.radius,0,Math.PI*2)}
 else if(kind==='Line'){ctx.moveTo(-data.length/2,0);ctx.lineTo(data.length/2,0);fill=false}
 else if(kind==='Rectangle'){ctx.rect(-data.width/2,-data.height/2,data.width,data.height)}
 else if(kind==='RegularPolygon'){const n=Math.max(3,data.sides|0);for(let i=0;i<n;i++){const a=i*Math.PI*2/n;const px=Math.cos(a)*data.radius,py=Math.sin(a)*data.radius;i?ctx.lineTo(px,py):ctx.moveTo(px,py)}ctx.closePath()}
 else if(kind==='Polygon'){const v=data.vertices||[];v.forEach((p,i)=>i?ctx.lineTo(p[0],p[1]):ctx.moveTo(p[0],p[1]));ctx.closePath()}
 else if(kind==='Fluid'){
   const v=data.boundary,arc=data.arc_boundary,multi=data.multi_contact_boundary;
   if(v&&v.length>=3){v.forEach((p,i)=>i?ctx.lineTo(p[0],p[1]):ctx.moveTo(p[0],p[1]));ctx.closePath()}
   else if(arc){const sx=arc.center_x+arc.radius*Math.cos(arc.start_angle_radians),sy=arc.center_y+arc.radius*Math.sin(arc.start_angle_radians);ctx.moveTo(sx,sy);ctx.arc(arc.center_x,arc.center_y,arc.radius,arc.start_angle_radians,arc.start_angle_radians+arc.sweep_radians,arc.sweep_radians<0);ctx.closePath()}
   else if(multi?.pieces?.length){const pieces=multi.pieces;let started=false;for(const piece of pieces){const key=Object.keys(piece)[0],p=piece[key];if(key==='SolidContact'){if(!started){ctx.moveTo(p.start_x,p.start_y);started=true}ctx.lineTo(p.end_x,p.end_y)}else if(key==='FreeArc'){const a=p,sx=a.center_x+a.radius*Math.cos(a.start_angle_radians),sy=a.center_y+a.radius*Math.sin(a.start_angle_radians);if(!started){ctx.moveTo(sx,sy);started=true}ctx.arc(a.center_x,a.center_y,a.radius,a.start_angle_radians,a.start_angle_radians+a.sweep_radians,a.sweep_radians<0)}}ctx.closePath()}
   else {const radius=Math.sqrt(Math.max(data.nominal_area||1,.001)/Math.PI);ctx.arc(0,0,radius,0,Math.PI*2)}
 } else {ctx.restore();return}
 if(fill){ctx.fillStyle=color;ctx.fill();ctx.strokeStyle='#071521';ctx.lineWidth=.09;ctx.stroke()}
 else {ctx.strokeStyle=color;ctx.lineWidth=.16;ctx.stroke()}
 if(selected){ctx.strokeStyle='#f5d76e';ctx.lineWidth=.2;ctx.stroke()}
 ctx.restore()
}
function draw(){if(!state)return;ctx.setTransform(1,0,0,1,0,0);ctx.fillStyle='#0b3147';ctx.fillRect(0,0,canvas.width,canvas.height);const t=transform();ctx.setTransform(t.scale,0,0,t.scale,t.ox,t.oy);ctx.strokeStyle='#17465d';ctx.lineWidth=.08;for(let x=0;x<=state.width;x+=10){ctx.beginPath();ctx.moveTo(x,0);ctx.lineTo(x,state.height);ctx.stroke()}for(let y=0;y<=state.height;y+=10){ctx.beginPath();ctx.moveTo(0,y);ctx.lineTo(state.width,y);ctx.stroke()}drawFlow();for(const m of state.materials){ctx.strokeStyle='#a7d8ed';ctx.lineWidth=.12;for(const bond of m.bonds){const a=m.placements[bond[0]],b=m.placements[bond[1]];if(!a||!b)continue;ctx.beginPath();ctx.moveTo(a[0],a[1]);ctx.lineTo(b[0],b[1]);ctx.stroke()}for(let i=0;i<m.parts.length;i++){const p=m.placements[i];if(!p)continue;drawPart(m.shapes?.[i],p[0],p[1],p[2],palette[m.parts[i][0]]||'#ddd',m.identity===selectedIdentity)}if(m.identity===selectedIdentity){for(const p of m.placements){ctx.beginPath();ctx.arc(p[0],p[1],.95,0,Math.PI*2);ctx.strokeStyle='#f5d76e';ctx.lineWidth=.12;ctx.stroke()}}}ctx.setTransform(1,0,0,1,0,0)}
function syncFlowControls(){if(!state)return;const wt=document.querySelector('#worldTone'),hs=document.querySelector('#hotspot');if(document.activeElement!==wt)wt.checked=state.world_tone_enabled;if(document.activeElement!==hs)hs.checked=state.hotspot_enabled;const f=state.flow;for(const [id,key] of [['flowDirection','direction_degrees'],['flowSpeed','speed'],['flowStrength','strength'],['flowVortices','vortex_strength']]){const el=document.querySelector('#'+id);if(document.activeElement!==el)el.value=f[key]}document.querySelector('#flowDirectionValue').textContent='°';document.querySelector('#flowSpeedValue').textContent=Number(f.speed).toFixed(1);document.querySelector('#flowStrengthValue').textContent=Number(f.strength).toFixed(2);document.querySelector('#flowVorticesValue').textContent=Number(f.vortex_strength).toFixed(1)}
function syncSelection(){const m=state.materials.find(item=>item.identity===selectedIdentity);if(!m){selectedIdentity=null;document.querySelector('#selected').textContent='Tap a molecule to inspect it.';return}const lineage=state.reaction_history.flatMap(e=>e.molecular_lifecycle).filter(l=>l.molecule_id===m.identity||l.parent_ids.includes(m.identity));document.querySelector('#selected').textContent='Identity #'+m.identity+'\nConstituents: '+m.parts.map(p=>p[0]+' × '+p[1]).join(', ')+'\nBonds: '+m.bond_count+'\nCoordinates:\n'+m.placements.map((p,i)=>i+': ('+p[0].toFixed(2)+', '+p[1].toFixed(2)+'), θ '+p[2].toFixed(3)+' rad').join('\n')+'\nBond graph: '+JSON.stringify(m.bonds)+'\nLineage: '+(lineage.length?lineage.map(l=>l.action+' #'+l.molecule_id+' via '+l.reaction+' parents ['+l.parent_ids.join(',')+']').join('; '):'No lineage event recorded for this identity')}
async function refresh(){state=await(await fetch('/api/state')).json();document.querySelector('#summary').textContent='Tick: '+state.tick+' · '+(state.running?'Running':'Paused')+' · Materials: '+state.material_count+' · Organisms: '+state.organisms+' · Usable: '+state.chemistry_usable_energy.toFixed(3)+' · Resonance: '+(state.resonant_energy===null?'n/a':state.resonant_energy.toFixed(3));document.querySelector('#materials').textContent=state.materials.map(m=>'#'+m.identity+' '+m.parts.map(p=>p[0]).join('–')+' · bonds '+m.bond_count).join('\n')||'No materials placed';const energy=state.energy;document.querySelector('#energy').textContent='External input: '+energy.external_input.toFixed(3)+'\nUsable held: '+energy.usable_energy_held.toFixed(3)+'\nResonance held: '+energy.resonant_energy_held.toFixed(3)+'\nStructural held: '+energy.structural_energy_held.toFixed(3)+'\nPair potential: '+energy.pair_interaction_potential_held.toFixed(3)+'\nWork: '+energy.work_expended.toFixed(3)+'\nHeat: '+energy.heat_dissipated.toFixed(3)+'\nWorld tone unabsorbed: '+energy.world_tone_unabsorbed.toFixed(3)+'\nWorld tone escaped: '+energy.world_tone_escaped.toFixed(3);document.querySelector('#reactions').textContent=state.reaction_history.slice(-40).reverse().map(e=>'t'+e.tick+' '+e.reaction+' '+e.outcome+' · work '+e.work_required.toFixed(3)+(e.product_material?' · product '+e.product_material.map(p=>p[0]).join('–'):'')+(e.molecular_lifecycle.length?'\n'+e.molecular_lifecycle.map(l=>l.action+' #'+l.molecule_id+' parents ['+l.parent_ids.join(',')+']').join('; '):'')+(e.failure_reason?' · '+e.failure_reason:'')).join('\n')||'No reactions recorded yet';document.querySelector('#network').textContent='Molecules: '+state.molecular_network.molecule_nodes+'\nFamilies: '+state.molecular_network.family_count+'\nLineage edges: '+state.molecular_network.lineage_edges+'\nLargest family: '+state.molecular_network.largest_family_size+'\nReaction types: '+(state.molecular_network.reaction_types.join(', ')||'None')+'\nEdges: '+state.molecular_network.edges.slice(0,50).map(e=>e[0]+' → '+e[1]).join(', ')+(state.molecular_network.edges.length>50?' … +'+(state.molecular_network.edges.length-50)+' more':'');syncFlowControls();syncSelection();document.querySelector('#status').textContent='Ready';draw()}
function worldAt(clientX,clientY){const r=canvas.getBoundingClientRect(),t=transform(),px=(clientX-r.left)*(devicePixelRatio||1),py=(clientY-r.top)*(devicePixelRatio||1);return {x:(px-t.ox)/t.scale,y:(py-t.oy)/t.scale,px,py,t}}
canvas.addEventListener('click',async e=>{if(suppressClick){suppressClick=false;return}const p=worldAt(e.clientX,e.clientY);const hit=state.materials.map(m=>({m,d:Math.min(...m.placements.map(q=>Math.hypot(q[0]-p.x,q[1]-p.y)))})).filter(v=>v.d<1.2).sort((a,b)=>a.d-b.d)[0];if(hit){selectedIdentity=hit.m.identity;syncSelection();draw();return}selectedIdentity=null;const response=await fetch('/api/place',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({resource:document.querySelector('#element').value,x:p.x,y:p.y})});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}await refresh()});
let pointer=null;canvas.addEventListener('pointerdown',e=>{pointer={x:e.clientX,y:e.clientY,dx:camera.dx,dy:camera.dy,moved:false};canvas.setPointerCapture(e.pointerId)});canvas.addEventListener('pointermove',e=>{if(!pointer)return;const d=(devicePixelRatio||1);const dx=(e.clientX-pointer.x)*d,dy=(e.clientY-pointer.y)*d;if(Math.abs(dx)+Math.abs(dy)>4)pointer.moved=true;if(pointer.moved){camera.dx=pointer.dx+dx;camera.dy=pointer.dy+dy;draw()}});canvas.addEventListener('pointerup',()=>{if(pointer?.moved)suppressClick=true;pointer=null});canvas.addEventListener('pointercancel',()=>{pointer=null});
canvas.addEventListener('wheel',e=>{e.preventDefault();const before=worldAt(e.clientX,e.clientY);camera.zoom=Math.max(.5,Math.min(20,camera.zoom*Math.exp(-e.deltaY*.001)));const after=transform();camera.dx+=before.px-(before.x*after.scale+after.ox);camera.dy+=before.py-(before.y*after.scale+after.oy);draw()},{passive:false});
function zoomBy(factor){const x=canvas.width/2,y=canvas.height/2,before=transform(),worldX=(x-before.ox)/before.scale,worldY=(y-before.oy)/before.scale;camera.zoom=Math.max(.5,Math.min(20,camera.zoom*factor));const after=transform();camera.dx+=x-(worldX*after.scale+after.ox);camera.dy+=y-(worldY*after.scale+after.oy);draw()}
document.querySelector('#zoomIn').onclick=()=>zoomBy(1.25);document.querySelector('#zoomOut').onclick=()=>zoomBy(.8);document.querySelector('#fitView').onclick=()=>{camera.zoom=1;camera.dx=0;camera.dy=0;draw()};
document.querySelector('#placeMolecule').onclick=async()=>{try{const payload=JSON.parse(document.querySelector('#molecule').value);const response=await fetch('/api/molecule',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}await refresh()}catch(error){document.querySelector('#status').textContent='Invalid molecule JSON: '+error.message}};
document.querySelector('#reset').onclick=async()=>{const payload={width:Number(document.querySelector('#width').value),height:Number(document.querySelector('#height').value),cell_size:5,ticks_per_second:20,seed:Number(document.querySelector('#seed').value),flow:state.flow,world_tone_enabled:document.querySelector('#worldTone').checked,hotspot_enabled:document.querySelector('#hotspot').checked};const response=await fetch('/api/reset',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}camera.zoom=1;camera.dx=0;camera.dy=0;selectedIdentity=null;await refresh()};
document.querySelector('#saveSetup').onclick=async()=>{const response=await fetch('/api/experiment');if(!response.ok){document.querySelector('#status').textContent=await response.text();return}const data=await response.json(),blob=new Blob([JSON.stringify(data,null,2)],{type:'application/json'}),url=URL.createObjectURL(blob),link=document.createElement('a');link.href=url;link.download='evosim-experiment.json';link.click();URL.revokeObjectURL(url)};
document.querySelector('#loadSetup').addEventListener('change',async e=>{const file=e.target.files[0];if(!file)return;try{const documentData=JSON.parse(await file.text()),response=await fetch('/api/experiment',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(documentData)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}camera.zoom=1;camera.dx=0;camera.dy=0;selectedIdentity=null;state=await response.json();await refresh()}catch(error){document.querySelector('#status').textContent='Could not load setup: '+error.message}finally{e.target.value=''}});
document.querySelector('#run').onclick=async()=>{await fetch('/api/run',{method:'POST'});await refresh()};
document.querySelector('#pause').onclick=async()=>{await fetch('/api/pause',{method:'POST'});await refresh()};
document.querySelector('#step').onclick=async()=>{await fetch('/api/step',{method:'POST',headers:{'content-type':'application/json'},body:'{"count":1}'});await refresh()};
document.querySelector('#step100').onclick=async()=>{await fetch('/api/step',{method:'POST',headers:{'content-type':'application/json'},body:'{"count":100}'});await refresh()};
async function updateFlow(){const flow={direction_degrees:Number(document.querySelector('#flowDirection').value),speed:Number(document.querySelector('#flowSpeed').value),strength:Number(document.querySelector('#flowStrength').value),vortex_strength:Number(document.querySelector('#flowVortices').value)};for(const [id,key] of [['flowDirection','direction_degrees'],['flowSpeed','speed'],['flowStrength','strength'],['flowVortices','vortex_strength']])document.querySelector('#'+id+'Value').textContent=flow[key].toFixed(id==='flowStrength'?2:id==='flowDirection'?0:1)+(id==='flowDirection'?'°':'');const response=await fetch('/api/flow',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(flow)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}state=await response.json();draw()}
for(const id of ['flowDirection','flowSpeed','flowStrength','flowVortices']){const control=document.querySelector('#'+id);control.addEventListener('change',updateFlow);if(id==='flowDirection')control.addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();control.blur()}})}
async function updateSources(){const payload={world_tone_enabled:document.querySelector('#worldTone').checked,hotspot_enabled:document.querySelector('#hotspot').checked};const response=await fetch('/api/sources',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}state=await response.json();draw()}
document.querySelector('#worldTone').addEventListener('change',updateSources);document.querySelector('#hotspot').addEventListener('change',updateSources);
(async()=>{await refresh();document.querySelector('#element').innerHTML=state.resources.map(n=>'<option>'+n+'</option>').join('');new ResizeObserver(resize).observe(canvas);setInterval(()=>{if(state&&state.running)refresh()},250)})();
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laboratory_starts_empty_and_without_organisms() {
        let laboratory = Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        let snapshot = laboratory.snapshot();
        assert_eq!(snapshot.tick, 0);
        assert_eq!(snapshot.material_count, 0);
        assert_eq!(snapshot.organisms, 0);
        assert!(snapshot.materials.is_empty());
    }

    #[test]
    fn placed_elements_receive_stable_physical_identity() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        let identity = laboratory
            .place_element("Carbon", 20.0, 30.0)
            .expect("carbon placement");
        assert_ne!(identity, 0);
        let snapshot = laboratory.snapshot();
        assert_eq!(snapshot.material_count, 1);
        assert_eq!(snapshot.materials[0].identity, identity);
        assert_eq!(
            snapshot.materials[0].parts,
            vec![("Carbon".to_string(), 1.0)]
        );
    }

    #[test]
    fn molecule_placement_preserves_the_requested_physical_bond_graph() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        let identity = laboratory
            .place_molecule(
                vec!["Carbon".into(), "Hydrogen".into()],
                vec![[0, 1]],
                vec![[20.0, 20.0, 0.1], [21.0, 20.0, 0.4]],
            )
            .expect("valid carbon-hydrogen geometry");
        let snapshot = laboratory.snapshot();
        let molecule = snapshot
            .materials
            .iter()
            .find(|material| material.identity == identity)
            .expect("placed molecule is observable");
        assert_eq!(molecule.parts.len(), 2);
        assert_eq!(molecule.bonds, vec![[0, 1]]);
        assert_eq!(molecule.bond_count, 1);
        assert_eq!(molecule.placements.len(), 2);
        assert!(
            laboratory
                .simulation
                .energy_inventory_snapshot()
                .is_balanced_without_creation(1e-9),
            "pre-placed bond potential belongs to the tick-zero initial condition"
        );
    }

    #[test]
    fn laboratory_rejects_overlapping_initial_material_placement_atomically() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory
            .place_element("Carbon", 20.0, 20.0)
            .expect("first carbon placement");
        assert!(laboratory.place_element("Carbon", 20.0, 20.0).is_err());
        let snapshot = laboratory.snapshot();
        assert_eq!(snapshot.material_count, 1);
        assert_eq!(
            snapshot.materials[0].parts,
            vec![("Carbon".to_string(), 1.0)]
        );
    }

    #[test]
    fn laboratory_rejects_material_injection_after_the_experiment_starts() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory.step(1).expect("first tick");
        assert!(laboratory.place_element("Carbon", 10.0, 10.0).is_err());
    }

    #[test]
    fn laboratory_rejects_oversized_grid_allocations() {
        let config = LaboratoryConfig {
            width: 10_000.0,
            height: 10_000.0,
            cell_size: 1.0,
            ..LaboratoryConfig::default()
        };
        assert!(Laboratory::new(config).is_err());
    }

    #[test]
    fn laboratory_rejects_invalid_dimensions_and_out_of_bounds_placement() {
        let invalid = LaboratoryConfig {
            width: f64::NAN,
            ..LaboratoryConfig::default()
        };
        assert!(Laboratory::new(invalid).is_err());

        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        assert!(laboratory.place_element("Carbon", 100.0, 10.0).is_err());
        assert!(laboratory.place_element("Unobtainium", 10.0, 10.0).is_err());
    }

    #[test]
    fn laboratory_flow_can_be_changed_while_running_and_is_reflected_in_state() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory.simulation.running = true;
        let flow = LaboratoryFlowConfig {
            direction_degrees: 90.0,
            speed: 2.5,
            strength: 0.0,
            vortex_strength: 0.0,
        };
        laboratory
            .configure_flow(flow)
            .expect("valid live flow change");
        assert_eq!(laboratory.snapshot().flow, flow);
        assert_eq!(laboratory.simulation.flow_field, flow.engine_config());
        assert!(laboratory.simulation.running);
    }

    #[test]
    fn laboratory_observation_captures_reaction_and_molecular_lineage_events() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory.simulation.environmental_chemistry_events.push(
            crate::state::EnvironmentalChemistryEvent {
                reaction_id: "7-0".into(),
                tick: 7,
                reaction: "COMBINE".into(),
                source_cells: vec![2],
                source_material_indices: vec![0, 1],
                source_material_ids: vec![Some(11), Some(12)],
                source_materials: vec![vec![("Carbon".into(), 1.0)], vec![("Carbon".into(), 1.0)]],
                work_required: 0.5,
                reaction_frequency_hz: None,
                activation_signal: None,
                accumulated_reaction: Some(1.0),
                broken_bond_index: None,
                outcome: "committed".into(),
                failure_reason: None,
                energy_settlement: crate::state::EnvironmentalEnergySettlement::default(),
                product_material: Some(vec![("Carbon".into(), 2.0)]),
                product_internal_bond_count: Some(1),
                post_commit_source_cell_materials: Vec::new(),
                molecular_lifecycle: vec![crate::state::EnvironmentalMoleculeLifecycleEvent {
                    event: "molecular_lifecycle".into(),
                    action: "created".into(),
                    tick: 7,
                    molecule_id: 42,
                    parent_ids: vec![11, 12],
                    reaction_id: "7-0".into(),
                    reaction: "COMBINE".into(),
                    molecule: None,
                }],
            },
        );
        let event = &laboratory.simulation.environmental_chemistry_events[0];
        assert_eq!(event.reaction_id, "7-0");
        assert_eq!(event.source_material_ids, vec![Some(11), Some(12)]);
        assert_eq!(event.molecular_lifecycle[0].reaction_id, event.reaction_id);
        laboratory.capture_current_tick_events();
        let history = laboratory.snapshot().reaction_history;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].tick, 7);
        assert_eq!(history[0].reaction, "COMBINE");
        assert_eq!(history[0].outcome, "committed");
        assert_eq!(history[0].molecular_lifecycle[0].molecule_id, 42);
        assert_eq!(history[0].molecular_lifecycle[0].parent_ids, vec![11, 12]);
        let network = laboratory.snapshot().molecular_network;
        assert_eq!(network.molecule_nodes, 3);
        assert_eq!(network.lineage_edges, 2);
        assert_eq!(network.family_count, 1);
        assert_eq!(network.largest_family_size, 3);
    }

    #[test]
    fn experiment_document_round_trips_config_and_initial_molecular_geometry() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory
            .place_element("Carbon", 15.0, 18.0)
            .expect("place carbon");
        laboratory
            .place_molecule(
                vec!["Carbon".into(), "Hydrogen".into()],
                vec![[0, 1]],
                vec![[35.0, 40.0, 0.1], [36.0, 40.0, 0.4]],
            )
            .expect("place molecule");
        let flow = LaboratoryFlowConfig {
            direction_degrees: 45.0,
            speed: 1.5,
            strength: 0.4,
            vortex_strength: 0.8,
        };
        laboratory.configure_flow(flow).expect("configure flow");
        laboratory
            .configure_energy_sources(true, false)
            .expect("enable world tone");
        let document = laboratory.document();
        let restored = Laboratory::from_document(document).expect("restore setup");
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.material_count, 2);
        assert_eq!(snapshot.flow, flow);
        assert!(snapshot.world_tone_enabled);
        assert!(!snapshot.hotspot_enabled);
        assert_eq!(
            snapshot
                .materials
                .iter()
                .map(|m| m.bond_count)
                .sum::<usize>(),
            1
        );
    }

    #[test]
    fn laboratory_energy_sources_use_the_existing_world_and_hotspot_models() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        assert!(!laboratory.snapshot().world_tone_enabled);
        assert!(!laboratory.snapshot().hotspot_enabled);
        laboratory
            .place_element("Carbon", 72.0, 27.5)
            .expect("place source-exposed carbon");
        laboratory
            .configure_energy_sources(true, true)
            .expect("enable existing sources");
        assert!(laboratory.snapshot().world_tone_enabled);
        assert!(laboratory.snapshot().hotspot_enabled);
        assert!(laboratory.simulation.world_tone.is_some());
        assert!(laboratory.simulation.world_tone_cycle.is_some());
        assert!(laboratory.simulation.hotspot.is_some());
        laboratory.step(1).expect("settle one tick of source input");
        assert!(
            laboratory
                .simulation
                .energy_inventory_snapshot()
                .is_balanced_without_creation(1e-8),
            "external source capture must reconcile through the existing ledger"
        );
    }

    #[test]
    fn laboratory_can_disable_energy_sources_during_a_live_run() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory
            .configure_energy_sources(true, true)
            .expect("enable sources");
        laboratory.simulation.running = true;
        laboratory
            .configure_energy_sources(false, false)
            .expect("disable sources while running");
        assert!(laboratory.simulation.world_tone.is_none());
        assert!(laboratory.simulation.world_tone_cycle.is_none());
        assert!(laboratory.simulation.hotspot.is_none());
        assert!(laboratory.simulation.running);
    }

    #[test]
    fn laboratory_rejects_invalid_flow_without_changing_the_current_field() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        let original = laboratory.config.flow;
        let invalid = LaboratoryFlowConfig {
            speed: f64::NAN,
            ..original
        };
        assert!(laboratory.configure_flow(invalid).is_err());
        assert_eq!(laboratory.config.flow, original);
    }

    #[test]
    fn laboratory_steps_the_shared_simulation_without_creating_an_organism() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory.step(3).expect("three physics steps");
        let snapshot = laboratory.snapshot();
        assert_eq!(snapshot.tick, 3);
        assert_eq!(snapshot.organisms, 0);
    }
}
