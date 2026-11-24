"""Leapfrog reference for conservative material-local ambient coupling.

The discrete energy is the staggered-time invariant, not an instantaneous
continuum-energy approximation. The local velocity solve preserves reciprocal
exchange while the field retains the Phase 2 leapfrog spatial update.

World-drive work is paired with the same integer-time force and centered
oscillator velocity used by the staggered update.

This module mirrors the second-order staggered wave update used by the isolated
Phase 2 solver. It remains independent of production code and uses only the
Python standard library.
"""
from __future__ import annotations

import cmath
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




def _patch_energy(w_previous, q_previous, vw, vq, w, q, start, end,
                  *, spacing, speed, field_mass, oscillator_mass, omega):
    """Patch inventory with half of each adjacent cross-time gradient edge."""
    total = 0.0
    size = len(w)
    for index in range(start, end + 1):
        following = (index + 1) % size
        previous = (index - 1) % size
        right_old = (w_previous[following] - w_previous[index]) / spacing
        right_new = (w[following] - w[index]) / spacing
        left_old = (w_previous[index] - w_previous[previous]) / spacing
        left_new = (w[index] - w[previous]) / spacing
        total += 0.5 * field_mass * vw[index] ** 2
        total += 0.25 * field_mass * speed**2 * (
            right_old * right_new + left_old * left_new
        )
        total += 0.5 * oscillator_mass * (
            vq[index] ** 2
            + omega**2 * q_previous[index] * q[index]
        )
    return spacing * total


def _face_flux(w, vw_before, vw_after, face, *, spacing, speed, field_mass):
    """Centered staggered-energy flux through one periodic grid face."""
    size = len(w)
    following = (face + 1) % size
    gradient = (w[following] - w[face]) / spacing
    velocity_here = 0.5 * (vw_before[face] + vw_after[face])
    velocity_next = 0.5 * (
        vw_before[following] + vw_after[following]
    )
    return -field_mass * speed**2 * gradient * 0.5 * (
        velocity_here + velocity_next
    )



def _laplacian_2d(values, x, y, spacing):
    size = len(values)
    return (
        values[(x + 1) % size][y] - 2.0 * values[x][y]
        + values[(x - 1) % size][y]
        + values[x][(y + 1) % size] - 2.0 * values[x][y]
        + values[x][(y - 1) % size]
    ) / spacing**2


def _velocity_step_2d(vw, vq, w, q, coupling, *, step, spacing, speed,
                      field_mass, oscillator_mass, omega, source):
    size = len(w)
    next_vw = [[0.0] * size for _ in range(size)]
    next_vq = [[0.0] * size for _ in range(size)]
    for x in range(size):
        for y in range(size):
            strength = coupling[x][y]
            a = step * strength / (2.0 * field_mass)
            b = step * strength / (2.0 * oscillator_mass)
            rhs_w = (
                vw[x][y] - a * vq[x][y]
                + step * speed**2 * _laplacian_2d(w, x, y, spacing)
            )
            rhs_q = (
                vq[x][y] + b * vw[x][y] - step * omega**2 * q[x][y]
                + step * source / oscillator_mass
            )
            determinant = 1.0 + a * b
            next_vw[x][y] = (rhs_w - a * rhs_q) / determinant
            next_vq[x][y] = (b * rhs_w + rhs_q) / determinant
    return next_vw, next_vq


