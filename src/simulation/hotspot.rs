//! Localized radial energy source used by the environmental chemistry experiment.
//!
//! The hotspot contributes an ordinary 440 Hz resonant signal. Its spatial
//! intensity is a smooth radial gradient; material properties determine the
//! response, and only energy actually retained by the environmental resonance
//! reservoir is made available to chemistry.

use crate::energy_ledger::EnergyLedgerAuthority;
use crate::resources::Form;
use crate::state::{EnergyLedger, Environment};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct RadialHotspot {
    pub(crate) center_x: f64,
    pub(crate) center_y: f64,
    pub(crate) radius: f64,
    /// Incident energy per tick per unit of realized material area at the center.
    pub(crate) peak_energy_per_tick_per_area: f64,
    pub(crate) frequency_hz: f64,
}

impl RadialHotspot {
    /// Place one hotspot in the right-hand bend, between the right eddy center
    /// and the adjacent S-flow spine. The radius scales with the world height.
    pub(crate) fn right_bend_for_world(width: f64, height: f64) -> Option<Self> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return None;
        }
        let x_fraction = 0.72;
        let phase = std::f64::consts::TAU * x_fraction;
        let hotspot = Self {
            center_x: width * x_fraction,
            center_y: height * 0.5 + height * 0.23 * phase.sin(),
            radius: height.min(width) * 0.15,
            peak_energy_per_tick_per_area: 100.0,
            frequency_hz: 440.0,
        };
        hotspot.is_valid().then_some(hotspot)
    }

    pub(crate) fn is_valid(self) -> bool {
        [
            self.center_x,
            self.center_y,
            self.radius,
            self.peak_energy_per_tick_per_area,
            self.frequency_hz,
        ]
        .iter()
        .all(|value| value.is_finite())
            && self.center_x >= 0.0
            && self.center_y >= 0.0
            && self.radius > 0.0
            && self.peak_energy_per_tick_per_area > 0.0
            && self.frequency_hz > 0.0
    }

    /// Cosine falloff has zero slope at both the center and the outer edge.
    pub(crate) fn intensity_at(self, x: f64, y: f64) -> Option<f64> {
        if !self.is_valid() || !x.is_finite() || !y.is_finite() {
            return None;
        }
        let distance = (x - self.center_x).hypot(y - self.center_y);
        if !distance.is_finite() {
            return None;
        }
        if distance >= self.radius {
            return Some(0.0);
        }
        let normalized = distance / self.radius;
        let falloff = 0.5 * (1.0 + (std::f64::consts::PI * normalized).cos());
        let intensity = self.peak_energy_per_tick_per_area * falloff;
        (intensity.is_finite() && intensity >= 0.0).then_some(intensity)
    }
}

fn form_area(form: &Form) -> Option<f64> {
    let area = match form {
        Form::Circle { radius } if radius.is_finite() && *radius > 0.0 => {
            std::f64::consts::PI * radius * radius
        }
        Form::Rectangle { width, height }
            if width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0 =>
        {
            width * height
        }
        Form::RegularPolygon { sides, radius }
            if *sides >= 3 && radius.is_finite() && *radius > 0.0 =>
        {
            0.5 * f64::from(*sides) * radius * radius
                * (std::f64::consts::TAU / f64::from(*sides)).sin()
        }
        Form::Polygon { vertices } if vertices.len() >= 3 => {
            let mut twice_area = 0.0;
            for index in 0..vertices.len() {
                let (x1, y1) = vertices[index];
                let (x2, y2) = vertices[(index + 1) % vertices.len()];
                if ![x1, y1, x2, y2].iter().all(|value| value.is_finite()) {
                    return None;
                }
                twice_area += x1 * y2 - x2 * y1;
            }
            twice_area.abs() * 0.5
        }
        Form::Fluid { nominal_area, .. } if nominal_area.is_finite() && *nominal_area > 0.0 => {
            *nominal_area
        }
        _ => return None,
    };
    (area.is_finite() && area > 0.0).then_some(area)
}

