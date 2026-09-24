use crate::resources::Material;

/// Split a physically realized material at constituent boundaries.
///
/// This partitions existing physical constituents and preserves only internal
/// bonds whose two endpoints remain in the same partition. It is the primitive
/// needed when a composite formation crosses a physical ownership boundary.
pub(crate) fn split_physical_material(
    instance: &crate::physical_material::PhysicalMaterial,
    selected: &[usize],
) -> Option<(
    crate::physical_material::PhysicalMaterial,
    crate::physical_material::PhysicalMaterial,
)> {
    let placements = instance.placements.as_ref()?;
    if !instance.is_realized() || placements.len() != instance.material.parts.len() {
        return None;
    }
    let mut flags = vec![false; instance.material.parts.len()];
    for &index in selected {
        if index >= flags.len() || flags[index] {
            return None;
        }
        flags[index] = true;
    }
    if selected.is_empty() || selected.len() == flags.len() {
        return None;
    }

    fn build(
        instance: &crate::physical_material::PhysicalMaterial,
        flags: &[bool],
        selected_side: bool,
    ) -> Option<crate::physical_material::PhysicalMaterial> {
        let mut remap = vec![usize::MAX; flags.len()];
        let mut parts = Vec::new();
        let mut placements = Vec::new();
        for (old, ((name, amount), placement)) in instance
            .material
            .parts
            .iter()
            .zip(instance.placements.as_ref()?.iter())
            .enumerate()
        {
            if flags[old] != selected_side {
                continue;
            }
            remap[old] = parts.len();
            parts.push((name.clone(), *amount));
            placements.push(*placement);
        }
        if parts.is_empty() {
            return None;
        }
        let connections = instance.internal_connections.as_ref()?;
        if connections.len() != instance.material.internal_bonds.len() {
            return None;
        }
        let mut bonds = Vec::new();
        let mut physical = Vec::new();
        for (bond, connection) in instance
            .material
            .internal_bonds
            .iter()
            .zip(connections.iter())
        {
            if flags[bond.part_a] != selected_side || flags[bond.part_b] != selected_side {
                continue;
            }
            bonds.push(crate::resources::InternalBond {
                part_a: remap[bond.part_a],
                part_b: remap[bond.part_b],
            });
            physical.push(crate::physical_material::PhysicalMaterialBond {
                part_a: remap[connection.part_a],
                endpoint_a: connection.endpoint_a,
                part_b: remap[connection.part_b],
                endpoint_b: connection.endpoint_b,
            });
        }
        Some(crate::physical_material::PhysicalMaterial {
            material: crate::resources::Material {
                parts,
                internal_bonds: bonds,
            },
            placements: Some(placements),
            internal_connections: Some(physical),
            owner_relative_origin: instance.owner_relative_origin,
        })
    }
    Some((
        build(instance, &flags, true)?,
        build(instance, &flags, false)?,
    ))
}
pub(crate) fn break_physical_material_bond(
    instance: &crate::physical_material::PhysicalMaterial,
    bond_index: usize,
) -> Option<Vec<crate::physical_material::PhysicalMaterial>> {
    let placements = instance.placements.as_ref()?;
    let connections = instance.internal_connections.as_ref()?;
    if !instance.is_realized()
        || placements.len() != instance.material.parts.len()
        || connections.len() != instance.material.internal_bonds.len()
    {
        return None;
    }
    let bonds = &instance.material.internal_bonds;
    let target = *bonds.get(bond_index)?;
    if target.part_a >= placements.len() || target.part_b >= placements.len() {
        return None;
    }

    let mut adjacency = vec![Vec::<usize>::new(); placements.len()];
    for (index, bond) in bonds.iter().enumerate() {
        if index == bond_index {
            continue;
        }
        adjacency[bond.part_a].push(bond.part_b);
        adjacency[bond.part_b].push(bond.part_a);
    }

    let mut component = vec![usize::MAX; placements.len()];
    let mut components = Vec::<Vec<usize>>::new();
    for start in 0..placements.len() {
        if component[start] != usize::MAX {
            continue;
        }
        let component_index = components.len();
        let mut stack = vec![start];
        component[start] = component_index;
        let mut members = Vec::new();
        while let Some(current) = stack.pop() {
            members.push(current);
            for &next in &adjacency[current] {
                if component[next] == usize::MAX {
                    component[next] = component_index;
                    stack.push(next);
                }
            }
        }
        components.push(members);
    }

    components
        .into_iter()
        .map(|members| {
            let mut remap = vec![usize::MAX; placements.len()];
            let mut parts = Vec::with_capacity(members.len());
            let mut new_placements = Vec::with_capacity(members.len());
            for old in members {
                remap[old] = parts.len();
                parts.push(instance.material.parts[old].clone());
                new_placements.push(placements[old]);
            }
            let mut new_bonds = Vec::new();
            let mut new_connections = Vec::new();
            for (index, bond) in bonds.iter().enumerate() {
                if index == bond_index || remap[bond.part_a] == usize::MAX || remap[bond.part_b] == usize::MAX {
                    continue;
                }
                new_bonds.push(crate::resources::InternalBond {
                    part_a: remap[bond.part_a],
                    part_b: remap[bond.part_b],
                });
                let connection = connections.get(index)?;
                new_connections.push(crate::physical_material::PhysicalMaterialBond {
                    part_a: remap[connection.part_a],
                    endpoint_a: connection.endpoint_a,
                    part_b: remap[connection.part_b],
                    endpoint_b: connection.endpoint_b,
                });
            }
            Some(crate::physical_material::PhysicalMaterial {
                material: crate::resources::Material {
                    parts,
                    internal_bonds: new_bonds,
                },
                placements: Some(new_placements),
                internal_connections: Some(new_connections),
                owner_relative_origin: instance.owner_relative_origin,
            })
        })
        .collect()
}

