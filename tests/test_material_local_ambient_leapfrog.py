"""Leapfrog reference for conservative material-local ambient coupling.

The discrete energy is the staggered-time invariant, not an instantaneous
continuum-energy approximation. The local velocity solve preserves reciprocal
exchange while the field retains the Phase 2 leapfrog spatial update.

This module mirrors the second-order staggered wave update used by the isolated
Phase 2 solver. It remains independent of production code and uses only the
Python standard library.
"""
from __future__ import annotations

import math
import unittest


def _lap(values, index, spacing):
    size = len(values)
    return (
        values[(index + 1) % size]
        - 2.0 * values[index]
        + values[(index - 1) % size]
    ) / spacing**2


def _world_drive(time, mass, omega):
    amplitude_rate = -0.06 * math.sin(0.4 * time)
    amplitude_acceleration = -0.024 * math.cos(0.4 * time)
    phase = omega * time + 0.2
    return mass * (
        amplitude_acceleration * math.cos(phase)
        - 2.0 * omega * amplitude_rate * math.sin(phase)
    )


def _velocity_step(vw, vq, w, q, coupling, *, step, spacing, speed, field_mass,
                   oscillator_mass, omega, source):
    """Solve the local midpoint velocity exchange in the leapfrog kick."""
    next_vw = []
    next_vq = []
    for index, strength in enumerate(coupling):
        a = step * strength / (2.0 * field_mass)
        b = step * strength / (2.0 * oscillator_mass)
        rhs_w = (
            vw[index] - a * vq[index]
            + step * speed**2 * _lap(w, index, spacing)
        )
        rhs_q = (
            vq[index] + b * vw[index] - step * omega**2 * q[index]
            + step * source / oscillator_mass
        )
        determinant = 1.0 + a * b
        next_vw.append((rhs_w - a * rhs_q) / determinant)
        next_vq.append((b * rhs_w + rhs_q) / determinant)
    return next_vw, next_vq


def _energy(w_previous, q_previous, vw, vq, w, q, *, spacing, speed,
            field_mass, oscillator_mass, omega):
    """Leapfrog invariant with cross-time potential terms."""
    total = 0.0
    size = len(w)
    for index in range(size):
        following = (index + 1) % size
        old_gradient = (w_previous[following] - w_previous[index]) / spacing
        new_gradient = (w[following] - w[index]) / spacing
        total += 0.5 * field_mass * (
            vw[index] ** 2 + speed**2 * old_gradient * new_gradient
        )
        total += 0.5 * oscillator_mass * (
            vq[index] ** 2 + omega**2 * q_previous[index] * q[index]
        )
    return spacing * total


def _run(size, step, *, final_time, profile="sharp", driven=False,
         oscillator_perturbation=False):
    """Run a periodic 1-D system with compact material-local coupling."""
    length = 16.0
    spacing = length / size
    speed, field_mass, oscillator_mass, omega = 1.0, 1.3, 0.8, 1.7
    coordinates = [index * spacing for index in range(size)]

    def smoothstep(value):
        return value * value * (3.0 - 2.0 * value)

    coupling = []
    for x in coordinates:
        if profile == "sharp":
            strength = 0.35 if 4.0 <= x < 8.0 else 0.0
        elif profile == "smooth":
            if x <= 3.5 or x >= 8.5:
                envelope = 0.0
            elif x < 4.5:
                envelope = smoothstep(x - 3.5)
            elif x <= 7.5:
                envelope = 1.0
            else:
                envelope = 1.0 - smoothstep(x - 7.5)
            strength = 0.35 * envelope
        elif profile == "vacuum":
            strength = 0.0
        elif profile == "far":
            strength = 0.35 if 8.0 <= x < 12.0 else 0.0
        elif profile == "bulk":
            strength = 0.35
        else:
            raise ValueError(f"unknown profile: {profile}")
        coupling.append(strength)

    width = 0.8
    w = [math.exp(-0.5 * ((x - 2.0) / width) ** 2) for x in coordinates]
    vw = [(x - 2.0) * value / width**2 for x, value in zip(coordinates, w)]
    q = [0.0] * size
    vq = [0.0] * size

    def source(time):
        return (
            _world_drive(time, oscillator_mass, omega) if driven else 0.0
        )

    if oscillator_perturbation:
        q = [
            0.02 * math.exp(-0.5 * ((x - 6.0) / 0.6) ** 2)
            for x in coordinates
        ]
    if driven:
        q = [1.15 * math.cos(0.2)] * size
        vq = [-1.15 * omega * math.sin(0.2)] * size

    count = round(final_time / step)
    step = final_time / count
    # Reconstruct the t=-dt/2 velocities from the initial t=0 state.
    vw_half, vq_half = _velocity_step(
        vw, vq, w, q, coupling,
        step=-0.5 * step, spacing=spacing, speed=speed,
        field_mass=field_mass, oscillator_mass=oscillator_mass,
        omega=omega, source=source(-0.25 * step),
    )
    w_previous = [value - step * velocity for value, velocity in zip(w, vw_half)]
    q_previous = [value - step * velocity for value, velocity in zip(q, vq_half)]
    initial_energy = _energy(
        w_previous, q_previous, vw_half, vq_half, w, q,
        spacing=spacing, speed=speed, field_mass=field_mass,
        oscillator_mass=oscillator_mass, omega=omega,
    )

    world_work = positive_work = negative_work = 0.0
    for step_index in range(count):
        # Leapfrog force is evaluated at t_n; adjacent half-step velocities
        # average to the oscillator velocity at that same integer time.
        applied_source = source(step_index * step)
        next_vw, next_vq = _velocity_step(
            vw_half, vq_half, w, q, coupling,
            step=step, spacing=spacing, speed=speed,
            field_mass=field_mass, oscillator_mass=oscillator_mass,
            omega=omega, source=applied_source,
        )
        next_w = [value + step * velocity for value, velocity in zip(w, next_vw)]
        next_q = [value + step * velocity for value, velocity in zip(q, next_vq)]
        work = (
            step * applied_source
            * sum(0.5 * (before + after)
                  for before, after in zip(vq_half, next_vq))
            * spacing
        )
        world_work += work
        if work >= 0.0:
            positive_work += work
        else:
            negative_work -= work
        w_previous, w = w, next_w
        q_previous, q = q, next_q
        vw_half, vq_half = next_vw, next_vq

    final_energy = _energy(
        w_previous, q_previous, vw_half, vq_half, w, q,
        spacing=spacing, speed=speed, field_mass=field_mass,
        oscillator_mass=oscillator_mass, omega=omega,
    )
    return {
        "w": w, "q": q, "initial": initial_energy, "final": final_energy,
        "work": world_work, "positive_work": positive_work,
        "negative_work": negative_work,
    }


