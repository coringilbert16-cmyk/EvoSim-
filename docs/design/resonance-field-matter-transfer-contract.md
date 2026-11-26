# Closed Wave Field — Field/Matter Transfer Contract

**Status:** Isolated reference implementation and deterministic settlement tests. This is not production chemistry integration.

## Interaction rule

A transfer is a localized, phase-sensitive impulse at a world-space interaction position. The position is mapped to the four surrounding field cells using the same deterministic bilinear weights as the read query. The impulse adds to the current displacement state and subtracts from the previous displacement state, changing the leapfrog velocity while preserving the midpoint displacement.

This is a discrete interaction event, not a withdrawal computed from pointwise continuum energy density. The local field phase and velocity determine whether the requested field-to-matter transfer is available.

## Exact settlement

For a fixed interaction position, the discrete field energy after an impulse amplitude (a) is quadratic:

`E(a) - E(0) = A a^2 + B a`

The implementation evaluates the discrete leapfrog invariant at amplitudes (+1) and (-1) to determine (A) and (B), then solves for an impulse that produces the requested signed energy change.

- **Field to matter:** solve `A a² + B a = -Q`. If the local phase cannot supply `Q`, reject the transaction without changing either side.
- **Matter to field:** solve `A a² + B a = +Q`. Debit matter only after the field's measured invariant increases by `Q`.
- The field energy change is measured using the same closed-domain discrete invariant as the solver's `energy()` method.
- Settlement is atomic: failed validation or an unavailable transfer leaves both field and matter unchanged.
- Sequential interactions always see the updated field state. Overlapping receivers cannot spend the same field energy twice.

## Accounting boundary

For every successful transaction:

`field_energy_before + matter_energy_before = field_energy_after + matter_energy_after`

within the documented floating-point settlement tolerance.

The matter account in the isolated tests is only a minimal receiver/releaser used to prove the interface. It does not replace chemistry's structural-energy accounting or staged environmental funding contract.

## Scope and limitations

- The field remains detached from the production simulation tick.
- No chemistry, material geometry, organism rules, ambient composition, or global usable-energy pool is connected.
- The initial interaction footprint is a point-local bilinear stencil, not a chemical structure geometry model.
- Transfer is phase-sensitive; a positive pointwise energy-density sample alone does not guarantee that an arbitrary requested transfer is available at that position.
- This first contract validates a single discrete interaction channel. Broader footprints, multiple independent interaction modes, and structure-specific coupling must be justified by subsequent tests rather than assumed.