/// Extract up to `requested` whole unstructured units from an ecological aggregate.
///
/// This is deliberately separate from `Material::take`: the latter is a
/// legacy aggregate operation that can represent fractional material and must
/// not be used for organism-facing physical transfer. Structured material is
/// never fractionally extracted here.
pub(crate) fn take_whole_unstructured(
    material: &mut Material,
    requested: usize,
) -> Option<Material> {
    if requested == 0 || material.has_internal_structure() || material.is_empty() {
        return None;
    }

    let available = material.total_amount();
    if !available.is_finite() || available < 1.0 {
        return None;
    }

    let whole_available = available.floor() as usize;
    let target_units = requested.min(whole_available);
    if target_units == 0 {
        return None;
    }

    let total = available;
    let target = target_units as f64;
    let mut taken_parts = Vec::new();
    let mut remaining = target_units;

    // Preserve aggregate composition as closely as possible while moving only
    // whole units. Because the environment may still use aggregate f64 stock,
    // the final remainder is allocated by largest fractional remainder.
    let mut allocations: Vec<(usize, f64, usize)> = Vec::with_capacity(material.parts.len());
    for (index, (_, amount)) in material.parts.iter().enumerate() {
        if *amount <= 0.0 {
            allocations.push((index, 0.0, 0));
            continue;
        }
        let ideal = (*amount / total) * target;
        let base = ideal.floor() as usize;
        allocations.push((index, ideal - base as f64, base));
        remaining = remaining.saturating_sub(base);
    }

    allocations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (index, _, base) in &mut allocations {
        if remaining == 0 {
            break;
        }
        if *base < material.parts[*index].1.floor() as usize {
            *base += 1;
            remaining -= 1;
        }
    }

    if remaining != 0 {
        return None;
    }

    allocations.sort_by_key(|(index, _, _)| *index);
    for (index, _, count) in allocations {
        if count == 0 {
            continue;
        }
        let amount = count as f64;
        material.parts[index].1 -= amount;
        taken_parts.push((material.parts[index].0.clone(), amount));
    }

    material.parts.retain(|(_, amount)| *amount > 1e-12);
    Some(Material {
        parts: taken_parts,
        internal_bonds: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::take_whole_unstructured;
    use crate::resources::{InternalBond, Material};

    #[test]
    fn takes_only_whole_units() {
        let mut material = Material::free_base("Carbon", 10.0);
        let taken = take_whole_unstructured(&mut material, 3).unwrap();
        assert_eq!(taken.total_amount(), 3.0);
        assert_eq!(material.total_amount(), 7.0);
    }

    #[test]
    fn takes_only_available_whole_units_when_request_exceeds_stock() {
        let mut material = Material::free_base("Carbon", 3.5);
        let taken = take_whole_unstructured(&mut material, 5).unwrap();
        assert_eq!(taken.total_amount(), 3.0);
        assert_eq!(material.total_amount(), 0.5);
    }

    #[test]
    fn transfers_whole_units_from_fractional_aggregate() {
        let mut material = Material::free_base("Carbon", 3.5);
        let taken = take_whole_unstructured(&mut material, 1).unwrap();
        assert_eq!(taken.total_amount(), 1.0);
        assert_eq!(material.total_amount(), 2.5);
    }

    #[test]
    fn refuses_structured_material() {
        let mut material = Material {
            parts: vec![("Carbon".to_string(), 1.0), ("Nitrogen".to_string(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        assert!(take_whole_unstructured(&mut material, 1).is_none());
        assert_eq!(material.total_amount(), 2.0);
    }
    #[test]
    fn splits_composite_at_constituent_boundary() {
        let catalog = crate::resources::default_catalog();
        let material = Material {
            parts: vec![
                ("Carbon".into(), 1.0),
                ("Hydrogen".into(), 1.0),
                ("Nitrogen".into(), 1.0),
            ],
            internal_bonds: vec![
                InternalBond {
                    part_a: 0,
                    part_b: 1,
                },
                InternalBond {
                    part_a: 1,
                    part_b: 2,
                },
            ],
        };
        let placements = vec![
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            crate::structure::Placement {
                x: 0.8,
                y: 0.0,
                rotation_radians: 0.0,
            },
            crate::structure::Placement {
                x: 1.6,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ];
        let physical =
            crate::physical_material::PhysicalMaterial::realized(material, placements, &catalog)
                .unwrap();
        let (inside, outside) = super::split_physical_material(&physical, &[0, 1]).unwrap();
        assert_eq!(inside.material.parts.len(), 2);
        assert_eq!(
            inside.material.internal_bonds,
            vec![InternalBond {
                part_a: 0,
                part_b: 1
            }]
        );
        assert_eq!(outside.material.parts, vec![("Nitrogen".into(), 1.0)]);
        assert!(outside.material.internal_bonds.is_empty());
        assert!(inside.is_realized());
        assert!(outside.is_realized());
    }

    #[test]
    fn rejects_whole_material_as_a_partition() {
        let catalog = crate::resources::default_catalog();
        let physical = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        assert!(super::split_physical_material(&physical, &[0]).is_none());
    }
}
