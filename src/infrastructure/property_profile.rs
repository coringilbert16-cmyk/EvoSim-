//! Opt-in timing counters for chemistry/material-property derivation.
//!
//! Enabled only when EVOSIM_PROFILE_PROPERTIES=1. The normal simulation path
//! performs no clock reads and reports nothing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

#[derive(Clone, Copy)]
#[repr(usize)]
pub(crate) enum Metric {
    CatalogBaselines = 0,
    MaterialCatalogValidation = 1,
    MaterialWeightedProperties = 2,
    MaterialMass = 3,
    StructuralUnitProperties = 4,
    WorldTonePhysicalProperties = 5,
    WorldToneStructureProperties = 6,
    StructuralUnitValidationSite = 7,
    EnvironmentalMovementValidationSite = 8,
    BroadPhaseValidationSite = 9,
    EnvironmentSnapshotValidationSite = 10,
    MaterialStorageValidationSite = 11,
}

const METRIC_COUNT: usize = 12;

struct Counter {
    calls: AtomicU64,
    nanoseconds: AtomicU64,
}

impl Counter {
    const fn new() -> Self {
        Self {
            calls: AtomicU64::new(0),
            nanoseconds: AtomicU64::new(0),
        }
    }
}

static ENABLED: OnceLock<bool> = OnceLock::new();
static COUNTERS: [Counter; METRIC_COUNT] = [
    Counter::new(),
    Counter::new(),
    Counter::new(),
    Counter::new(),
    Counter::new(),
    Counter::new(),
    Counter::new(),
];

pub(crate) struct Scope {
    metric: Metric,
    started: Instant,
}

pub(crate) fn measure_call<T>(metric: Metric, operation: impl FnOnce() -> T) -> T {
    let profile = measure(metric);
    let result = operation();
    drop(profile);
    result
}

pub(crate) fn measure(metric: Metric) -> Option<Scope> {
    if *ENABLED.get_or_init(|| {
        std::env::var_os("EVOSIM_PROFILE_PROPERTIES")
            .is_some_and(|value| value == std::ffi::OsStr::new("1"))
    }) {
        Some(Scope {
            metric,
            started: Instant::now(),
        })
    } else {
        None
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        let counter = &COUNTERS[self.metric as usize];
        counter.calls.fetch_add(1, Ordering::Relaxed);
        counter.nanoseconds.fetch_add(elapsed, Ordering::Relaxed);
    }
}

pub(crate) fn report() {
    if !*ENABLED.get_or_init(|| {
        std::env::var_os("EVOSIM_PROFILE_PROPERTIES")
            .is_some_and(|value| value == std::ffi::OsStr::new("1"))
    }) {
        return;
    }
    const NAMES: [&str; METRIC_COUNT] = [
        "catalog_baselines",
        "material_catalog_validation",
        "material_weighted_properties",
        "material_mass",
        "structural_unit_properties",
        "world_tone_physical_properties",
        "world_tone_structure_properties",
        "structural_unit_validation_site",
        "environmental_movement_validation_site",
        "broad_phase_validation_site",
        "environment_snapshot_validation_site",
        "material_storage_validation_site",
    ];
    println!("PROPERTY_PROFILE_BEGIN");
    for (name, counter) in NAMES.iter().zip(COUNTERS.iter()) {
        let calls = counter.calls.load(Ordering::Relaxed);
        let nanoseconds = counter.nanoseconds.load(Ordering::Relaxed);
        println!(
            "PROPERTY_PROFILE metric={name} calls={calls} inclusive_ms={:.3}",
            nanoseconds as f64 / 1_000_000.0
        );
    }
    println!("PROPERTY_PROFILE_END");
}
