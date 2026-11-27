# Closed Wave Field — Local Query Contract

**Status:** Initial chemistry-facing diagnostic contract; isolated reference only. This contract does not authorize material energy transfer or production-tick integration.

## Position

Chemistry-facing callers use world-space coordinates in project length units (L), with the origin at the lower-left grid cell. The valid closed domain is:

- (0 \le x \le (width - 1) \times spacing)
- (0 \le y \le (height - 1) \times spacing)

The implementation converts world coordinates to grid coordinates internally. Non-finite and out-of-domain positions fail closed. The lower-level grid-coordinate query remains available for solver tests; callers should prefer `query_world_at_current_time`.

## Time-centering

A query is a snapshot of the latest completed leapfrog state, not a request for an arbitrary historical or future time.

- Displacement, spatial gradient, energy density, and flux are reconstructed from the current displacement state. Their displacement timestamp is `displacement_time_ticks`.
- The time derivative is reconstructed from the current/previous displacement difference. It is centred at the intervening half-step; its timestamp is `velocity_time_ticks`.
- Both timestamps are returned explicitly. Consumers must not silently treat these quantities as a fully synchronized sample.
- To query another time, the field must first be advanced to that state or a separately validated temporal reconstruction must be implemented. No extrapolation is implied.

## Returned quantities

The sample reports displacement, time derivative, spatial gradient, pointwise energy density, and directional energy flux. Gradients are in displacement per L; timestamps are in logical ticks. Energy density and flux use the solver's normalization and propagation-speed parameters.

Reflecting boundaries impose zero normal gradient/flux. Interpolation is deterministic and bilinear over the stored grid values and node-centred finite-difference gradients.

## Accounting boundary

The returned pointwise energy density is a local continuum diagnostic. Its spatial sum is **not** the exact discrete leapfrog invariant. The invariant remains available through the field-wide `energy()` method.

A query is read-only and never reserves, debits, or transfers energy. In particular, multiplying pointwise density by an assumed chemical area/volume does not establish a conservative withdrawable amount. Field-to-matter transfer requires a separately defined discrete interaction operator whose field debit exactly matches the receiver credit and preserves the closed-field energy balance.

## Scope

This contract remains isolated from the production simulation tick, chemistry, material geometry, ambient composition, and energy settlement. It is the read interface needed to design the transfer operator; it is not that operator.