def _patch_energy_2d(w_previous, q_previous, vw, vq, w, q, patch,
                     *, spacing, speed, field_mass, oscillator_mass, omega):
    x0, y0, x1, y1 = patch
    size = len(w)
    total = 0.0
    for x in range(x0, x1 + 1):
        for y in range(y0, y1 + 1):
            xp, xm = (x + 1) % size, (x - 1) % size
            yp, ym = (y + 1) % size, (y - 1) % size
            gradients = (
                (w_previous[xp][y] - w_previous[x][y]) / spacing,
                (w[xp][y] - w[x][y]) / spacing,
                (w_previous[x][y] - w_previous[xm][y]) / spacing,
                (w[x][y] - w[xm][y]) / spacing,
                (w_previous[x][yp] - w_previous[x][y]) / spacing,
                (w[x][yp] - w[x][y]) / spacing,
                (w_previous[x][y] - w_previous[x][ym]) / spacing,
                (w[x][y] - w[x][ym]) / spacing,
            )
            total += 0.5 * field_mass * vw[x][y] ** 2
            total += 0.25 * field_mass * speed**2 * sum(
                gradients[index] * gradients[index + 1]
                for index in (0, 2, 4, 6)
            )
            total += 0.5 * oscillator_mass * (
                vq[x][y] ** 2
                + omega**2 * q_previous[x][y] * q[x][y]
            )
    return spacing**2 * total


def _face_flux_x_2d(w, vw_before, vw_after, face, y, *, spacing,
                    speed, field_mass):
    size = len(w)
    following = (face + 1) % size
    gradient = (w[following][y] - w[face][y]) / spacing
    velocity_here = 0.5 * (
        vw_before[face][y] + vw_after[face][y]
    )
    velocity_next = 0.5 * (
        vw_before[following][y] + vw_after[following][y]
    )
    return -field_mass * speed**2 * gradient * 0.5 * (
        velocity_here + velocity_next
    ) * spacing


def _face_flux_y_2d(w, vw_before, vw_after, x, face, *, spacing,
                    speed, field_mass):
    size = len(w)
    following = (face + 1) % size
    gradient = (w[x][following] - w[x][face]) / spacing
    velocity_here = 0.5 * (
        vw_before[x][face] + vw_after[x][face]
    )
    velocity_next = 0.5 * (
        vw_before[x][following] + vw_after[x][following]
    )
    return -field_mass * speed**2 * gradient * 0.5 * (
        velocity_here + velocity_next
    ) * spacing


