/// Shared transformation math. Reactivity uses this exponential everywhere
/// (affinity, combine cost, break yield). Spec §11, locked by design answers.

pub fn complexity(n: f64) -> f64 {
    if n <= 1.0 {
        0.0
    } else {
        n * n.log2()
    }
}

/// Maps x ≥ 0 to (0, 1): 1 − e^(−x). Stable, bounded, reusable.
pub fn exponential_influence(x: f64) -> f64 {
    if x <= 0.0 {
        0.0
    } else {
        1.0 - (-x).exp()
    }
}

/// Signed version for property deviations.
pub fn signed_exponential(x: f64) -> f64 {
    let sign = if x < 0.0 {
        -1.0
    } else if x > 0.0 {
        1.0
    } else {
        0.0
    };
    sign * exponential_influence(x.abs())
}