fn spectrum_energy(spectrum: &crate::harmonics::ToneSpectrum) -> Option<f64> {
    let mut energy = 0.0;
    for component in &spectrum.components {
        if !component.frequency_hz.is_finite()
            || component.frequency_hz <= 0.0
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

/// Settle the hotspot against realized environmental material only. The
/// transaction is staged so an invalid response or ledger allocation cannot
/// partially credit the environmental resonance reservoir.
pub(crate) fn settle_environment_inputs(
    hotspot: RadialHotspot,
    environment: &mut Environment,
    ledger: &mut EnergyLedger,
) -> bool {
    if !hotspot.is_valid() {
        return false;
    }
    let baselines = crate::resources::ResourceBaselines::from_catalog(&environment.catalog);
    let mut staged_reservoir = environment.resonant_energy.clone();
    let mut total_incident = 0.0;
    let mut total_captured = 0.0;
    let mut total_heat = 0.0;
    let total_escaped;

    for cell in &environment.field.cells {
        for physical in &cell.physical_materials {
            let Some(placements) = physical.placements.as_ref() else {
                return false;
            };
            if placements.len() != physical.material.parts.len() {
                return false;
            }
            for (index, (name, amount)) in physical.material.parts.iter().enumerate() {
                if !amount.is_finite() || *amount <= 0.0 {
                    return false;
                }
                let Some(placement) = placements.get(index) else {
                    return false;
                };
                if !placement.x.is_finite()
                    || !placement.y.is_finite()
                    || !placement.rotation_radians.is_finite()
                {
                    return false;
                }
                let Some(intensity) = hotspot.intensity_at(placement.x, placement.y) else {
                    return false;
                };
                if intensity <= f64::EPSILON {
                    continue;
                }
                let Some(resource) = environment.catalog.iter().find(|item| item.name == *name)
                else {
                    return false;
                };
                let Some(area) = form_area(&resource.shape.form) else {
                    return false;
                };
                let incident = intensity * area * amount;
                if !incident.is_finite() || incident < 0.0 {
                    return false;
                }
                if incident <= f64::EPSILON {
                    continue;
                }

                let signal = crate::harmonics::ToneSpectrum {
                    components: vec![crate::harmonics::ToneComponent {
                        frequency_hz: hotspot.frequency_hz,
                        amplitude: incident.sqrt(),
                        phase_radians: 0.0,
                    }],
                };
                let Some(response) = crate::harmonics::material_response_spectrum(
                    resource.properties,
                    baselines,
                    &signal,
                ) else {
                    return false;
                };
                let Some(response_energy) = spectrum_energy(&response) else {
                    return false;
                };
                if response_energy > incident + 1e-9 * incident.max(1.0) {
                    return false;
                }
                let response_energy = response_energy.min(incident);
                let damping = crate::harmonics::damping_ratio(resource.properties, baselines);
                if !damping.is_finite() || damping < 0.0 {
                    return false;
                }
                let damping_heat = response_energy * damping.clamp(0.0, 1.0);
                let retained_budget = (response_energy - damping_heat).max(0.0);
                let Some(retained) = crate::harmonics::transform_signal_to_energy_budget(
                    resource.properties,
                    baselines,
                    &signal,
                    retained_budget,
                ) else {
                    return false;
                };

                let Some(before) = staged_reservoir.total_energy() else {\n                    return false;\n                };
                let mut candidate = staged_reservoir.clone();
                let mut deposit_ok = true;
                for component in &retained.components {
                    let energy = component.amplitude * component.amplitude;
                    if energy > f64::EPSILON
                        && !candidate.deposit(component.frequency_hz, energy)
                    {
                        deposit_ok = false;
                        break;
                    }
                }
                let captured = if deposit_ok {
                    let Some(after) = candidate.total_energy() else {\n                        return false;\n                    };
                    let delta = after - before;
                    if !delta.is_finite() || delta < 0.0 || delta > response_energy + 1e-9 {
                        return false;
                    }
                    staged_reservoir = candidate;
                    delta.min(response_energy)
                } else {
                    0.0
                };

                total_incident += incident;
                total_captured += captured;
                total_heat += response_energy - captured;
                total_escaped += incident - response_energy;
                if ![total_incident, total_captured, total_heat, total_escaped]
                    .iter()
                    .all(|value| value.is_finite() && *value >= 0.0)
                {
                    return false;
                }
            }
        }
    }

    if total_incident <= f64::EPSILON {
        return true;
    }
    // Derive escape as the final shared residual so the source and all sinks
    // reconcile from the same aggregate arithmetic.
    total_escaped = total_incident - total_captured - total_heat;
    if !total_escaped.is_finite() || total_escaped < 0.0 {
        return false;
    }
    if !ledger.settle_world_tone_paths(total_incident, total_captured, total_heat, total_escaped) {
        return false;
    }
    environment.resonant_energy = staged_reservoir;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_bend_hotspot_has_a_smooth_radial_gradient_and_100_unit_peak() {
        let hotspot = RadialHotspot::right_bend_for_world(1000.0, 1000.0)
            .expect("the default world should have a valid right-bend hotspot");
        assert!((hotspot.intensity_at(hotspot.center_x, hotspot.center_y).unwrap() - 100.0).abs() < 1e-12);
        assert!((hotspot.intensity_at(hotspot.center_x + hotspot.radius * 0.5, hotspot.center_y).unwrap() - 50.0).abs() < 1e-10);
        assert_eq!(hotspot.intensity_at(hotspot.center_x + hotspot.radius, hotspot.center_y), Some(0.0));
        assert_eq!(hotspot.intensity_at(hotspot.center_x + hotspot.radius * 1.1, hotspot.center_y), Some(0.0));
    }

    #[test]
    fn right_bend_hotspot_is_between_the_right_eddy_and_the_s_flow_spine() {
        let hotspot = RadialHotspot::right_bend_for_world(1000.0, 1000.0).unwrap();
        let spine_y = 500.0 + 230.0 * (std::f64::consts::TAU * 0.72).sin();
        let eddy_x = 750.0;
        let eddy_y = 500.0 + 230.0 * (std::f64::consts::TAU * 0.75).sin();
        assert!((hotspot.center_y - spine_y).abs() < 1e-10);
        assert!(hotspot.center_x > 700.0 && hotspot.center_x < eddy_x);
        assert!((hotspot.center_y - eddy_y).abs() < 20.0);
    }

    #[test]
    fn hotspot_is_opt_in_and_does_not_change_the_default_world() {
        let mut simulation = crate::state::Simulation::new(911, 1.0);
        assert!(simulation.hotspot.is_none());
        assert!(simulation.enable_chemistry_experiment_hotspot());
        let hotspot = simulation.hotspot.expect("explicit experiment setup should enable the source");
        assert!((hotspot.peak_energy_per_tick_per_area - 100.0).abs() < f64::EPSILON);
        assert!((hotspot.center_x - 720.0).abs() < 1e-10);
    }

    #[test]
    fn hotspot_energy_is_captured_as_environmental_resonance_and_balances() {
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::Material;
        use crate::state::Simulation;
        use crate::structure::Placement;

        let mut simulation = Simulation::new(912, 1.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let hotspot = RadialHotspot {
            center_x: 500.0,
            center_y: 500.0,
            radius: 100.0,
            peak_energy_per_tick_per_area: 100.0,
            frequency_hz: 440.0,
        };
        let physical = PhysicalMaterial::realized(
            Material::free_base("Nitrogen", 1.0),
            vec![Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 }],
            &simulation.environment.catalog,
        ).expect("test material should have physical geometry");
        assert!(simulation.environment.field.deposit_physical(
            500.0, 500.0, physical, &simulation.environment.catalog
        ));
        let before = simulation.energy_inventory_snapshot();
        assert!(settle_environment_inputs(hotspot, &mut simulation.environment, &mut simulation.energy_ledger));
        let after = simulation.energy_inventory_snapshot();
        assert!(simulation.environment.resonant_energy.total_energy().unwrap() > 0.0);
        assert!(after.is_balanced(1e-7), "hotspot energy must reconcile: before={:?}, after={:?}", before.residual(), after.residual());
    }
}
