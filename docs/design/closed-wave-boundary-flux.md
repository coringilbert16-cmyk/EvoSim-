# Closed wave-field boundary flux measurement

**Status:** Geometry-to-field measurement adapter implemented for supported polygonal straight receiver intervals; still isolated from production chemistry and live ticks.

## Contract

The geometry/visibility layer remains authoritative for identifying exact, disjoint exposed straight boundary intervals. The flux measurement accepts those intervals and evaluates the inward normal energy flux

`P_in(t) = ∫_Γ max(0, -J(x,t) · n(x)) dℓ`

using three-point Gauss–Legendre quadrature on each interval. The outward normal must be finite and unit length, the interval must be non-degenerate, and the normal must be perpendicular to the segment. Collinear overlapping intervals are rejected to guard against duplicate counting. Any field sample outside the finite solver domain fails closed.

The time-domain energy over an interaction window is obtained from consecutive substep power samples using the trapezoidal rule. Samples use the leapfrog velocity time centre and must have strictly increasing timestamps. Callers must sample each solver substep; sparse sampling is not silently extrapolated.

## Time-centred flux

The leapfrog solver stores displacement at integer time centres and velocity at half-step centres. The previous point-query implementation multiplied half-step velocity by the gradient at the integer displacement centre. The query now also reconstructs the spatial gradient at the half-step centre by averaging gradients from the previous and current displacement states. The flux diagnostic uses that synchronized gradient:

`J^(n-1/2) = -K c² u_t^(n-1/2) ∇u^(n-1/2)`.

The displacement-centred gradient remains available separately for pointwise energy-density diagnostics.

## Scope and limitations

The module now has two layers: a low-level measurement kernel over supplied intervals, and `incoming_power_for_visible_polygonal_targets`, which calls the existing `visible_polygonal_intervals_for_targets` geometry authority and passes its exact exposed intervals to the field sampler. The adapter keeps all bodies available as occluders while restricting measured intervals to the selected receiver indices. It does not:
- replace or duplicate geometry/visibility calculations;
- support curved receiver boundaries;
- establish that every production geometry arrangement is representable;
- derive a spectral decomposition or material response coefficients;
- withdraw field energy or credit reaction work;
- connect to chemistry or the production tick.

The finite solver's world coordinates currently use the lower-left grid cell as the origin. Intervals must already be expressed in that same world coordinate frame and lie inside the solver domain. The adapter is deliberately limited to the polygonal straight-interval output currently supported by the visibility API. Curved receiver intervals need a separate exact arc integration path before those shapes can participate.

## Validation

Tests cover exact integration of a linear inward-flux profile, a receiver rectangle passed through the actual geometry visibility API with an analytically predictable incoming-power result, duplicate/degenerate interval rejection, disjoint collinear interval acceptance, and trapezoidal integration of linear power in time. Full solver and chemistry CI remains the acceptance gate.
