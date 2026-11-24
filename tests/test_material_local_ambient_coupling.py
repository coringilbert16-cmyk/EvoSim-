"""Finite-dimensional reference tests for material-local ambient coupling.

This is a deliberately small, standard-library-only reference model. It does not
import or modify the production solver. Periodic centered differences and implicit
midpoint integration make the source-free semi-discrete Hamiltonian auditable.
"""
from __future__ import annotations

import math
import random
import unittest


def _operator(size, *, spacing, speed, field_mass, oscillator_mass, omega, coupling):
    """Build y' = A y for y = (w, w_t, q, q_t)."""
    dimension = 4 * size
    matrix = [[0.0] * dimension for _ in range(dimension)]
    inverse_spacing_squared = 1.0 / spacing**2
    for index in range(size):
        previous = (index - 1) % size
        following = (index + 1) % size
        strength = coupling[index]

        matrix[index][size + index] = 1.0
        matrix[2 * size + index][3 * size + index] = 1.0

        laplacian_scale = speed**2 * inverse_spacing_squared
        matrix[size + index][previous] += laplacian_scale
        matrix[size + index][index] -= 2.0 * laplacian_scale
        matrix[size + index][following] += laplacian_scale
        matrix[size + index][3 * size + index] = -strength / field_mass

        matrix[3 * size + index][2 * size + index] = -omega**2
        matrix[3 * size + index][size + index] = strength / oscillator_mass
    return matrix


def _factor(matrix):
    """Small dense LU factorization with partial pivoting."""
    size = len(matrix)
    lu = [row[:] for row in matrix]
    permutation = list(range(size))
    for column in range(size):
        pivot = max(range(column, size), key=lambda row: abs(lu[row][column]))
        if abs(lu[pivot][column]) < 1.0e-14:
            raise ArithmeticError("singular reference-system matrix")
        lu[column], lu[pivot] = lu[pivot], lu[column]
        permutation[column], permutation[pivot] = (
            permutation[pivot],
            permutation[column],
        )
        for row in range(column + 1, size):
            lu[row][column] /= lu[column][column]
            multiplier = lu[row][column]
            for entry in range(column + 1, size):
                lu[row][entry] -= multiplier * lu[column][entry]
    return lu, permutation


def _solve(factorization, right_hand_side):
    lu, permutation = factorization
    size = len(right_hand_side)
    solution = [right_hand_side[permutation[i]] for i in range(size)]
    for row in range(size):
        solution[row] -= sum(
            lu[row][column] * solution[column] for column in range(row)
        )
    for row in range(size - 1, -1, -1):
        solution[row] = (
            solution[row]
            - sum(
                lu[row][column] * solution[column]
                for column in range(row + 1, size)
            )
        ) / lu[row][row]
    return solution


def _matvec(matrix, vector):
    return [
        sum(coefficient * value for coefficient, value in zip(row, vector))
        for row in matrix
    ]


def _energy(
    state,
    *,
    size,
    spacing,
    speed,
    field_mass,
    oscillator_mass,
    omega,
):
    w = state[:size]
    velocity = state[size : 2 * size]
    q = state[2 * size : 3 * size]
    q_velocity = state[3 * size :]
    total = 0.0
    for index in range(size):
        forward_gradient = (w[(index + 1) % size] - w[index]) / spacing
        total += 0.5 * field_mass * (
            velocity[index] ** 2 + speed**2 * forward_gradient**2
        )
        total += 0.5 * oscillator_mass * (
            q_velocity[index] ** 2 + omega**2 * q[index] ** 2
        )
    return spacing * total


