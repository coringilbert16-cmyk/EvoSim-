use crate::combine_runtime::combine_specific_pair;
use crate::contact::ConnectionCompatibilityCache;
use crate::material_restoration::restore_material;
use crate::physical_material::PhysicalMaterial;
use crate::resources::{
    BaseResource, Form, InternalBond, Material, PhysicalState, ResourceProperties, Shape,
};
use crate::state::EnergyLedger;
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
fn translated_realized_material_preserves_internal_endpoint_identity() {
    let catalog = line_catalog();
    let instance = PhysicalMaterial::realized(
        bonded_material(),
        vec![
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
        ],
        &catalog,
    )
    .unwrap();
    let translated = instance
        .translated(Placement {
            x: 7.0,
            y: -3.0,
            rotation_radians: 0.5,
        })
        .unwrap();
    assert_eq!(
        translated.internal_connections,
        instance.internal_connections
    );
    assert_eq!(translated.material, instance.material);
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