def _run_2d_patch_balance():
    size, spacing, step, steps = 24, 1.0, 0.1, 60
    speed, field_mass, oscillator_mass, omega = 1.0, 1.3, 0.8, 1.7
    coupling = [
        [
            0.35 if 8 <= x < 13 and 8 <= y < 13 else 0.0
            for y in range(size)
        ]
        for x in range(size)
    ]
    w = [
        [
            math.exp(
                -0.5 * (((x - 8.0) / 2.0) ** 2 + ((y - 8.0) / 2.0) ** 2)
            )
            for y in range(size)
        ]
        for x in range(size)
    ]
    q = [[0.0] * size for _ in range(size)]
    vw = [[0.0] * size for _ in range(size)]
    vq = [[0.0] * size for _ in range(size)]

    def drive(time):
        return _world_drive(time, oscillator_mass, omega)

    vw_half, vq_half = _velocity_step_2d(
        vw, vq, w, q, coupling, step=-0.5 * step, spacing=spacing,
        speed=speed, field_mass=field_mass, oscillator_mass=oscillator_mass,
        omega=omega, source=drive(-0.25 * step),
    )
    w_previous = [
        [w[x][y] - step * vw_half[x][y] for y in range(size)]
        for x in range(size)
    ]
    q_previous = [
        [q[x][y] - step * vq_half[x][y] for y in range(size)]
        for x in range(size)
    ]
    patches = ((3, 3, 12, 12), (5, 5, 18, 18), (0, 0, 23, 23))
    initial = [
        _patch_energy_2d(
            w_previous, q_previous, vw_half, vq_half, w, q, patch,
            spacing=spacing, speed=speed, field_mass=field_mass,
            oscillator_mass=oscillator_mass, omega=omega,
        )
        for patch in patches
    ]
    flux = [0.0] * len(patches)
    work = [0.0] * len(patches)

    for step_index in range(steps):
        source = drive(step_index * step)
        next_vw, next_vq = _velocity_step_2d(
            vw_half, vq_half, w, q, coupling, step=step, spacing=spacing,
            speed=speed, field_mass=field_mass, oscillator_mass=oscillator_mass,
            omega=omega, source=source,
        )
        next_w = [
            [w[x][y] + step * next_vw[x][y] for y in range(size)]
            for x in range(size)
        ]
        next_q = [
            [q[x][y] + step * next_vq[x][y] for y in range(size)]
            for x in range(size)
        ]

        for patch_index, (x0, y0, x1, y1) in enumerate(patches):
            centered_q_velocity = sum(
                0.5 * (vq_half[x][y] + next_vq[x][y])
                for x in range(x0, x1 + 1)
                for y in range(y0, y1 + 1)
            )
            work[patch_index] += (
                step * spacing**2 * source * centered_q_velocity
            )
            outward = 0.0
            for y in range(y0, y1 + 1):
                outward += (
                    _face_flux_x_2d(
                        w, vw_half, next_vw, x1, y, spacing=spacing,
                        speed=speed, field_mass=field_mass,
                    )
                    - _face_flux_x_2d(
                        w, vw_half, next_vw, x0 - 1, y, spacing=spacing,
                        speed=speed, field_mass=field_mass,
                    )
                )
            for x in range(x0, x1 + 1):
                outward += (
                    _face_flux_y_2d(
                        w, vw_half, next_vw, x, y1, spacing=spacing,
                        speed=speed, field_mass=field_mass,
                    )
                    - _face_flux_y_2d(
                        w, vw_half, next_vw, x, y0 - 1, spacing=spacing,
                        speed=speed, field_mass=field_mass,
                    )
                )
            flux[patch_index] += step * outward

        w_previous, w = w, next_w
        q_previous, q = q, next_q
        vw_half, vq_half = next_vw, next_vq

    residuals = []
    for patch_index, patch in enumerate(patches):
        final = _patch_energy_2d(
            w_previous, q_previous, vw_half, vq_half, w, q, patch,
            spacing=spacing, speed=speed, field_mass=field_mass,
            oscillator_mass=oscillator_mass, omega=omega,
        )
        residuals.append(
            final - initial[patch_index] + flux[patch_index] - work[patch_index]
        )
    return residuals




def _continuum_scattering(*, length, omega=1.0, ambient_omega=1.7,
                           coupling_strength=0.35, field_mass=1.3,
                           oscillator_mass=0.8, speed=1.0):
    """Exact plane-wave scattering by a uniform material slab."""
    vacuum_k = omega / speed
    material_k = vacuum_k * math.sqrt(
        1.0 + coupling_strength**2
        / (
            field_mass * oscillator_mass
            * (ambient_omega**2 - omega**2)
        )
    )
    cosine = math.cos(material_k * length)
    sine = math.sin(material_k * length)
    transfer_a = cosine
    transfer_b = sine / material_k
    transfer_c = -material_k * sine
    transfer_d = cosine
    coefficient_a = transfer_c - 1j * vacuum_k * transfer_a
    coefficient_b = 1j * vacuum_k * (
        transfer_d - 1j * vacuum_k * transfer_b
    )
    reflection = -(coefficient_a + coefficient_b) / (
        coefficient_a - coefficient_b
    )
    transmission = (
        transfer_a * (1.0 + reflection)
        + transfer_b * 1j * vacuum_k * (1.0 - reflection)
    )
    return reflection, transmission


