//! Physical reproduction lifecycle.
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;
use crate::material_storage::MaterialStorage;
use crate::resources::{BaseResource,Material};
use crate::state::{DevelopmentStage,Organism,ReproductiveConstruction,ResourceSense};
use crate::structure::OrganismStructure;

const CORE_UNIT_COUNT: usize = 6;
const JUVENILE_MATURE_MASS_FRACTION: f64 = 0.40;

fn assemble_blueprint_material(remaining:&mut MaterialStorage,target:&Material)->Option<Material>{
    let mut inputs=Vec::with_capacity(target.parts.len());
    for(name,amount)in &target.parts{
        if(*amount-1.0).abs()>f64::EPSILON{return None}
        inputs.push(remaining.take_one_unstructured_named(name)?);
    }
    let assembled=if inputs.len()==1{inputs.into_iter().next()?}else{crate::resources::combine_materials(&inputs)};
    if assembled==*target{Some(assembled)}else{None}
}

fn blueprint_order(blueprint:&crate::structural_blueprint::StructuralBlueprint,catalog:&[BaseResource])->Option<Vec<usize>>{
    if !blueprint.is_valid()||blueprint.elements.len()<CORE_UNIT_COUNT{return None}
    let cx=blueprint.elements.iter().map(|e|e.placement.x).sum::<f64>()/blueprint.elements.len()as f64;
    let cy=blueprint.elements.iter().map(|e|e.placement.y).sum::<f64>()/blueprint.elements.len()as f64;
    let mut by_center=(0..blueprint.elements.len()).collect::<Vec<_>>();
    by_center.sort_by(|&a,&b|{
        let da=(blueprint.elements[a].placement.x-cx).powi(2)+(blueprint.elements[a].placement.y-cy).powi(2);
        let db=(blueprint.elements[b].placement.x-cx).powi(2)+(blueprint.elements[b].placement.y-cy).powi(2);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(&b))
    });
    let mut order=by_center[..CORE_UNIT_COUNT].to_vec();
    let mature_mass=blueprint.structural_mass(catalog);
    let target_mass=mature_mass*JUVENILE_MATURE_MASS_FRACTION;
    let mut selected=order.iter().copied().collect::<HashSet<_>>();
    let mut current_mass=order.iter().map(|&i|blueprint.elements[i].material.mass(catalog)).sum::<f64>();
    while current_mass+f64::EPSILON<target_mass{
        let mut frontier=blueprint.elements.iter().enumerate().filter(|(index,_)|!selected.contains(index)&&blueprint.connections.iter().any(|c|(c.element_a==*index&&selected.contains(&c.element_b))||(c.element_b==*index&&selected.contains(&c.element_a)))).map(|(index,_)|index).collect::<Vec<_>>();
        if frontier.is_empty(){return None}
        frontier.sort_by(|&a,&b|{
            let da=(blueprint.elements[a].placement.x-cx).powi(2)+(blueprint.elements[a].placement.y-cy).powi(2);
            let db=(blueprint.elements[b].placement.x-cx).powi(2)+(blueprint.elements[b].placement.y-cy).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(&b))
        });
        let next=frontier[0];
        selected.insert(next);
        order.push(next);
        current_mass+=blueprint.elements[next].material.mass(catalog);
    }
    Some(order)
}

fn add_blueprint_element(structure:&mut OrganismStructure,realized:&[usize],blueprint_index:usize,material:Material,blueprint:&crate::structural_blueprint::StructuralBlueprint,catalog:&[BaseResource])->Option<(usize,f64)>{
    let element=&blueprint.elements[blueprint_index];
    let unit=crate::structure::StructuralUnit::from_material(material,element.placement)?;
    let mut candidate=structure.clone();
    let new_index=candidate.add_unit(unit);
    let mut added_stress=0.0;
    for connection in &blueprint.connections{
        let other_blueprint_index=if connection.element_a==blueprint_index{Some(connection.element_b)}else if connection.element_b==blueprint_index{Some(connection.element_a)}else{None};
        let Some(other_blueprint_index)=other_blueprint_index else{continue};
        let Some(other_structure_index)=realized.iter().position(|&index|index==other_blueprint_index)else{continue};
        let(new_point,other_point)=if connection.element_a==blueprint_index{(connection.point_a,connection.point_b)}else{(connection.point_b,connection.point_a)};
        let a=candidate.connection_site(crate::structure::ConnectionSiteRef{unit_index:new_index,point_index:new_point},catalog)?;
        let b=candidate.connection_site(crate::structure::ConnectionSiteRef{unit_index:other_structure_index,point_index:other_point},catalog)?;
        if !crate::contact::connection_points_contact(a,&candidate.units[new_index],b,&candidate.units[other_structure_index],1e-9,1.0-1e-9){return None}
        let pa=candidate.units[new_index].properties(catalog)?;
        let pb=candidate.units[other_structure_index].properties(catalog)?;
        let strength=crate::combine::bond_strength(pa,pb);
        if !strength.is_finite()||!(0.0..=1.0).contains(&strength){return None}
        candidate.add_bond(crate::structure::Bond{unit_a:new_index,point_a:new_point,unit_b:other_structure_index,point_b:other_point,strength,bond_energy:0.0});
        added_stress+=strength;
    }
    *structure=candidate;
    Some((new_index,added_stress))
}

