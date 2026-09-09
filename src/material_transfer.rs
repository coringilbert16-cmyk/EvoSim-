use crate::resources::Material;

/// Extract whole units only from unstructured ecological composition.
/// Structured material is a physical graph and can only be separated by an
/// explicit physical BREAK/decomposition operation.
pub(crate) fn take_whole_unstructured(material: &mut Material, requested: usize) -> Option<Material> {
    if requested == 0 || material.is_structured() || material.is_empty() { return None; }
    let available = material.total_amount();
    if !available.is_finite() || available < 1.0 { return None; }
    let target_units = requested.min(available.floor() as usize);
    if target_units == 0 { return None; }
    let total = available;
    let target = target_units as f64;
    let mut allocations: Vec<(usize, f64, usize)> = Vec::with_capacity(material.composition().len());
    let mut remaining = target_units;
    for (index, component) in material.composition().iter().enumerate() {
        if component.amount <= 0.0 { allocations.push((index, 0.0, 0)); continue; }
        let ideal = component.amount / total * target;
        let base = ideal.floor() as usize;
        allocations.push((index, ideal - base as f64, base));
        remaining = remaining.saturating_sub(base);
    }
    allocations.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (index, _, base) in &mut allocations {
        if remaining == 0 { break; }
        if *base < material.composition()[*index].amount.floor() as usize { *base += 1; remaining -= 1; }
    }
    if remaining != 0 { return None; }
    allocations.sort_by_key(|(index,_,_)| *index);
    let mut taken = Vec::new();
    for (index, _, count) in allocations {
        if count == 0 { continue; }
        material.composition[index].amount -= count as f64;
        taken.push(crate::resources::MaterialComponent { resource: material.composition[index].resource.clone(), amount: count as f64 });
    }
    material.composition.retain(|c| c.amount > 1e-12);
    Some(Material { composition: taken, structure: None })
}

#[cfg(test)]
mod tests {
    use super::take_whole_unstructured;
    use crate::attachment::{AttachmentFeature, ConstituentAttachment, ConstituentId};
    use crate::material_structure::{InternalAttachmentBond, MaterialConstituent, MaterialStructure};
    use crate::resources::Material;
    #[test] fn takes_only_whole_units(){let mut m=Material::free_base("Carbon",10.0);let t=take_whole_unstructured(&mut m,3).unwrap();assert_eq!(t.total_amount(),3.0);assert_eq!(m.total_amount(),7.0)}
    #[test] fn takes_only_available_whole_units(){let mut m=Material::free_base("Carbon",3.5);let t=take_whole_unstructured(&mut m,5).unwrap();assert_eq!(t.total_amount(),3.0);assert_eq!(m.total_amount(),0.5)}
    #[test] fn transfers_whole_units_from_fractional_aggregate(){let mut m=Material::free_base("Carbon",3.5);let t=take_whole_unstructured(&mut m,1).unwrap();assert_eq!(t.total_amount(),1.0);assert_eq!(m.total_amount(),2.5)}
    #[test] fn refuses_structured_material(){let mut m=Material{composition:Vec::new(),structure:Some(MaterialStructure{constituents:vec![MaterialConstituent{id:ConstituentId(1),resource:"Carbon".into()},MaterialConstituent{id:ConstituentId(2),resource:"Nitrogen".into()}],internal_bonds:vec![InternalAttachmentBond{a:ConstituentAttachment{constituent:ConstituentId(1),feature:AttachmentFeature::Discrete(0)},b:ConstituentAttachment{constituent:ConstituentId(2),feature:AttachmentFeature::Discrete(0)}}]})};assert!(take_whole_unstructured(&mut m,1).is_none());assert_eq!(m.total_amount(),2.0)}
}
