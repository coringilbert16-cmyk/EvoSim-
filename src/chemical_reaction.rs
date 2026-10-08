//! Local runtime chemical reaction accumulation.
use crate::resources::BaseResource;
use crate::state::Organism;
use crate::structure::Bond;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChemicalBreakOperation {
    pub(crate) organism_id: String,
    pub(crate) bond: Bond,
    pub(crate) reaction_energy: f64,
    pub(crate) disruption_cost: f64,
    pub(crate) remaining_ticks: u64,
}

pub(crate) const CHEMICAL_BREAK_DURATION_TICKS: u64 = 3;

pub(crate) fn accumulate(organism: &mut Organism, catalog: &[BaseResource]) -> Vec<ChemicalBreakOperation> {
    let _ = (organism, catalog);
    Vec::new()
}

pub(crate) fn resolve(
    _operation: ChemicalBreakOperation,
    _organism: &mut Organism,
    _ledger: &mut crate::state::EnergyLedger,
) -> bool {
    false
}