def _patch_energy(
    state,
    *,
    start,
    end,
    size,
    spacing,
    speed,
    field_mass,
    oscillator_mass,
    omega,
):
    """Patch inventory with half of each adjacent field-gradient edge."""
    w = state[:size]
    velocity = state[size : 2 * size]
    q = state[2 * size : 3 * size]
    q_velocity = state[3 * size :]
    total = 0.0
    for index in range(start, end + 1):
        right_gradient = (w[(index + 1) % size] - w[index]) / spacing
        left_gradient = (w[index] - w[(index - 1) % size]) / spacing
        total += 0.5 * field_mass * velocity[index] ** 2
        total += 0.25 * field_mass * speed**2 * (
            right_gradient**2 + left_gradient**2
        )
        total += 0.5 * oscillator_mass * (
            q_velocity[index] ** 2 + omega**2 * q[index] ** 2
        )
    return spacing * total


def _face_flux(
    state,
    *,
    face,
    size,
    spacing,
    speed,
    field_mass,
):
    """Centered-in-time discrete field-energy flux through one grid face."""
    w = state[:size]
    velocity = state[size : 2 * size]
    gradient = (w[(face + 1) % size] - w[face]) / spacing
    average_velocity = 0.5 * (velocity[face] + velocity[(face + 1) % size])
    return -field_mass * speed**2 * gradient * average_velocity


def _amplitude(time):
    return 1.0 + 0.15 * math.cos(0.4 * time)


def _amplitude_derivative(time):
    return -0.06 * math.sin(0.4 * time)


def _amplitude_second_derivative(time):
    return -0.024 * math.cos(0.4 * time)


def _target_q(time, omega):
    return _amplitude(time) * math.cos(omega * time + 0.2)


def _target_q_velocity(time, omega):
    phase = omega * time + 0.2
    return (
        _amplitude_derivative(time) * math.cos(phase)
        - _amplitude(time) * omega * math.sin(phase)
    )


def _world_drive(time, *, oscillator_mass, omega):
    phase = omega * time + 0.2
    return oscillator_mass * (
        _amplitude_second_derivative(time) * math.cos(phase)
        - 2.0 * omega * _amplitude_derivative(time) * math.sin(phase)
    )


def _advance(
    state,
    operator,
    factorization,
    *,
    time,
    step,
    size,
    oscillator_mass,
    omega,
    driven,
):
    """One implicit-midpoint step; source is evaluated at the time midpoint."""
    derivative = _matvec(operator, state)
    rhs = [
        value + 0.5 * step * rate
        for value, rate in zip(state, derivative)
    ]
    drive = _world_drive(
        time + 0.5 * step,
        oscillator_mass=oscillator_mass,
        omega=omega,
    ) if driven else 0.0
    if driven:
        for index in range(size):
            rhs[3 * size + index] += step * drive / oscillator_mass

    next_state = _solve(factorization, rhs)
    midpoint_q_velocity = [
        0.5 * (state[3 * size + index] + next_state[3 * size + index])
        for index in range(size)
    ]
    source_work = step * drive * sum(midpoint_q_velocity)
    return next_state, source_work


