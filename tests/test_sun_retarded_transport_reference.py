import math
import unittest

TAU = 2.0 * math.pi


def trajectory(t):
    # Test-only orbit parameters; the 24,000-tick period is project-approved.
    # Radius and height are numerical fixtures, not calibrated physical values.
    omega = TAU / 24000.0
    radius = 0.7
    center_z = 3.0
    phase = omega * t + 0.31
    return (1.2 + radius * math.cos(phase), -0.4,
            center_z + radius * math.sin(phase),
            -radius * omega * math.sin(phase), 0.0,
            radius * omega * math.cos(phase))


def ray_map(t, mu, phi, c):
    x, y, z, vx, vy, vz = trajectory(t)
    transverse = math.sqrt(1.0 - mu * mu) / mu
    delay = z / (c * mu)
    return (x + z * transverse * math.cos(phi),
            y + z * transverse * math.sin(phi), t + delay)


def analytic_jacobian(t, mu, phi, c):
    _, _, z, vx, vy, vz = trajectory(t)
    bracket = (c + mu * vz - math.sqrt(1.0 - mu * mu) *
               (vx * math.cos(phi) + vy * math.sin(phi)))
    return z * z / (c * mu ** 3) * bracket


def det3(m):
    return (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))


def numerical_jacobian(t, mu, phi, c, h):
    columns = []
    for axis in range(3):
        plus = [t, mu, phi]
        minus = [t, mu, phi]
        plus[axis] += h
        minus[axis] -= h
        fp = ray_map(*plus, c)
        fm = ray_map(*minus, c)
        columns.append([(fp[k] - fm[k]) / (2.0 * h) for k in range(3)])
    matrix = [[columns[col][row] for col in range(3)] for row in range(3)]
    return abs(det3(matrix))


class SunTransportReferenceTests(unittest.TestCase):
    def test_moving_source_jacobian_against_independent_finite_difference(self):
        c = 0.35
        max_relative = 0.0
        for t in (-4.3, 0.0, 2.7, 11.9):
            for mu in (0.17, 0.31, 0.62, 0.88):
                for phi in (0.2, 1.4, 3.1, 5.6):
                    exact = analytic_jacobian(t, mu, phi, c)
                    numeric = numerical_jacobian(t, mu, phi, c, 2.0e-5)
                    max_relative = max(max_relative,
                                       abs(numeric - exact) / exact)
        self.assertLess(max_relative, 2.0e-7,
                        f"max relative determinant error {max_relative}")

    def test_static_ray_to_plane_radial_energy_distribution(self):
        # Uniform mu is uniform downward solid angle. This independent ray
        # count does not evaluate the area/time Jacobian or flux formula.
        height = 2.0
        emitted_power = 7.3
        n = 200_000
        for radius in (1.0, 3.0, 8.0):
            threshold = height / math.sqrt(radius * radius + height * height)
            captured = sum(1 for i in range(n)
                          if (i + 0.5) / n >= threshold) / n
            expected = 1.0 - threshold
            self.assertLessEqual(abs(captured - expected), 1.1 / n)
            self.assertLessEqual(abs(emitted_power * 0.5 * (captured - expected)),
                               emitted_power * 0.55 / n)
        self.assertAlmostEqual(emitted_power * 0.5, 3.65, places=14)

    def test_full_orbit_emission_partition(self):
        # Integrate a constant power envelope over the approved orbital
        # period. Radius/height remain numerical fixtures, not calibration.
        period, n_t = 24000.0, 2400
        power, mu_min, n_mu = 1.7, 0.12, 400
        dt = period / n_t
        dmu = (1.0 - mu_min) / n_mu
        downward = 0.0
        for it in range(n_t):
            t = (it + 0.5) * dt
            self.assertGreater(trajectory(t)[2], 0.0)
            for im in range(n_mu):
                mu = mu_min + (im + 0.5) * dmu
                self.assertGreater(t + trajectory(t)[2] / (0.35 * mu), t)
                downward += (power * 0.5) * dt * dmu
        resolved = power * period * 0.5 * (1.0 - mu_min)
        self.assertAlmostEqual(downward, resolved, places=6)
        upward_external = power * period * 0.5
        grazing_unresolved = power * period * 0.5 * mu_min
        self.assertAlmostEqual(downward + grazing_unresolved,
                               power * period * 0.5, places=6)
        self.assertAlmostEqual(upward_external,
                               power * period * 0.5, places=6)

    def test_finite_window_emission_arrival_and_in_flight_balance(self):
        # Deterministic ray parcels. The grazing range mu<mu_min is excluded
        # from this finite quadrature and its omitted power is reported below.
        power, c, mu_min, n_mu = 2.4, 0.35, 0.12, 500
        dmu = (1.0 - mu_min) / n_mu
        dt = 0.05
        t_start, t_end = 0.0, 25.0
        emit_lo, emit_hi = -100.0, t_end
        n_t = int(round((emit_hi - emit_lo) / dt))
        emitted_window = initial_flight = arrivals_window = final_flight = 0.0
        for it in range(n_t):
            emit_t = emit_lo + (it + 0.5) * dt
            z = trajectory(emit_t)[2]
            for im in range(n_mu):
                mu = mu_min + (im + 0.5) * dmu
                energy = (power * 0.5) * dt * dmu
                arrival = emit_t + z / (c * mu)
                if t_start <= emit_t < t_end:
                    emitted_window += energy
                if emit_t < t_start and arrival > t_start:
                    initial_flight += energy
                if t_start <= arrival < t_end:
                    arrivals_window += energy
                if emit_t < t_end and arrival >= t_end:
                    final_flight += energy
        residual = (emitted_window + initial_flight
                    - arrivals_window - final_flight)
        self.assertLess(abs(residual), 2.0e-9,
                        f"finite-window balance residual {residual}")
        omitted_grazing_power = power * 0.5 * mu_min
        self.assertAlmostEqual(omitted_grazing_power, 0.144, places=12)


if __name__ == "__main__":
    unittest.main(verbosity=2)
