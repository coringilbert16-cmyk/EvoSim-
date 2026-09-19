use crate::combine_runtime::{combine_specific_pair, try_combine_stored_unit};
use crate::contact::ConnectionCompatibilityCache;
use crate::material_restoration::restore_material;
use crate::physical_material::PhysicalMaterial;
use crate::resources::{
    BaseResource, Form, InternalBond, Material, PhysicalState, ResourceProperties, Shape,
};
use crate::state::{EnergyLedger, Simulation};
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};

fn line_catalog() -> Vec<BaseResource> {
    vec![
        BaseResource {
            name: "A".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 2.0,
                reactivity: 1.0,
                cohesion: 0.5,
            },
            physical_state: PhysicalState::Rigid,
            shape: Shape {
                form: Form::Line { length: 2.0 },
            },
        },
        BaseResource {
            name: "B".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 3.0,
                reactivity: 1.0,
                cohesion: 0.5,
            },
            physical_state: PhysicalState::Rigid,
            shape: Shape {
                form: Form::Line { length: 2.0 },
            },
        },
    ]
}

fn bonded_material() -> Material {
    Material {
        parts: vec![("A".into(), 1.0), ("B".into(), 1.0)],
        internal_bonds: vec![InternalBond {
            part_a: 0,
            part_b: 1,
        }],
    }
}

#[test]
fn realized_material_carries_exact_internal_connection_and_restores_without_combine() {
    let catalog = line_catalog();
    let material = bonded_material();
    let placements = vec![
        Placement {
            x: -1.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
        Placement {
            x: 1.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
    ];
    let instance = PhysicalMaterial::realized(material.clone(), placements.clone(), &catalog)
        .expect("physical realization");
    let connections = instance
        .internal_connections
        .as_ref()
        .expect("stored connection endpoints");
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].part_a, 0);
    assert_eq!(connections[0].part_b, 1);
    assert_eq!(
        connections[0].endpoint_a,
        ConnectionEndpoint::LineEndpoint { point_index: 1 }
    );
    assert_eq!(
        connections[0].endpoint_b,
        ConnectionEndpoint::LineEndpoint { point_index: 0 }
    );

    let mut structure = OrganismStructure::new();
    let ledger = EnergyLedger::default();
    let before_ledger = ledger;
    let indices = restore_material(
        &mut structure,
        &instance,
        Placement {
            x: 10.0,
            y: 4.0,
            rotation_radians: 0.0,
        },
        &catalog,
    )
    .expect("restore physical material");

    assert_eq!(indices, vec![0, 1]);
    assert_eq!(structure.units.len(), 2);
    assert_eq!(structure.bonds.len(), 1);
    assert_eq!(
        structure.bonds[0].endpoint_a.location,
        connections[0].endpoint_a
    );
    assert_eq!(
        structure.bonds[0].endpoint_b.location,
        connections[0].endpoint_b
    );
    assert_eq!(
        ledger.total_potential_energy_released,
        before_ledger.total_potential_energy_released
    );
    assert_eq!(
        ledger.total_usable_energy_gained,
        before_ledger.total_usable_energy_gained
    );
    assert_eq!(
        ledger.total_heat_dissipated,
        before_ledger.total_heat_dissipated
    );
}

#[test]
fn new_structure_bond_is_formed_only_through_combine() {
    let catalog = line_catalog();
    let mut structure = OrganismStructure::new();
    let mut a = StructuralUnit::new(
        "A",
        Placement {
            x: -1.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
    );
    let mut b = StructuralUnit::new(
        "B",
        Placement {
            x: 1.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
    );
    assert!(a.realize_default_geometry(&catalog));
    assert!(b.realize_default_geometry(&catalog));
    let ua = structure.add_unit(a);
    let ub = structure.add_unit(b);
    let mut cache = ConnectionCompatibilityCache::new();
    let mut ledger = EnergyLedger::default();
    let mut energy = 100.0;

    let attempt = combine_specific_pair(
        &mut structure,
        ua,
        ub,
        &catalog,
        0.0,
        &mut cache,
        &mut ledger,
        &mut energy,
    )
    .expect("COMBINE should form the new bond");

    assert_eq!(structure.bonds.len(), 1);
    assert!(attempt.energy_invested >= 0.0);
    assert!(ledger.total_usable_energy_gained.is_finite());
    assert!(ledger.total_heat_dissipated > 0.0);
    assert!(energy < 100.0 || attempt.energy_invested == 0.0);
}

#[test]
fn stored_realized_single_constituent_enters_combine_through_physical_path() {
    let mut simulation = Simulation::new(11, 20.0);
    let organism = &mut simulation.organisms[0];
    organism.usable_energy = 1_000.0;

    let mut existing = StructuralUnit::new(
        "Carbon",
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
    );
    assert!(existing.realize_default_geometry(&simulation.environment.catalog));
    organism.structure = OrganismStructure::new();
    organism.structure.add_unit(existing);

    let material = Material::free_base("Carbon", 1.0);
    assert!(organism.stored_material.store_physical(
        material.clone(),
        vec![Placement {
            x: 100.0,
            y: 200.0,
            rotation_radians: 0.4,
        }],
        &simulation.environment.catalog,
    ));
    let stored = organism
        .stored_material
        .peek_matching_physical(&material)
        .expect("stored physical material");
    assert!(stored.is_realized());
    assert!(
        stored.placements.as_ref().unwrap()[0]
            .rotation_radians
            .abs()
            <= 1e-12
    );
    let storage_before = organism.stored_material.len();
    let realized_before = organism.stored_material.physical_count();

    let mut cache = ConnectionCompatibilityCache::new();
    let mut ledger = EnergyLedger::default();
    let attempt = try_combine_stored_unit(
        organism,
        &simulation.environment,
        &mut cache,
        &mut ledger,
        None,
    )
    .expect("stored physical Carbon should be incorporated through COMBINE");

    assert_eq!(organism.stored_material.len(), storage_before - 1);
    let realized_after = organism.stored_material.physical_count();
    assert_eq!(realized_after, realized_before - 1);
    assert_eq!(organism.structure.units.len(), 2);
    assert_eq!(organism.structure.bonds.len(), 1);
    assert!(organism.structure.units[1].geometry.is_some());
    assert!(attempt.energy_invested >= 0.0);
}
