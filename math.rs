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

/// Cosine-based compatibility between two directed surface normals.
/// Returns 1 for identical direction, 0 for perpendicular, and -1
/// for opposite direction. Inputs need not already be normalized.
pub fn directional_compatibility(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let a_len = (ax * ax + ay * ay).sqrt();
    let b_len = (bx * bx + by * by).sqrt();
    if a_len <= f64::EPSILON || b_len <= f64::EPSILON {
        return 0.0;
    }
    ((ax * bx + ay * by) / (a_len * b_len)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod math_tests {
    use super::*;

    #[test]
    fn directional_compatibility_matches_cosine_geometry() {
        assert!((directional_compatibility(1.0, 0.0, 1.0, 0.0) - 1.0).abs() < 1e-12);
        assert!(directional_compatibility(1.0, 0.0, 0.0, 1.0).abs() < 1e-12);
        assert!((directional_compatibility(1.0, 0.0, -1.0, 0.0) + 1.0).abs() < 1e-12);
    }

    #[test]
    fn directional_compatibility_is_scale_invariant() {
        assert!(directional_compatibility(2.0, 0.0, 0.0, 5.0).abs() < 1e-12);
        assert!((directional_compatibility(3.0, 4.0, 6.0, 8.0) - 1.0).abs() < 1e-12);
    }
}
