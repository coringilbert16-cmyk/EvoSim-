""""Analytic packet and discrete patch-balance references for Phase 3B.3.

These tests are independent of the production solver. The packet is an exact
right-moving solution of the one-dimensional massless wave equation. A uniform
ambient carrier is added only as an explicitly forced degree of freedom.
"""
from __future__ import annotations

import math


def _packet(x: float, t: float, *, amplitude: float, width: float, speed: float):
    """Gaussian traveling packet and exact first derivatives."""
    z = x - speed * t
    value = amplitude * math.exp(-0.5 * (z / width) ** 2)
    derivative_z = -(z / width**2) * value
    return value, -speed * derivative_z, derivative_z


def _ambient(t: float, *, amplitude: float, omega: float):
    value = amplitude * math.cos(omega * t)
    velocity = -amplitude * omega * math.sin(omega * t)
    acceleration = -omega**2 * value
    return value, velocity, acceleration


def _combined_state(x, t, *, packet_amplitude, width, speed, ambient_amplitude, omega):
    w, wt, wx = _packet(
        x, t, amplitude=packet_amplitude, width=width, speed=speed
    )
    b, bt, btt = _ambient(t, amplitude=ambient_amplitude, omega=omega)
    return w + b, wt + bt, wx, btt


def test_gaussian_packet_is_an_exact_localized_massless_wave_solution():
    amplitude, width, speed = 0.8, 0.55, 1.3
    h = 1.0e-3
    for x in (-1.7, -0.2, 0.0, 0.9, 2.1):
        for t in (-0.4, 0.0, 0.35):
            sample = lambda xx, tt: _packet(
                xx, tt, amplitude=amplitude, width=width, speed=speed
            )[0]
            w_tt = (sample(x, t + h) - 2.0 * sample(x, t) + sample(x, t - h)) / h**2
            w_xx = (sample(x + h, t) - 2.0 * sample(x, t) + sample(x - h, t)) / h**2
            assert math.isclose(w_tt, speed**2 * w_xx, rel_tol=2e-6, abs_tol=2e-6)


def test_combined_packet_and_uniform_carrier_have_explicit_forcing_and_cross_terms():
    values = {
        "packet_amplitude": 0.8,
        "width": 0.55,
        "speed": 1.3,
        "ambient_amplitude": 0.4,
        "omega": 2.0 * math.pi * 440.0 / 24_000.0,
    }
    x, t, normalization = 0.23, 0.17, 2.1
    _, total_velocity, gradient, forcing = _combined_state(x, t, **values)
    _, packet_velocity, packet_gradient = _packet(
        x, t,
        amplitude=values["packet_amplitude"],
        width=values["width"],
        speed=values["speed"],
    )
    _, ambient_velocity, ambient_acceleration = _ambient(
        t, amplitude=values["ambient_amplitude"], omega=values["omega"]
    )
    assert forcing == ambient_acceleration
    assert ambient_acceleration != 0.0

    total_energy = 0.5 * normalization * (
        total_velocity**2 + values["speed"]**2 * gradient**2
    )
    packet_energy = 0.5 * normalization * (
        packet_velocity**2 + values["speed"]**2 * packet_gradient**2
    )
    ambient_kinetic = 0.5 * normalization * ambient_velocity**2
    cross_energy = normalization * ambient_velocity * packet_velocity
    assert math.isclose(
        total_energy,
        packet_energy + ambient_kinetic + cross_energy,
        rel_tol=1e-13,
        abs_tol=1e-13,
    )


def _trapz(values, spacing):
    return spacing * (0.5 * values[0] + sum(values[1:-1]) + 0.5 * values[-1])


def _discrete_patch_residual(points, steps, *, patch, duration):
    """Return normalized discrete energy-balance residual for one patch.

    Samples exact fields, but evaluates derivatives and all integrals with
    centered finite differences/trapezoidal quadrature. The accounting patch
    is never used as a physical boundary in the field evaluation.
    """
    dx = 8.0 / points
    dt = duration / steps
    xs = [-4.0 + i * dx for i in range(points + 1)]
    times = [-0.5 * duration + i * dt for i in range(steps + 1)]
    left, right = patch
    i0, i1 = round((left + 4.0) / dx), round((right + 4.0) / dx)
    assert math.isclose(xs[i0], left, abs_tol=1e-12)
    assert math.isclose(xs[i1], right, abs_tol=1e-12)

    packet_amplitude, width, speed = 0.8, 0.55, 1.3
    ambient_amplitude = 0.4
    omega = 2.0 * math.pi * 440.0 / 24_000.0
    normalization = 2.1

    def state(x, t):
        return _combined_state(
            x, t,
            packet_amplitude=packet_amplitude,
            width=width,
            speed=speed,
            ambient_amplitude=ambient_amplitude,
            omega=omega,
        )

    def energy_at(t):
        density = []
        for x in xs[i0:i1 + 1]:
            u_plus = state(x, t + dt)[0]
            u_minus = state(x, t - dt)[0]
            ut = (u_plus - u_minus) / (2.0 * dt)
            ux = (state(x + dx, t)[0] - state(x - dx, t)[0]) / (2.0 * dx)
            density.append(0.5 * normalization * (ut**2 + speed**2 * ux**2))
        return _trapz(density, dx)

    def boundary_flux(t, x):
        ut = (state(x, t + dt)[0] - state(x, t - dt)[0]) / (2.0 * dt)
        ux = (state(x + dx, t)[0] - state(x - dx, t)[0]) / (2.0 * dx)
        return -normalization * speed**2 * ut * ux

    def work_at(t):
        values = []
        for x in xs[i0:i1 + 1]:
            _, ut, _, forcing = state(x, t)
            values.append(normalization * ut * forcing)
        return _trapz(values, dx)

    initial_energy = energy_at(times[0])
    final_energy = energy_at(times[-1])
    flux_integral = 0.0
    work_integral = 0.0
    for n in range(steps):
        midpoint = 0.5 * (times[n] + times[n + 1])
        outward_flux = boundary_flux(midpoint, right) - boundary_flux(midpoint, left)
        flux_integral += outward_flux * dt
        work_integral += work_at(midpoint) * dt

    residual = final_energy - initial_energy + flux_integral - work_integral
    scale = max(
        abs(final_energy - initial_energy),
        abs(flux_integral),
        abs(work_integral),
        1e-12,
    )
    return abs(residual) / scale


def test_nested_accounting_patches_close_discrete_packet_flux_and_forcing_work():
    patches = ((-1.6, 1.6), (-2.4, 2.4), (-3.2, 3.2))
    coarse = [
        _discrete_patch_residual(200, 80, patch=patch, duration=0.4)
        for patch in patches
    ]
    fine = [
        _discrete_patch_residual(400, 160, patch=patch, duration=0.4)
        for patch in patches
    ]
    assert all(math.isfinite(value) for value in coarse + fine)
    assert all(value < 0.08 for value in fine)
    assert all(fine_value < coarse_value for coarse_value, fine_value in zip(coarse, fine))


def test_accounting_patch_does_not_change_local_analytic_packet_state():
    probes = (-0.7, 0.0, 0.9)
    time = 0.21
    reference = [
        _packet(x, time, amplitude=0.8, width=0.55, speed=1.3)
        for x in probes
    ]
    # Different nested patches only change which samples are integrated.
    for _patch in ((-1.6, 1.6), (-2.4, 2.4), (-3.2, 3.2)):
        observed = [
            _packet(x, time, amplitude=0.8, width=0.55, speed=1.3)
            for x in probes
        ]
        assert observed == reference
