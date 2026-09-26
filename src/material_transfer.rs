use crate::resources::Material;

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
}
