#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::decision::{DecisionHistory, DecisionParameters};
use crate::decomposition::DecomposingBody;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::environment::ActiveMaterialField;
use crate::genome::Genome;
use crate::material_storage::MaterialStorage;
use crate::resources::{BaseResource, Material};
use crate::structure::{Bond, OrganismStructure};
use crate::runtime::SimulationProcess;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Clone)]