def _discrete_scattering(*, spacing, step, length, omega=1.0,
                         ambient_omega=1.7, coupling_strength=0.35,
                         field_mass=1.3, oscillator_mass=0.8, speed=1.0):
    """Leapfrog/FDTD slab scattering from its exact spatial recurrence.

    Material nodes are centered between vacuum nodes, placing both interfaces
    on half-grid faces and avoiding a first-order interface-position offset.
    """
    temporal_phase = omega * step
    omega_tilde = 2.0 * math.sin(temporal_phase / 2.0) / step
    omega_centered = math.sin(temporal_phase) / step
    material_potential = (
        omega_tilde**2
        + coupling_strength**2 * omega_centered**2
        / (
            field_mass * oscillator_mass
            * (ambient_omega**2 - omega_tilde**2)
        )
    )
    vacuum_k = 2.0 / spacing * math.asin(
        spacing * omega_tilde / (2.0 * speed)
    )
    count = round(length / spacing)
    recurrence_a = 2.0 - spacing**2 * material_potential / speed**2

    # Transfer across count material nodes. The two interfaces lie halfway
    # between the first/last material nodes and the adjacent vacuum nodes.
    p00, p01, p10, p11 = 1.0, 0.0, 0.0, 1.0
    for _ in range(count):
        p00, p01, p10, p11 = (
            recurrence_a * p00 - p10,
            recurrence_a * p01 - p11,
            p00,
            p01,
        )

    phase = cmath.exp(-1j * vacuum_k * spacing)
    half_phase = cmath.exp(-0.5j * vacuum_k * spacing)
    coefficient_a = p10 - phase * p00
    coefficient_b = p11 - phase * p01
    reflection = -(
        coefficient_a / half_phase + coefficient_b * half_phase
    ) / (
        coefficient_a * half_phase + coefficient_b / half_phase
    )
    transmission = half_phase * (
        p00 * (1.0 / half_phase + reflection * half_phase)
        + p01 * (half_phase + reflection / half_phase)
    )
    return reflection, transmission


