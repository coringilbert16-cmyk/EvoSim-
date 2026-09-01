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

/// Area of overlap between two circles (radii r1 and r2) whose centers
/// are `distance` apart. Pure generic geometry used by acquisition as a
/// bounded approximation of physical contact area.
pub fn circle_overlap_area(r1: f64, r2: f64, distance: f64) -> f64 {
    let r1 = r1.max(0.0);
    let r2 = r2.max(0.0);
    let distance = distance.abs();

    if r1 <= 0.0 || r2 <= 0.0 {
        return 0.0;
    }

    if distance >= r1 + r2 {
        return 0.0;
    }

    if distance <= (r1 - r2).abs() {
        return std::f64::consts::PI * r1.min(r2).powi(2);
    }

    let r1_sq = r1 * r1;
    let r2_sq = r2 * r2;
    let d = distance;

    let alpha = ((d * d + r1_sq - r2_sq) / (2.0 * d * r1))
        .clamp(-1.0, 1.0)
        .acos();
    let beta = ((d * d + r2_sq - r1_sq) / (2.0 * d * r2))
        .clamp(-1.0, 1.0)
        .acos();

    let term = (-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2);

    r1_sq * alpha + r2_sq * beta - 0.5 * term.max(0.0).sqrt()
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

    #[test]
    fn no_overlap_when_circles_are_far_apart() {
        assert_eq!(circle_overlap_area(1.0, 1.0, 5.0), 0.0);
    }

    #[test]
    fn no_overlap_exactly_at_touching_distance() {
        assert_eq!(circle_overlap_area(1.0, 2.0, 3.0), 0.0);
    }

    #[test]
    fn full_overlap_when_concentric() {
        let area = circle_overlap_area(2.0, 5.0, 0.0);
        let expected = std::f64::consts::PI * 2.0_f64.powi(2);
        assert!((area - expected).abs() < 1e-9);
    }

    #[test]
    fn full_overlap_when_one_circle_contains_the_other() {
        let area = circle_overlap_area(1.0, 10.0, 5.0);
        let expected = std::f64::consts::PI * 1.0_f64.powi(2);
        assert!((area - expected).abs() < 1e-9);
    }

    #[test]
    fn partial_overlap_is_between_zero_and_smaller_circle_area() {
        let area = circle_overlap_area(3.0, 3.0, 3.0);
        let smaller_circle_area = std::f64::consts::PI * 3.0_f64.powi(2);
        assert!(area > 0.0);
        assert!(area < smaller_circle_area);
    }

    #[test]
    fn overlap_area_decreases_monotonically_as_distance_increases() {
        let a = circle_overlap_area(2.0, 2.0, 0.5);
        let b = circle_overlap_area(2.0, 2.0, 1.5);
        let c = circle_overlap_area(2.0, 2.0, 3.0);
        assert!(a > b);
        assert!(b > c);
    }

    #[test]
    fn symmetric_in_its_two_radii() {
        let a = circle_overlap_area(1.5, 4.0, 3.0);
        let b = circle_overlap_area(4.0, 1.5, 3.0);
        assert!((a - b).abs() < 1e-9);
    }

    #[test]
    fn negative_or_zero_radius_gives_no_overlap() {
        assert_eq!(circle_overlap_area(0.0, 2.0, 0.0), 0.0);
        assert_eq!(circle_overlap_area(-1.0, 2.0, 0.0), 0.0);
    }
}
