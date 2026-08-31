//! Isolated energy-accounting primitives for COMBINE and BREAK.
//!
//! These functions deliberately separate architectural mechanics from the
//! unresolved biological/material equations. Callers supply formation work,
//! break work, and the eventual energy partition parameters.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombineEnergyResult {
    pub input_potential_energy: f64,
    pub formation_work: f64,
    pub organism_energy_deficit: f64,
    pub interaction_surplus: f64,
    pub bond_energy: f64,
    pub usable_energy: f64,
    pub heat_stress: f64,
}

/// Resolve the energetic accounting of a successful COMBINE.
///
/// The caller supplies the eventual bond/usable/heat partition. This keeps
/// the conservation invariant explicit without prematurely choosing the
/// unresolved partition equation.
///
/// `input_potential_energy` is the raw energetic opportunity presented by
/// the participating resources. `formation_work` is the work required to
/// establish the interaction. If the latter exceeds the former, the caller
/// must provide the deficit from organism energy; that deficit is reported
/// separately rather than silently turning negative energy into heat.
pub fn resolve_combine_energy(
    input_potential_energy: f64,
    formation_work: f64,
    bond_energy: f64,
    usable_energy: f64,
) -> Option<CombineEnergyResult> {
    if !input_potential_energy.is_finite()
        || !formation_work.is_finite()
        || !bond_energy.is_finite()
        || !usable_energy.is_finite()
        || input_potential_energy < 0.0
        || formation_work < 0.0
        || bond_energy < 0.0
        || usable_energy < 0.0
    {
        return None;
    }

    let organism_energy_deficit = (formation_work - input_potential_energy).max(0.0);
    let interaction_surplus = (input_potential_energy - formation_work).max(0.0);

    // Bond + usable must never exceed the energy actually available after
    // formation work. The remainder is the explicitly accounted-for
    // heat/stress channel. When the interaction has a deficit, the
    // organism's payment covers formation work and no interaction surplus
    // exists to allocate.
    if bond_energy + usable_energy > interaction_surplus + 1e-12 {
        return None;
    }

    let heat_stress = (interaction_surplus - bond_energy - usable_energy).max(0.0);

    Some(CombineEnergyResult {
        input_potential_energy,
        formation_work,
        organism_energy_deficit,
        interaction_surplus,
        bond_energy,
        usable_energy,
        heat_stress,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BreakEnergyResult {
    pub bond_energy: f64,
    pub break_work: f64,
    pub net_energy: f64,
    pub usable_energy: f64,
    pub heat_stress: f64,
}

/// Resolve BREAK from the bond's stored energy.
///
/// Raw resource potential energy is intentionally absent from this API.
/// BREAK consumes the bond's stored energy opportunity and compares it to
/// break work. A favorable break can produce usable energy; an unfavorable
/// break produces a deficit that becomes stress/heat.
pub fn resolve_break_energy(
    bond_energy: f64,
    break_work: f64,
    processing_efficiency: f64,
) -> Option<BreakEnergyResult> {
    if !bond_energy.is_finite()
        || !break_work.is_finite()
        || !processing_efficiency.is_finite()
        || bond_energy < 0.0
        || break_work < 0.0
    {
        return None;
    }

    let efficiency = processing_efficiency.clamp(0.0, 1.0);
    let net_energy = bond_energy - break_work;
    let positive_net = net_energy.max(0.0);
    let usable_energy = positive_net * efficiency;
    let heat_stress = (-net_energy).max(0.0) + (positive_net - usable_energy);

    Some(BreakEnergyResult {
        bond_energy,
        break_work,
        net_energy,
        usable_energy,
        heat_stress,
    })
}

/// Soft physiological handling of an organism's usable-energy pool.
///
/// This is deliberately parameterized. `capacity` is a physiological scale,
/// not a hard maximum. The function returns the effective amount available
/// to downstream processes and the remainder that should become stress/heat.
pub fn apply_soft_energy_capacity(
    usable_energy: f64,
    capacity: f64,
) -> Option<(f64, f64)> {
    if !usable_energy.is_finite() || !capacity.is_finite() || usable_energy < 0.0 || capacity <= 0.0 {
        return None;
    }

    let effective = usable_energy * (capacity / (capacity + usable_energy));
    let excess = usable_energy - effective;
    Some((effective, excess))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_favorable_interaction_conserves_available_energy() {
        let result = resolve_combine_energy(10.0, 4.0, 3.0, 2.0).unwrap();
        assert_eq!(result.organism_energy_deficit, 0.0);
        assert_eq!(result.interaction_surplus, 6.0);
        assert_eq!(result.heat_stress, 1.0);
        assert!((result.bond_energy + result.usable_energy + result.heat_stress - result.interaction_surplus).abs() < 1e-12);
    }

    #[test]
    fn combine_unfavorable_interaction_reports_organism_deficit() {
        let result = resolve_combine_energy(3.0, 5.0, 0.0, 0.0).unwrap();
        assert_eq!(result.organism_energy_deficit, 2.0);
        assert_eq!(result.interaction_surplus, 0.0);
        assert_eq!(result.heat_stress, 0.0);
    }

    #[test]
    fn combine_rejects_partition_that_exceeds_surplus() {
        assert!(resolve_combine_energy(10.0, 4.0, 4.0, 3.0).is_none());
    }

    #[test]
    fn break_uses_bond_energy_not_resource_potential_energy() {
        let result = resolve_break_energy(10.0, 4.0, 0.5).unwrap();
        assert_eq!(result.net_energy, 6.0);
        assert_eq!(result.usable_energy, 3.0);
        assert_eq!(result.heat_stress, 3.0);
    }

    #[test]
    fn unfavorable_break_becomes_deficit() {
        let result = resolve_break_energy(2.0, 5.0, 1.0).unwrap();
        assert_eq!(result.net_energy, -3.0);
        assert_eq!(result.usable_energy, 0.0);
        assert_eq!(result.heat_stress, 3.0);
    }

    #[test]
    fn soft_capacity_has_no_hard_cutoff() {
        let (effective, excess) = apply_soft_energy_capacity(100.0, 10.0).unwrap();
        assert!(effective > 0.0);
        assert!(effective < 100.0);
        assert!(excess > 0.0);
    }
}
