//! Opt-in process-local profiling for static-potential cache behavior.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

static ENABLED: OnceLock<bool> = OnceLock::new();
static HITS: AtomicU64 = AtomicU64::new(0);
static MISSES: AtomicU64 = AtomicU64::new(0);
static CALCULATIONS: AtomicU64 = AtomicU64::new(0);
static CALCULATION_NS: AtomicU64 = AtomicU64::new(0);
static COMBINE_CALLS: AtomicU64 = AtomicU64::new(0);
static COMBINE_NS: AtomicU64 = AtomicU64::new(0);

fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var_os("EVOSIM_CHEMISTRY_CACHE_METRICS").is_some())
}

pub(crate) fn record_lookup(hit: bool) {
    if enabled() {
        if hit {
            HITS.fetch_add(1, Ordering::Relaxed);
        } else {
            MISSES.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub(crate) fn calculate_static_potential(a: f64, b: f64) -> Option<f64> {
    if !enabled() {
        return crate::chemistry::interaction_potential(
            a,
            b,
            crate::chemistry::CHEMICAL_K,
            crate::chemistry::CHEMICAL_D_MAX,
        );
    }

    let started = std::time::Instant::now();
    let result = crate::chemistry::interaction_potential(
        a,
        b,
        crate::chemistry::CHEMICAL_K,
        crate::chemistry::CHEMICAL_D_MAX,
    );
    CALCULATIONS.fetch_add(1, Ordering::Relaxed);
    CALCULATION_NS.fetch_add(
        started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
        Ordering::Relaxed,
    );
    result
}

pub(crate) fn record_combine_discovery(duration: Duration) {
    if enabled() {
        COMBINE_CALLS.fetch_add(1, Ordering::Relaxed);
        COMBINE_NS.fetch_add(
            duration.as_nanos().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
    }
}

pub(crate) fn summary_json() -> String {
    format!(
        r#"{{"cache_hits":{},"cache_misses":{},"potential_calculations":{},"potential_calculation_ns":{},"combine_discovery_calls":{},"combine_discovery_ns":{}}}"#,
        HITS.load(Ordering::Relaxed),
        MISSES.load(Ordering::Relaxed),
        CALCULATIONS.load(Ordering::Relaxed),
        CALCULATION_NS.load(Ordering::Relaxed),
        COMBINE_CALLS.load(Ordering::Relaxed),
        COMBINE_NS.load(Ordering::Relaxed),
    )
}