class MaterialLocalAmbientLeapfrogTests(unittest.TestCase):
    def test_source_free_coupled_leapfrog_conserves_discrete_energy(self):
        result = _run(
            80, 0.02, final_time=10.0,
            oscillator_perturbation=True,
        )
        self.assertLess(abs(result["final"] - result["initial"]), 1.0e-11)

    def test_world_drive_work_reconciles_without_double_counting_exchange(self):
        for profile in ("vacuum", "sharp"):
            result = _run(
                80, 0.02, final_time=10.0, profile=profile, driven=True,
            )
            residual = result["final"] - result["initial"] - result["work"]
            self.assertLess(abs(residual), 1.0e-10)
            self.assertGreater(result["positive_work"], 0.0)
            self.assertGreater(result["negative_work"], 0.0)

        vacuum = _run(80, 0.02, final_time=10.0, profile="vacuum", driven=True)
        coupled = _run(80, 0.02, final_time=10.0, profile="sharp", driven=True)
        self.assertGreater(
            max(abs(a - b) for a, b in zip(vacuum["q"], coupled["q"])),
            1.0e-5,
        )

    def test_uncoupled_drive_recovers_reference_trajectory_second_order(self):
        target = lambda time: (
            1.0 + 0.15 * math.cos(0.4 * time)
        ) * math.cos(1.7 * time + 0.2)
        errors = []
        for step in (0.04, 0.02, 0.01):
            result = _run(
                80, step, final_time=10.0, profile="vacuum", driven=True,
            )
            errors.append(max(abs(value - target(10.0)) for value in result["q"]))
        self.assertGreater(errors[0] / errors[1], 3.8)
        self.assertGreater(errors[1] / errors[2], 3.8)

    def test_sharp_and_smooth_support_show_second_order_spatial_convergence(self):
        for profile in ("sharp", "smooth"):
            solutions = {
                size: _run(
                    size, 0.2 * 16.0 / size,
                    final_time=5.5, profile=profile,
                )
                for size in (32, 64, 128, 256)
            }
            reference = solutions[256]
            errors = []
            for size in (32, 64, 128):
                stride = 256 // size
                candidate = solutions[size]
                squared_error = sum(
                    (candidate["w"][index] - reference["w"][index * stride]) ** 2
                    + (candidate["q"][index] - reference["q"][index * stride]) ** 2
                    for index in range(size)
                )
                errors.append(math.sqrt(squared_error / size))
            self.assertGreater(errors[0] / errors[1], 3.0, profile)
            self.assertGreater(errors[1] / errors[2], 3.0, profile)

    def test_temporal_refinement_is_second_order_at_fixed_grid(self):
        steps = (0.2, 0.1, 0.05, 0.025, 0.0125)
        solutions = {
            step: _run(64, step, final_time=5.5)
            for step in steps
        }
        reference = solutions[steps[-1]]
        errors = []
        for step in steps[:-1]:
            candidate = solutions[step]
            squared_error = sum(
                (candidate["w"][index] - reference["w"][index]) ** 2
                + (candidate["q"][index] - reference["q"][index]) ** 2
                for index in range(64)
            )
            errors.append(math.sqrt(squared_error / 64))
        self.assertGreater(errors[0] / errors[1], 3.0)
        self.assertGreater(errors[1] / errors[2], 3.0)

    def test_empty_space_is_unchanged_before_packet_reaches_material(self):
        vacuum = _run(64, 0.025, final_time=1.0, profile="vacuum")
        localized = _run(64, 0.025, final_time=1.0, profile="far")
        bulk = _run(64, 0.025, final_time=1.0, profile="bulk")
        localized_error = max(
            abs(a - b) for a, b in zip(vacuum["w"], localized["w"])
        )
        bulk_error = max(abs(a - b) for a, b in zip(vacuum["w"], bulk["w"]))
        self.assertLess(localized_error, 1.0e-9)
        self.assertGreater(bulk_error, 1.0e-4)


if __name__ == "__main__":
    unittest.main()
