use crate::state::{
    AffinityResponses, Environment, Organism, PropertyDeviations, ResourceObservation,
    DESIRABILITY_AMOUNT_HALF_SATURATION, DESIRABILITY_MAX,
};

impl crate::state::Simulation {
    pub(crate) fn calculate_property_deviations(
        properties: &crate::resources::ResourceProperties,
        baselines: &crate::resources::ResourceBaselines,
        ranges: &crate::resources::ResourceProperties,
    ) -> PropertyDeviations {
        PropertyDeviations {
            mass: ((properties.mass - baselines.mass) / ranges.mass).clamp(-1.0, 1.0),
            potential_energy: ((properties.potential_energy - baselines.potential_energy) / ranges.potential_energy).clamp(-1.0, 1.0),
            reactivity: ((properties.reactivity - baselines.reactivity) / ranges.reactivity).clamp(-1.0, 1.0),
            cohesion: ((properties.cohesion - baselines.cohesion) / ranges.cohesion).clamp(-1.0, 1.0),
        }
    }

    pub(crate) fn affinity_response(deviation: f64, affinity: f64) -> f64 {
        (deviation * affinity * 3.0).tanh()
    }

    pub(crate) fn amount_factor(amount: f64) -> f64 {
        let amount = amount.max(0.0);
        amount / (amount + DESIRABILITY_AMOUNT_HALF_SATURATION)
    }

    pub(crate) fn energy_need_factor(usable_energy: f64) -> f64 {
        1.0 / (1.0 + usable_energy.max(0.0))
    }

    pub(crate) fn calculate_desirability(
        organism: &Organism,
        properties: &crate::resources::ResourceProperties,
        perceived_amount: f64,
        baselines: &crate::resources::ResourceBaselines,
        ranges: &crate::resources::ResourceProperties,
    ) -> (PropertyDeviations, AffinityResponses, f64, f64, f64, f64) {
        let deviations = Self::calculate_property_deviations(properties, baselines, ranges);
        let responses = AffinityResponses {
            mass: Self::affinity_response(deviations.mass, organism.genome.mass_affinity()),
            potential_energy: Self::affinity_response(deviations.potential_energy, organism.genome.potential_energy_affinity()),
            reactivity: Self::affinity_response(deviations.reactivity, organism.genome.reactivity_affinity()),
            cohesion: Self::affinity_response(deviations.cohesion, organism.genome.cohesion_affinity()),
        };
        let energy_need = Self::energy_need_factor(organism.usable_energy);
        let energy_response = responses.potential_energy * (1.0 + energy_need);
        let base_desirability = (responses.mass + energy_response + responses.reactivity + responses.cohesion) / 4.0;
        let amount_factor = Self::amount_factor(perceived_amount);
        let desirability = (base_desirability * amount_factor).clamp(-DESIRABILITY_MAX, DESIRABILITY_MAX);
        (deviations, responses, base_desirability, amount_factor, energy_need, desirability)
    }

    pub(crate) fn update_resource_perception(
        organism: &mut Organism,
        environment: &Environment,
    ) {
        let perception_radius = organism.genome.perception_radius();
        let sensory_resolution = organism.genome.sensory_resolution();
        let directional_resolution = organism.genome.directional_resolution();
        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
        };

        organism.resource_sense.sensed_resources.clear();
        organism.resource_sense.direction_x = 0.0;
        organism.resource_sense.direction_y = 0.0;
        organism.resource_sense.direction_strength = 0.0;

        let baselines = crate::resources::ResourceBaselines::from_catalog(&environment.catalog);
        let ranges = crate::resources::property_ranges(&environment.catalog);
        for cell_index in environment.field.cells_within_radius(px, py, perception_radius) {
            let (cell_x, cell_y) = environment.field.cell_center(cell_index);
            let dx = cell_x - px;
            let dy = cell_y - py;
            let distance = (dx * dx + dy * dy).sqrt();
            let direction_x = if distance > 0.0 { dx / distance } else { 0.0 };
            let direction_y = if distance > 0.0 { dy / distance } else { 0.0 };
            let cell = &environment.field.cells[cell_index];

            for (bonded, material) in [(true, &cell.bonded), (false, &cell.unbonded)] {
                let perceived_amount = material.total_amount() * sensory_resolution;
                if perceived_amount <= 0.0 { continue; }
                let properties = material.weighted_properties(&environment.catalog);
                let (deviations, responses, base_desirability, amount_factor, energy_need_factor, desirability) =
                    Self::calculate_desirability(organism, &properties, perceived_amount, &baselines, &ranges);
                let label = material.parts.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join("+");
                organism.resource_sense.sensed_resources.push(ResourceObservation {
                    name: label,
                    properties,
                    bonded,
                    perceived_amount,
                    deviations,
                    affinity_responses: responses,
                    base_desirability,
                    amount_factor,
                    potential_energy_need_factor: energy_need_factor,
                    desirability,
                    distance,
                    source_x: cell_x,
                    source_y: cell_y,
                    field_index: cell_index,
                });
                organism.resource_sense.direction_x += direction_x * desirability;
                organism.resource_sense.direction_y += direction_y * desirability;
            }
        }

        let magnitude = (organism.resource_sense.direction_x * organism.resource_sense.direction_x
            + organism.resource_sense.direction_y * organism.resource_sense.direction_y).sqrt();
        organism.resource_sense.direction_strength = magnitude;
        if magnitude <= f64::EPSILON { return; }
        organism.resource_sense.direction_x /= magnitude;
        organism.resource_sense.direction_y /= magnitude;
        let angle = organism.resource_sense.direction_y.atan2(organism.resource_sense.direction_x);
        let resolution = directional_resolution.max(0.001);
        let direction_steps = (resolution * 32.0).max(1.0);
        let step_angle = std::f64::consts::TAU / direction_steps;
        let quantized_angle = (angle / step_angle).round() * step_angle;
        organism.resource_sense.direction_x = quantized_angle.cos();
        organism.resource_sense.direction_y = quantized_angle.sin();
    }
}