class MaterialLocalAmbientCouplingTests(unittest.TestCase):
    def setUp(self):
        self.size = 12
        self.spacing = 1.0
        self.speed = 1.0
        self.field_mass = 1.3
        self.oscillator_mass = 0.8
        self.omega = 1.7
        self.step = 0.02
        self.coupling = [
            0.35 if 4 <= index <= 7 else 0.0
            for index in range(self.size)
        ]

    def _system(self, coupling=None, step=None):
        selected_coupling = self.coupling if coupling is None else coupling
        selected_step = self.step if step is None else step
        operator = _operator(
            self.size,
            spacing=self.spacing,
            speed=self.speed,
            field_mass=self.field_mass,
            oscillator_mass=self.oscillator_mass,
            omega=self.omega,
            coupling=selected_coupling,
        )
        dimension = 4 * self.size
        midpoint_matrix = [
            [
                float(row == column)
                - 0.5 * selected_step * operator[row][column]
                for column in range(dimension)
            ]
            for row in range(dimension)
        ]
        return operator, _factor(midpoint_matrix)

    def _energy(self, state):
        return _energy(
            state,
            size=self.size,
            spacing=self.spacing,
            speed=self.speed,
            field_mass=self.field_mass,
            oscillator_mass=self.oscillator_mass,
            omega=self.omega,
        )

    def test_hamiltonian_is_nonnegative_for_random_finite_states(self):
        generator = random.Random(314159)
        for _ in range(100):
            state = [generator.uniform(-2.0, 2.0) for _ in range(4 * self.size)]
            self.assertGreaterEqual(self._energy(state), 0.0)

    def test_coupling_operator_is_zero_outside_physical_material_support(self):
        uncoupled, _ = self._system(coupling=[0.0] * self.size)
        localized, _ = self._system()
        changed_entries = {
            (row, column)
            for row in range(4 * self.size)
            for column in range(4 * self.size)
            if localized[row][column] != uncoupled[row][column]
        }
        expected_entries = set()
        for index, strength in enumerate(self.coupling):
            if strength:
                expected_entries.add((self.size + index, 3 * self.size + index))
                expected_entries.add((3 * self.size + index, self.size + index))
        self.assertEqual(changed_entries, expected_entries)

    def test_implicit_midpoint_conserves_source_free_total_energy(self):
        operator, factorization = self._system()
        generator = random.Random(271828)
        state = [generator.uniform(-0.2, 0.2) for _ in range(4 * self.size)]
        initial_energy = self._energy(state)
        for step_index in range(500):
            state, source_work = _advance(
                state,
                operator,
                factorization,
                time=step_index * self.step,
                step=self.step,
                size=self.size,
                oscillator_mass=self.oscillator_mass,
                omega=self.omega,
                driven=False,
            )
            self.assertEqual(source_work, 0.0)
        relative_residual = abs(self._energy(state) - initial_energy) / initial_energy
        self.assertLess(relative_residual, 1.0e-12)

    def test_nested_accounting_patches_reconcile_flux_and_world_work(self):
        operator, factorization = self._system()
        state = [0.0] * (4 * self.size)
        for index in range(self.size):
            position = index - 3.0
            state[index] = math.exp(-0.5 * (position / 1.4) ** 2)
            state[self.size + index] = 0.04 * math.sin(0.7 * index)
            state[2 * self.size + index] = _target_q(0.0, self.omega)
            state[3 * self.size + index] = _target_q_velocity(0.0, self.omega)

        patches = ((2, 9), (1, 10), (0, self.size - 1))
        initial = [
            _patch_energy(
                state,
                start=start,
                end=end,
                size=self.size,
                spacing=self.spacing,
                speed=self.speed,
                field_mass=self.field_mass,
                oscillator_mass=self.oscillator_mass,
                omega=self.omega,
            )
            for start, end in patches
        ]
        cumulative_flux = [0.0] * len(patches)
        cumulative_work = [0.0] * len(patches)

        for step_index in range(80):
            time = step_index * self.step
            drive = _world_drive(
                time + 0.5 * self.step,
                oscillator_mass=self.oscillator_mass,
                omega=self.omega,
            )
            derivative = _matvec(operator, state)
            rhs = [
                value + 0.5 * self.step * rate
                for value, rate in zip(state, derivative)
            ]
            for index in range(self.size):
                rhs[3 * self.size + index] += (
                    self.step * drive / self.oscillator_mass
                )
            next_state = _solve(factorization, rhs)
            midpoint = [
                0.5 * (before + after)
                for before, after in zip(state, next_state)
            ]

            for patch_index, (start, end) in enumerate(patches):
                cumulative_work[patch_index] += (
                    self.step
                    * self.spacing
                    * drive
                    * sum(midpoint[3 * self.size + start : 3 * self.size + end + 1])
                )
                right_flux = _face_flux(
                    midpoint,
                    face=end,
                    size=self.size,
                    spacing=self.spacing,
                    speed=self.speed,
                    field_mass=self.field_mass,
                )
                left_flux = _face_flux(
                    midpoint,
                    face=start - 1,
                    size=self.size,
                    spacing=self.spacing,
                    speed=self.speed,
                    field_mass=self.field_mass,
                )
                cumulative_flux[patch_index] += self.step * (
                    right_flux - left_flux
                )
            state = next_state

        for patch_index, (start, end) in enumerate(patches):
            final = _patch_energy(
                state,
                start=start,
                end=end,
                size=self.size,
                spacing=self.spacing,
                speed=self.speed,
                field_mass=self.field_mass,
                oscillator_mass=self.oscillator_mass,
                omega=self.omega,
            )
            residual = (
                final
                - initial[patch_index]
                + cumulative_flux[patch_index]
                - cumulative_work[patch_index]
            )
            scale = max(
                abs(final - initial[patch_index]),
                abs(cumulative_flux[patch_index]),
                abs(cumulative_work[patch_index]),
                1.0e-12,
            )
            self.assertLess(abs(residual) / scale, 1.0e-10)

    def test_driven_coupled_system_reconciles_world_work_without_double_counting(self):
        operator, factorization = self._system()
        state = [0.0] * (4 * self.size)
        for index in range(self.size):
            position = index - 3.0
            state[index] = math.exp(-0.5 * (position / 1.4) ** 2)
            state[self.size + index] = 0.04 * math.sin(0.7 * index)
            state[2 * self.size + index] = _target_q(0.0, self.omega)
            state[3 * self.size + index] = _target_q_velocity(0.0, self.omega)

        initial_energy = self._energy(state)
        cumulative_world_work = 0.0
        steps = 200
        for step_index in range(steps):
            state, source_work = _advance(
                state,
                operator,
                factorization,
                time=step_index * self.step,
                step=self.step,
                size=self.size,
                oscillator_mass=self.oscillator_mass,
                omega=self.omega,
                driven=True,
            )
            cumulative_world_work += source_work

        residual = self._energy(state) - initial_energy - cumulative_world_work
        self.assertLess(abs(residual), 2.0e-11)
        # Coupling changes the oscillator response: it must not be clamped back
        # to the uncoupled target trajectory.
        self.assertGreater(
            abs(state[2 * self.size + 5] - _target_q(steps * self.step, self.omega)),
            1.0e-5,
        )

    def test_intrinsic_drive_recovers_target_and_has_second_order_convergence(self):
        errors = []
        for step in (0.04, 0.02):
            size = 4
            operator = _operator(
                size,
                spacing=1.0,
                speed=1.0,
                field_mass=self.field_mass,
                oscillator_mass=self.oscillator_mass,
                omega=self.omega,
                coupling=[0.0] * size,
            )
            dimension = 4 * size
            matrix = [
                [
                    float(row == column) - 0.5 * step * operator[row][column]
                    for column in range(dimension)
                ]
                for row in range(dimension)
            ]
            factorization = _factor(matrix)
            state = [0.0] * (4 * size)
            for index in range(size):
                state[2 * size + index] = _target_q(0.0, self.omega)
                state[3 * size + index] = _target_q_velocity(0.0, self.omega)

            total_time = 2.0
            count = round(total_time / step)
            initial_energy = _energy(
                state,
                size=size,
                spacing=1.0,
                speed=1.0,
                field_mass=self.field_mass,
                oscillator_mass=self.oscillator_mass,
                omega=self.omega,
            )
            cumulative_work = 0.0
            for step_index in range(count):
                state, source_work = _advance(
                    state,
                    operator,
                    factorization,
                    time=step_index * step,
                    step=step,
                    size=size,
                    oscillator_mass=self.oscillator_mass,
                    omega=self.omega,
                    driven=True,
                )
                cumulative_work += source_work

            final_energy = _energy(
                state,
                size=size,
                spacing=1.0,
                speed=1.0,
                field_mass=self.field_mass,
                oscillator_mass=self.oscillator_mass,
                omega=self.omega,
            )
            self.assertLess(
                abs(final_energy - initial_energy - cumulative_work), 2.0e-11
            )
            errors.append(
                abs(state[2 * size] - _target_q(total_time, self.omega))
                + abs(
                    state[3 * size]
                    - _target_q_velocity(total_time, self.omega)
                )
            )

        self.assertGreater(errors[0] / errors[1], 3.8)


if __name__ == "__main__":
    unittest.main()
