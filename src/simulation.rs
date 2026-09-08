use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

use crate::decision::{ActionEligibility, ActionKind, CurrentNeeds, DecisionParameters};
use crate::decision_runtime::{select_action, ActionCandidate, DecisionContext};
use crate::environment::{
    apply_settling, apply_vents, ActiveMaterialField, DeepReservoir, Vent, DEFAULT_CELL_SIZE,
    DEFAULT_DIFFUSION_FRACTION, DEFAULT_RESERVOIR_BLOCK_SIZE, DEFAULT_SETTLING_FRACTION,
    DEFAULT_SETTLING_INTERVAL_TICKS,
};
use crate::genome::initial_genome;
use crate::state::{
    DevelopmentStage, EnergyLedger, Environment, Organism, Position, ResourceSense, Simulation,
};

const ADULTHOOD_GROWTH_FRACTION: f64 = 0.90;
const ACQUISITION_CONTACT_TOLERANCE: f64 = 1e-9;
const INITIAL_ORGANISM_POSITION: Position = Position { x: 500.0, y: 500.0 };

impl Simulation {
    pub(crate) fn new(seed: u64, ticks_per_second: f64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let environment = Self::create_environment();
        let organism = Self::create_initial_organism();
        Self { tick: 0, ticks_per_second, running: true, organisms: vec![organism], environment, active_transformations: Vec::new(), decomposing_bodies: Vec::new(), energy_ledger: EnergyLedger::default(), next_organism_id: 2, next_transformation_id: 1, rng, decision_parameters: DecisionParameters::default() }
    }

    pub(crate) fn create_environment() -> Environment {
        let catalog = crate::resources::default_catalog();
        let width = 1000.0; let height = 1000.0;
        let field = ActiveMaterialField::new(width, height, DEFAULT_CELL_SIZE);
        let mut reservoir = DeepReservoir::new_matching_field(&field, DEFAULT_RESERVOIR_BLOCK_SIZE);
        let starting_amounts: [(&str, f64); 7] = [("Carbon",10_000.0),("Methane",5_000.0),("Hydrogen",5_000.0),("Sulfur",5_000.0),("Nitrogen",5_000.0),("Phosphorus",5_000.0),("Water",20_000.0)];
        for (name, amount) in starting_amounts { reservoir.seed_uniform(name, amount); }
        let vents = vec![
            Vent{x:250.0,y:250.0,composition:vec![("Carbon".into(),0.10),("Methane".into(),0.45),("Hydrogen".into(),0.25),("Sulfur".into(),0.10),("Nitrogen".into(),0.05),("Phosphorus".into(),0.02),("Water".into(),0.03)],emission_amount:100.0,emission_interval:20,emission_timer:0},
            Vent{x:750.0,y:300.0,composition:vec![("Carbon".into(),0.35),("Methane".into(),0.10),("Hydrogen".into(),0.15),("Sulfur".into(),0.25),("Nitrogen".into(),0.05),("Phosphorus".into(),0.05),("Water".into(),0.05)],emission_amount:100.0,emission_interval:30,emission_timer:0},
            Vent{x:520.0,y:550.0,composition:vec![("Carbon".into(),0.25),("Methane".into(),0.15),("Hydrogen".into(),0.30),("Sulfur".into(),0.10),("Nitrogen".into(),0.10),("Phosphorus".into(),0.02),("Water".into(),0.08)],emission_amount:100.0,emission_interval:25,emission_timer:0},
        ];
        Environment{width,height,catalog,field,reservoir,vents}
    }

