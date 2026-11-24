"""Isolated algebraic/numerical checks for ambient/disturbance composition.

This module deliberately does not import or modify the production wave solver.
All quantities use project-native units.
"""
from __future__ import annotations

import math


DAY_TICKS = 24_000
CARRIER_CYCLES_PER_DAY = 440
MIN_ENERGY_DENSITY = 1.0
MAX_ENERGY_DENSITY = 10.0


def ambient_energy_density(tick: float) -> float:
    phase = 2.0 * math.pi * tick / DAY_TICKS
    return 1.0 + 4.5 * (1.0 + math.cos(phase))


def ambient_energy_density_derivative(tick: float) -> float:
    phase = 2.0 * math.pi * tick / DAY_TICKS
    envelope_angular_frequency = 2.0 * math.pi / DAY_TICKS
    return -4.5 * envelope_angular_frequency * math.sin(phase)


def carrier_angular_frequency() -> float:
    return 2.0 * math.pi * CARRIER_CYCLES_PER_DAY / DAY_TICKS


def carrier_displacement(tick: float, amplitude: float, phase: float = 0.0) -> float:
    return amplitude * math.cos(carrier_angular_frequency() * tick + phase)


def carrier_second_derivative(
    tick: float, amplitude: float, phase: float = 0.0
) -> float:
    omega = carrier_angular_frequency()
    return -(omega**2) * carrier_displacement(tick, amplitude, phase)


def combined_energy_density(
    *,
    field_velocity: float,
    field_gradient_x: float,
    field_gradient_y: float,
    ambient_velocity: float,
    wave_speed: float,
    normalization: float,
) -> float:
    return 0.5 * normalization * (
        (field_velocity + ambient_velocity) ** 2
        + wave_speed**2 * (field_gradient_x**2 + field_gradient_y**2)
    )


def disturbance_energy_density(
    *,
    field_velocity: float,
    field_gradient_x: float,
    field_gradient_y: float,
    wave_speed: float,
    normalization: float,
) -> float:
    return 0.5 * normalization * (
        field_velocity**2
        + wave_speed**2 * (field_gradient_x**2 + field_gradient_y**2)
    )


def combined_flux(
    *,
    field_velocity: float,
    field_gradient_x: float,
    field_gradient_y: float,
    ambient_velocity: float,
    wave_speed: float,
    normalization: float,
) -> tuple[float, float]:
    scale = -normalization * wave_speed**2 * (field_velocity + ambient_velocity)
    return scale * field_gradient_x, scale * field_gradient_y


def disturbance_flux(
    *,
    field_velocity: float,
    field_gradient_x: float,
    field_gradient_y: float,
    wave_speed: float,
    normalization: float,
) -> tuple[float, float]:
    scale = -normalization * wave_speed**2 * field_velocity
    return scale * field_gradient_x, scale * field_gradient_y


def envelope_correction_relative_to_target(tick: float) -> float:
    """Return (K a'^2 / 4) / e_target; K cancels exactly."""
    target = ambient_energy_density(tick)
    target_derivative = ambient_energy_density_derivative(tick)
    omega = carrier_angular_frequency()
    # a'/a = e'/(2e), and e_target = K omega^2 a^2 / 2.
    return target_derivative**2 / (8.0 * target**2 * omega**2)


def test_uniform_nonzero_frequency_carrier_is_not_a_free_massless_wave_mode():
    omega = carrier_angular_frequency()
    tick = 173.25
    amplitude = 2.0
    displacement = carrier_displacement(tick, amplitude)
    residual = carrier_second_derivative(tick, amplitude)
    assert omega > 0.0
    assert math.isclose(residual, -(omega**2) * displacement, rel_tol=1e-14)
    assert abs(residual) > 1e-8


def test_additive_field_energy_requires_the_cross_energy_term():
    values = {
        "field_velocity": 0.37,
        "field_gradient_x": -0.21,
        "field_gradient_y": 0.44,
        "ambient_velocity": 0.63,
        "wave_speed": 1.7,
        "normalization": 2.3,
    }
    total = combined_energy_density(**values)
    disturbance = disturbance_energy_density(
        field_velocity=values["field_velocity"],
        field_gradient_x=values["field_gradient_x"],
        field_gradient_y=values["field_gradient_y"],
        wave_speed=values["wave_speed"],
        normalization=values["normalization"],
    )
    ambient_kinetic = 0.5 * values["normalization"] * values["ambient_velocity"] ** 2
    cross = (
        values["normalization"]
        * values["ambient_velocity"]
        * values["field_velocity"]
    )
    assert math.isclose(
        total, disturbance + ambient_kinetic + cross, rel_tol=1e-14, abs_tol=1e-14
    )
    assert not math.isclose(
        total, disturbance + ambient_kinetic, rel_tol=1e-12, abs_tol=1e-12
    )


def test_additive_field_flux_requires_the_cross_flux_term():
    values = {
        "field_velocity": 0.37,
        "field_gradient_x": -0.21,
        "field_gradient_y": 0.44,
        "ambient_velocity": 0.63,
        "wave_speed": 1.7,
        "normalization": 2.3,
    }
    total = combined_flux(**values)
    disturbance = disturbance_flux(
        field_velocity=values["field_velocity"],
        field_gradient_x=values["field_gradient_x"],
        field_gradient_y=values["field_gradient_y"],
        wave_speed=values["wave_speed"],
        normalization=values["normalization"],
    )
    expected_cross = tuple(
        -values["normalization"]
        * values["wave_speed"] ** 2
        * values["ambient_velocity"]
        * gradient
        for gradient in (values["field_gradient_x"], values["field_gradient_y"])
    )
    for actual, base, cross in zip(total, disturbance, expected_cross):
        assert math.isclose(actual, base + cross, rel_tol=1e-14, abs_tol=1e-14)
        assert cross != 0.0


def test_daily_envelope_energy_correction_is_small_but_nonzero():
    corrections = [
        envelope_correction_relative_to_target(tick)
        for tick in range(DAY_TICKS)
    ]
    maximum = max(corrections)
    assert maximum > 0.0
    # The 1:440 envelope/carrier frequency ratio makes the correction ppm-scale.
    assert 1.0e-6 < maximum < 2.0e-6
    assert all(math.isfinite(value) and value >= 0.0 for value in corrections)


def test_accounting_patch_area_changes_inventory_not_local_density():
    local_density = ambient_energy_density(1_234.0)
    patch_areas = (10.0, 25.0, 100.0)
    patch_inventories = [local_density * area for area in patch_areas]
    assert all(
        math.isclose(inventory / area, local_density, rel_tol=1e-14)
        for inventory, area in zip(patch_inventories, patch_areas)
    )
    assert patch_inventories[0] < patch_inventories[1] < patch_inventories[2]


def test_ambient_schedule_and_diagnostics_are_deterministic():
    first = [
        (
            ambient_energy_density(tick),
            ambient_energy_density_derivative(tick),
            envelope_correction_relative_to_target(tick),
        )
        for tick in range(DAY_TICKS)
    ]
    second = [
        (
            ambient_energy_density(tick),
            ambient_energy_density_derivative(tick),
            envelope_correction_relative_to_target(tick),
        )
        for tick in range(DAY_TICKS)
    ]
    assert first == second