pub(crate)fn begin_reproduction(parent:&mut Organism,rng:&mut ChaCha8Rng,catalog:&[BaseResource])->bool{
    if !matches!(parent.development_stage,DevelopmentStage::Adult)||parent.reproductive_readiness<1.0-f64::EPSILON||parent.reproductive_construction.is_some(){return false}
    let mut child_genome=parent.genome.clone();
    child_genome.mutate(rng);
    let blueprint=&child_genome.structural_blueprint;
    let Some(order)=blueprint_order(blueprint,catalog)else{return false};
    let core=&order[..CORE_UNIT_COUNT];
    let mut remaining=parent.stored_material.clone();
    let mut structure=OrganismStructure::new();
    let mut realized=Vec::with_capacity(order.len());
    let mut initial_stress=0.0;
    for &blueprint_index in core{
        let Some(material)=assemble_blueprint_material(&mut remaining,&blueprint.elements[blueprint_index].material)else{return false};
        let Some((_,stress))=add_blueprint_element(&mut structure,&realized,blueprint_index,material,blueprint,catalog)else{return false};
        realized.push(blueprint_index);
        initial_stress+=stress;
    }
    parent.stored_material=remaining;
    parent.reproductive_readiness=0.0;
    parent.add_transaction_stress(initial_stress);
    parent.reproductive_construction=Some(ReproductiveConstruction{committed_material:MaterialStorage::default(),developing_structure:structure,child_genome,target_elements:order,realized_elements:realized,pending_stress:0.0});
    true
}

pub(crate)fn advance_construction(stored_material:&mut MaterialStorage,construction:&mut ReproductiveConstruction,catalog:&[BaseResource])->Option<f64>{
    if construction.realized_elements.len()>=construction.target_elements.len(){return None}
    let Some(&blueprint_index)=construction.target_elements.get(construction.realized_elements.len())else{return None};
    let target=&construction.child_genome.structural_blueprint.elements[blueprint_index].material;
    let mut remaining=stored_material.clone();
    let Some(material)=assemble_blueprint_material(&mut remaining,target)else{return None};
    let Some((_,stress))=add_blueprint_element(&mut construction.developing_structure,&construction.realized_elements,blueprint_index,material,&construction.child_genome.structural_blueprint,catalog)else{return None};
    *stored_material=remaining;
    construction.realized_elements.push(blueprint_index);
    Some(stress)
}

pub(crate)fn finish_reproduction(parent:&mut Organism,child_id:String)->Option<Organism>{
    let construction=parent.reproductive_construction.take()?;
    if construction.realized_elements.len()!=construction.target_elements.len()||!construction.committed_material.is_empty()==false{parent.reproductive_construction=Some(construction);return None}
    let position=match parent.occupied_cells.first().cloned(){Some(p)=>p,None=>{parent.reproductive_construction=Some(construction);return None}};
    Some(Organism{id:child_id,occupied_cells:vec![position],genome:construction.child_genome,resource_sense:ResourceSense{sensed_resources:Vec::new(),direction_x:0.0,direction_y:0.0,direction_strength:0.0},memory:Vec::new(),decision_history:crate::decision::DecisionHistory::default(),usable_energy:0.0,stress:0.0,stress_threshold:crate::state::INITIAL_STRESS_THRESHOLD,stored_material:MaterialStorage::default(),structure:construction.developing_structure,development_stage:DevelopmentStage::Juvenile,age:0,reproductive_readiness:0.0,active_transformation_id:None,reproductive_construction:None})
}