class MaterialLocalAmbientLeapfrogTests(unittest.TestCase):
    def test_sharp_interface_scattering_converges_to_continuum(self):
        reference = _continuum_scattering(length=4.0)
        errors = []
        for spacing in (0.4, 0.2, 0.1, 0.05, 0.025):
            reflection, transmission = _discrete_scattering(
                spacing=spacing, step=0.2 * spacing, length=4.0,
            )
            errors.append(
                abs(reflection - reference[0])
                + abs(transmission - reference[1])
            )
        for coarse, fine in zip(errors, errors[1:]):
            self.assertGreater(coarse / fine, 3.8)
        self.assertGreater(abs(reference[0]), 1.0e-3)

    def test_sharp_interface_scattering_has_second_order_temporal_refinement(self):
        spacing = 0.05
        reference = _discrete_scattering(
            spacing=spacing, step=1.0e-5, length=4.0,
        )
        errors = []
        for step in (0.05, 0.025, 0.0125, 0.00625):
            reflection, transmission = _discrete_scattering(
                spacing=spacing, step=step, length=4.0,
            )
            errors.append(
                abs(reflection - reference[0])
                + abs(transmission - reference[1])
            )
        for coarse, fine in zip(errors, errors[1:]):
            self.assertGreater(coarse / fine, 3.9)

    def test_uncoupled_scattering_has_no_reflection(self):
        continuum = _continuum_scattering(
            length=4.0, coupling_strength=0.0,
        )
        self.assertLess(abs(continuum[0]), 1.0e-12)
        self.assertLess(
            abs(continuum[1] - cmath.exp(4.0j)),
            1.0e-12,
        )
        spacing, step = 0.05, 0.01
        discrete = _discrete_scattering(
            spacing=spacing, step=step, length=4.0,
            coupling_strength=0.0,
        )
        omega_tilde = 2.0 * math.sin(step / 2.0) / step
        vacuum_k = 2.0 / spacing * math.asin(
            spacing * omega_tilde / 2.0
        )
        expected = cmath.exp(1j * vacuum_k * 4.0)
        self.assertLess(abs(discrete[0]), 1.0e-10)
        self.assertLess(abs(discrete[1] - expected), 1.0e-10)

    def test_two_dimensional_five_point_update_closes_nested_patch_balances(self):
        cfl_number = math.sqrt(2.0) * 0.1
        self.assertLess(cfl_number, 1.0)
        for residual in _run_2d_patch_balance():
            self.assertLess(abs(residual), 1.0e-10)

    def test_nested_patches_reconcile_staggered_flux_and_world_work(self):
        size = 80
        step = 0.02
        spacing = 1.0
        speed, field_mass, oscillator_mass, omega = 1.0, 1.3, 0.8, 1.7
        coordinates = [index * spacing for index in range(size)]
        coupling = [
            0.35 if 30.0 <= coordinate < 40.0 else 0.0
            for coordinate in coordinates
        ]
        displacement = [
            math.exp(-0.5 * ((coordinate - 18.0) / 3.0) ** 2)
            for coordinate in coordinates
        ]
        oscillator = [0.0] * size
        field_velocity = [0.0] * size
        oscillator_velocity = [0.0] * size

        def drive(time):
            return _world_drive(time, oscillator_mass, omega)

        field_half, oscillator_half = _velocity_step(
            field_velocity, oscillator_velocity, displacement, oscillator,
            coupling, step=-0.5 * step, spacing=spacing, speed=speed,
            field_mass=field_mass, oscillator_mass=oscillator_mass,
            omega=omega, source=drive(-0.25 * step),
        )
        previous_displacement = [
            value - step * velocity
            for value, velocity in zip(displacement, field_half)
        ]
        previous_oscillator = [
            value - step * velocity
            for value, velocity in zip(oscillator, oscillator_half)
        ]
        patches = ((5, 35), (10, 50), (0, size - 1))
        initial = [
            _patch_energy(
                previous_displacement, previous_oscillator,
                field_half, oscillator_half, displacement, oscillator,
                start, end, spacing=spacing, speed=speed,
                field_mass=field_mass, oscillator_mass=oscillator_mass,
                omega=omega,
            )
            for start, end in patches
        ]
        cumulative_flux = [0.0] * len(patches)
        cumulative_work = [0.0] * len(patches)

        for step_index in range(80):
            source = drive(step_index * step)
            next_field_half, next_oscillator_half = _velocity_step(
                field_half, oscillator_half, displacement, oscillator,
                coupling, step=step, spacing=spacing, speed=speed,
                field_mass=field_mass, oscillator_mass=oscillator_mass,
                omega=omega, source=source,
            )
            next_displacement = [
                value + step * velocity
                for value, velocity in zip(displacement, next_field_half)
            ]
            next_oscillator = [
                value + step * velocity
                for value, velocity in zip(oscillator, next_oscillator_half)
            ]

            for patch_index, (start, end) in enumerate(patches):
                centered_oscillator_velocity = sum(
                    0.5 * (before + after)
                    for before, after in zip(
                        oscillator_half[start : end + 1],
                        next_oscillator_half[start : end + 1],
                    )
                )
                cumulative_work[patch_index] += (
                    step * spacing * source * centered_oscillator_velocity
                )
                right_flux = _face_flux(
                    displacement, field_half, next_field_half, end,
                    spacing=spacing, speed=speed, field_mass=field_mass,
                )
                left_flux = _face_flux(
                    displacement, field_half, next_field_half, start - 1,
                    spacing=spacing, speed=speed, field_mass=field_mass,
                )
                cumulative_flux[patch_index] += step * (
                    right_flux - left_flux
                )

            previous_displacement, displacement = (
                displacement, next_displacement
            )
            previous_oscillator, oscillator = oscillator, next_oscillator
            field_half, oscillator_half = next_field_half, next_oscillator_half

        for patch_index, (start, end) in enumerate(patches):
            final = _patch_energy(
                previous_displacement, previous_oscillator,
                field_half, oscillator_half, displacement, oscillator,
                start, end, spacing=spacing, speed=speed,
                field_mass=field_mass, oscillator_mass=oscillator_mass,
                omega=omega,
            )
            residual = (
                final - initial[patch_index]
                + cumulative_flux[patch_index]
                - cumulative_work[patch_index]
            )
            scale = max(
                abs(final - initial[patch_index]),
                abs(cumulative_flux[patch_index]),
                abs(cumulative_work[patch_index]),
                1.0e-12,
            )
            self.assertLess(abs(residual) / scale, 1.0e-11)

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
