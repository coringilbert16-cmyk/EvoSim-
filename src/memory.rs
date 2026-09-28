
    raw.tanh()
}

/// Record one completed experience against all spatially attributed
/// resonance signals involved in it. Location and spectral associations are
/// reinforced independently; the encounter record keeps the linkage.
pub(crate) fn memory_consequence_from_action(
    consequence: crate::decision::ActionConsequence,
) -> MemoryConsequence {
    MemoryConsequence {
        energy_delta: consequence.energy_delta,
        stress_delta: consequence.stress_delta,
        damage_delta: 0.0,
        developmental_delta: consequence.developmental_delta,
        material_acquired: 0.0,
        material_consumed: 0.0,
        material_transformed: 0.0,
    }
}

pub(crate) fn record_experience(
    memory: &mut ExperienceMemory,
    perceptions: &[crate::harmonics::ResonancePerception],
    action: crate::decision::ActionKind,