    pub(crate) fn create_initial_organism() -> Organism {
        let genome = initial_genome(); let catalog = crate::resources::default_catalog();
        let mut structure = genome.structural_blueprint.realize(&catalog).expect("initial structural blueprint must be realizable");
        for unit in &mut structure.units {
            unit.placement.x += INITIAL_ORGANISM_POSITION.x;
            unit.placement.y += INITIAL_ORGANISM_POSITION.y;
        }
        Organism{id:"1".into(),occupied_cells:vec![INITIAL_ORGANISM_POSITION],genome,resource_sense:ResourceSense{sensed_resources:Vec::new(),sensed_organisms:Vec::new(),direction_x:0.0,direction_y:0.0,direction_strength:0.0},memory:Vec::new(),decision_history:crate::decision::DecisionHistory::default(),usable_energy:0.0,stress:0.0,stress_threshold:crate::state::INITIAL_STRESS_THRESHOLD,stored_material:crate::material_storage::MaterialStorage::default(),structure,development_stage:DevelopmentStage::Juvenile,age:0,reproductive_readiness:0.0,active_transformation_id:None,reproductive_construction:None}
    }

    pub(crate) fn step_environment(&mut self){apply_vents(&mut self.environment.field,&mut self.environment.reservoir,&mut self.environment.vents);self.environment.field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);if self.tick%DEFAULT_SETTLING_INTERVAL_TICKS==0{apply_settling(&mut self.environment.field,&mut self.environment.reservoir,DEFAULT_SETTLING_FRACTION);}}
    fn mature_structural_mass(organism:&Organism,environment:&Environment)->f64{organism.genome.structural_blueprint.structural_mass(&environment.catalog)}
    fn growth_fraction(organism:&Organism,environment:&Environment)->f64{let mature_mass=Self::mature_structural_mass(organism,environment);if !mature_mass.is_finite()||mature_mass<=0.0{return 0.0}(organism.structural_mass(&environment.catalog)/mature_mass).clamp(0.0,1.0)}
    fn update_development_stage(organism:&mut Organism,environment:&Environment){match organism.development_stage{DevelopmentStage::Offspring=>{if organism.reproductive_construction.is_none(){organism.development_stage=DevelopmentStage::Juvenile}},DevelopmentStage::Juvenile=>{if Self::growth_fraction(organism,environment)>=ADULTHOOD_GROWTH_FRACTION{organism.development_stage=DevelopmentStage::Adult}},DevelopmentStage::Adult=>{}}}
    fn current_needs(organism:&Organism,environment:&Environment,parameters:DecisionParameters)->CurrentNeeds{let survival_reserve=parameters.survival_reserve.max(f64::EPSILON);let reserve_pressure=(1.0-organism.usable_energy/survival_reserve).clamp(0.0,1.0);let energetic_survival=(reserve_pressure*(1.0+organism.stress.max(0.0))).clamp(0.0,1.0);let developmental_survival=if matches!(organism.development_stage,DevelopmentStage::Juvenile){(1.0-Self::growth_fraction(organism,environment)).clamp(0.0,1.0)}else{0.0};let survival=energetic_survival.max(developmental_survival);CurrentNeeds{survival,reproduction:organism.reproductive_readiness.clamp(0.0,1.0)}}
    fn update_reproductive_readiness(organism:&mut Organism,environment:&Environment,parameters:DecisionParameters){if !matches!(organism.development_stage,DevelopmentStage::Adult){return}let mature_mass=Self::mature_structural_mass(organism,environment).max(f64::EPSILON);let maturity=(organism.structural_mass(&environment.catalog)/mature_mass).clamp(0.0,1.0);let reproduction_reserve=parameters.reproduction_reserve.max(f64::EPSILON);let energy_readiness=(organism.usable_energy/reproduction_reserve).clamp(0.0,1.0);let accumulation=(maturity*energy_readiness*parameters.reproduction_accumulation_rate.max(0.0)).clamp(0.0,1.0);organism.reproductive_readiness=(organism.reproductive_readiness+accumulation).clamp(0.0,1.0)}

    fn acquisition_targets(organism:&Organism,environment:&Environment)->Vec<u64>{let Some(body)=crate::organism_geometry::OrganismBodyGeometry::from_structure(&organism.structure,&environment.catalog)else{return Vec::new()};let Some(capacity)=crate::water::experimental_transfer_capacity(organism,&environment.catalog)else{return Vec::new()};if capacity<1.0{return Vec::new()}environment.field.contacting_materials(&body,&environment.catalog,ACQUISITION_CONTACT_TOLERANCE).into_iter().filter_map(|(cell_index,material_index,_)|environment.field.cells.get(cell_index).and_then(|cell|cell.materials.get(material_index))).filter(|material|{if material.is_empty()||!material.is_valid(){return false}if material.has_internal_structure(){material.total_amount()<=capacity+crate::field::MATERIAL_EPSILON}else{material.total_amount()>=1.0}}).map(|material|material.id).collect()}
    fn acquisition_context_key(material_id:u64)->String{format!("target:{material_id}")}
    fn bond_context_key(bond:&crate::structure::Bond)->String{format!("bond:{}:{}:{}:{}",bond.unit_a,bond.point_a,bond.unit_b,bond.point_b)}
    fn action_eligibility(organism:&Organism,environment:&Environment,organisms:&[Organism])->ActionEligibility{let can_build_from_storage=!organism.structure.units.is_empty()&&organism.stored_material.has_valid_material();let can_join_existing_structure=organism.structure.units.len()>=2;let can_external_break=!crate::transformation::external_break_candidates(organism,organisms,&environment.catalog).is_empty();ActionEligibility{can_move:organism.active_transformation_id.is_none(),can_acquire:organism.active_transformation_id.is_none()&&!Self::acquisition_targets(organism,environment).is_empty(),can_combine:organism.active_transformation_id.is_none()&&(can_build_from_storage||can_join_existing_structure),can_break:organism.active_transformation_id.is_none()&&(!organism.structure.bonds.is_empty()||can_external_break),can_expel:false}}
    fn decision_candidates(organism:&Organism,environment:&Environment,organisms:&[Organism],needs:CurrentNeeds,eligibility:ActionEligibility)->Vec<ActionCandidate>{let mut candidates=Vec::new();let relevant=|action:ActionKind|eligibility.permits(action)&&needs.any_for(action.relevant_needs());if relevant(ActionKind::Break){candidates.extend(organism.structure.bonds.iter().map(|bond|ActionCandidate{action:ActionKind::Break,context_key:Some(Self::bond_context_key(bond))}));candidates.extend(crate::transformation::external_break_candidates(organism,organisms,&environment.catalog));}if relevant(ActionKind::Combine){candidates.push(ActionCandidate{action:ActionKind::Combine,context_key:None})}if relevant(ActionKind::Move){candidates.push(ActionCandidate{action:ActionKind::Move,context_key:None})}if relevant(ActionKind::Acquire){candidates.extend(Self::acquisition_targets(organism,environment).into_iter().map(|material_id|ActionCandidate{action:ActionKind::Acquire,context_key:Some(Self::acquisition_context_key(material_id))}))}if relevant(ActionKind::Expel){candidates.push(ActionCandidate{action:ActionKind::Expel,context_key:None})}candidates}

    fn acquire_target(organism:&mut Organism,environment:&mut Environment,material_id:u64)->bool{let Some(body)=crate::organism_geometry::OrganismBodyGeometry::from_structure(&organism.structure,&environment.catalog)else{return false};let Some(capacity)=crate::water::experimental_transfer_capacity(organism,&environment.catalog)else{return false};if capacity<1.0{return false}let contact_exists=environment.field.contacting_materials(&body,&environment.catalog,ACQUISITION_CONTACT_TOLERANCE).into_iter().any(|(cell_index,material_index,_)|environment.field.cells.get(cell_index).and_then(|cell|cell.materials.get(material_index)).map(|material|material.id==material_id).unwrap_or(false));if !contact_exists{return false}let Some(material)=environment.field.take_for_acquisition_by_id(material_id,capacity)else{return false};organism.store_material(material)}

    fn recycle_material(environment:&mut Environment,position:&Position,material:crate::resources::Material){if material.has_internal_structure(){let placement=crate::structure::Placement{x:position.x,y:position.y,rotation_radians:0.0};let _=environment.field.deposit_structured(material,vec![placement]);}else{environment.field.deposit(position.x,position.y,material);}}

    fn recycle_dead_organism(environment:&mut Environment,organism:&Organism)->Option<crate::decomposition::DecomposingBody>{let position=organism.occupied_cells.first().cloned()?;for material in organism.stored_material.materials.iter().cloned(){Self::recycle_material(environment,&position,material);}if let Some(construction)=&organism.reproductive_construction{for material in construction.committed_material.materials.iter().cloned(){Self::recycle_material(environment,&position,material);}}let structure=if let Some(construction)=&organism.reproductive_construction{crate::reproduction::take_dead_construction_structure(&organism.structure,construction,position.clone())}else{organism.structure.clone()};crate::decomposition::DecomposingBody::new(structure,organism.usable_energy,position)}

    fn process_decomposing_bodies(&mut self){
        let mut finished_indices=Vec::new();
        for index in 0..self.decomposing_bodies.len(){
            let Some(step)=crate::decomposition::resolve_one_bond(&mut self.decomposing_bodies[index],&self.environment)else{continue};
            self.energy_ledger.total_heat_dissipated+=step.heat;
            self.energy_ledger.total_potential_energy_released+=step.bond_energy;
            if step.break_interaction_energy>0.0{self.energy_ledger.total_potential_energy_released+=step.break_interaction_energy;}
            if step.net_energy>0.0{
                if let Some(organism_index)=crate::decomposition::harvestable_decomposition_energy(&self.organisms,&self.decomposing_bodies[index].position){
                    self.organisms[organism_index].usable_energy+=step.net_energy;
                    self.energy_ledger.total_usable_energy_gained+=step.net_energy;
                }
            }
            if let Some(materials)=step.released_material{for (material,placement) in materials{let position=Position{x:placement.x,y:placement.y};Self::recycle_material(&mut self.environment,&position,material);}finished_indices.push(index);}
        }
        for index in finished_indices.into_iter().rev(){self.decomposing_bodies.remove(index);}
    }

    pub(crate) fn step(&mut self){
        self.tick+=1;self.step_environment();
        let mut still_active=Vec::new();let mut completed=Vec::new();for mut transformation in self.active_transformations.drain(..){if transformation.remaining_ticks>0{transformation.remaining_ticks-=1}if transformation.remaining_ticks==0{completed.push(transformation)}else{still_active.push(transformation)}}self.active_transformations=still_active;
        let mut completed_organisms=HashSet::new();for transformation in &completed{completed_organisms.insert(transformation.organism_id.clone());Self::resolve_transformation(transformation,&mut self.organisms,&mut self.environment,&mut self.energy_ledger);}
        let environment_snapshot=self.environment.clone();let organism_snapshot=self.organisms.clone();let decision_parameters=self.decision_parameters;for organism in &mut self.organisms{organism.age+=1;Self::update_development_stage(organism,&environment_snapshot);Self::update_resource_perception(organism,&environment_snapshot);Self::update_organism_perception(organism,&organism_snapshot,&environment_snapshot);Self::update_memory_from_sources(organism,&environment_snapshot);Self::update_reproductive_readiness(organism,&environment_snapshot,decision_parameters);}
        let mut reproduction_requests=Vec::new();{let(organisms,environment)=(&mut self.organisms,&mut self.environment);let mut compatibility_cache=crate::contact::ConnectionCompatibilityCache::new();for organism in organisms{if completed_organisms.contains(&organism.id){continue}let needs=Self::current_needs(organism,environment,decision_parameters);let eligibility=Self::action_eligibility(organism,environment,&organism_snapshot);let context=DecisionContext{needs,eligibility};let candidates=Self::decision_candidates(organism,environment,&organism_snapshot,needs,eligibility);let Some(selected)=select_action(context,&organism.decision_history,&candidates)else{continue};match selected.action{ActionKind::Move=>{let moved=Self::update_movement(organism,environment);crate::decision_runtime::record_outcome(&mut organism.decision_history,&selected,if moved{crate::decision::OutcomeKind::Neutral}else{crate::decision::OutcomeKind::Harmful});}ActionKind::Combine=>{let combined=crate::combine_runtime::try_combine(organism,environment,&mut compatibility_cache).is_some();crate::decision_runtime::record_outcome(&mut organism.decision_history,&selected,if combined{crate::decision::OutcomeKind::Neutral}else{crate::decision::OutcomeKind::Harmful});if combined&&organism.reproductive_readiness>=1.0-f64::EPSILON{reproduction_requests.push(organism.id.clone());}}ActionKind::Break=>{if let Some(transformation)=Self::try_start_transformation(organism,&environment.catalog,&mut self.next_transformation_id,&selected,&organism_snapshot){self.active_transformations.push(transformation);}}ActionKind::Acquire=>{let success=selected.context_key.as_deref().and_then(|key|key.strip_prefix("target:")).and_then(|id|id.parse::<u64>().ok()).map(|material_id|Self::acquire_target(organism,environment,material_id)).unwrap_or(false);crate::decision_runtime::record_outcome(&mut organism.decision_history,&selected,if success{crate::decision::OutcomeKind::Neutral}else{crate::decision::OutcomeKind::Harmful});}ActionKind::Expel=>{}}}}
        let catalog=self.environment.catalog.clone();for id in reproduction_requests{if let Some(organism)=self.organisms.iter_mut().find(|o|o.id==id){let _=crate::reproduction::begin_reproduction(organism,&mut self.rng,&catalog);}}
        let mut offspring=Vec::new();let mut next_organism_id=self.next_organism_id;for organism in &mut self.organisms{if organism.reproductive_construction.is_some(){if let Some(construction)=organism.reproductive_construction.as_mut(){if let Some(stress)=crate::reproduction::advance_construction(&mut organism.stored_material,construction,&catalog){organism.add_transaction_stress(stress);}}if organism.reproductive_construction.as_ref().map(|construction|construction.realized_elements.len()==construction.target_elements.len()).unwrap_or(false){let child_id=next_organism_id.to_string();if let Some(child)=crate::reproduction::finish_reproduction(organism,child_id){next_organism_id+=1;offspring.push(child);}}}}self.next_organism_id=next_organism_id;self.organisms.extend(offspring);
        let mut survivors=Vec::with_capacity(self.organisms.len());for mut organism in self.organisms.drain(..){let dead=Self::apply_energy_capacity(&mut organism,&self.environment,&mut self.energy_ledger);if dead{if let Some(body)=Self::recycle_dead_organism(&mut self.environment,&organism){self.decomposing_bodies.push(body);}}else{survivors.push(organism);}}self.organisms=survivors;
        self.process_decomposing_bodies();
        let live_ids:HashSet<String>=self.organisms.iter().map(|o|o.id.clone()).collect();self.active_transformations.retain(|t|live_ids.contains(&t.organism_id));self.energy_ledger.total_usable_energy_held=self.organisms.iter().map(|o|o.usable_energy).sum();
    }

    pub(crate) fn apply_energy_capacity(organism:&mut Organism,environment:&Environment,ledger:&mut EnergyLedger)->bool{organism.stress*=crate::state::STRESS_DECAY_PER_TICK;organism.apply_stress_damage(environment,ledger)}
    #[cfg(test)]pub(crate)fn total_material_in_system(&self)->f64{let mut total=self.environment.field.total_amount()+self.environment.reservoir.total_amount();for transformation in &self.active_transformations{total+=transformation.material.total_amount();}for organism in &self.organisms{total+=organism.stored_material.total_amount();if let Some(construction)=&organism.reproductive_construction{total+=construction.committed_material.total_amount();total+=construction.developing_structure.units.iter().map(|unit|unit.material.material().total_amount()).sum::<f64>();}total+=organism.structure.units.iter().map(|unit|unit.material.material().total_amount()).sum::<f64>();}for body in &self.decomposing_bodies{total+=body.structure.units.iter().map(|unit|unit.material.material().total_amount()).sum::<f64>();}total}
}