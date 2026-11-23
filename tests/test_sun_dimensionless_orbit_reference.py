"""Dimensionless calibrated-family reference checks for retarded sun transport.

R=1 is a conventional length normalization, not a measured physical radius.
The orbit period is project-approved; eta=H/R and beta=v_orbit/c are
dimensionless study parameters, not calibrated physical constants.
This module intentionally imports no simulator code.
"""
import math
import unittest

TAU = 2.0 * math.pi
PERIOD = 24000.0
OMEGA = TAU / PERIOD


def orbit(t, eta, beta):
    radius = 1.0
    height = eta * radius
    phase = OMEGA * t + 0.31
    x = radius * math.cos(phase)
    z = height + radius * math.sin(phase)
    vx = -radius * OMEGA * math.sin(phase)
    vz = radius * OMEGA * math.cos(phase)
    speed = radius * OMEGA
    c = speed / beta
    return x, 0.0, z, vx, 0.0, vz, c


def arrival_map(t, mu, phi, eta, beta):
    x, y, z, _, _, _, c = orbit(t, eta, beta)
    transverse = math.sqrt(1.0 - mu * mu) / mu
    delay = z / (c * mu)
    return (x + z * transverse * math.cos(phi),
            y + z * transverse * math.sin(phi),
            t + delay)


def analytic_jacobian(t, mu, phi, eta, beta):
    _, _, z, vx, vy, vz, c = orbit(t, eta, beta)
    bracket = (c + mu * vz - math.sqrt(1.0 - mu * mu) *
               (vx * math.cos(phi) + vy * math.sin(phi)))
    return z * z / (c * mu ** 3) * bracket


def determinant3(m):
    return (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))


def finite_difference_jacobian(t, mu, phi, eta, beta):
    steps = (0.02, 2.0e-5, 2.0e-5)
    columns = []
    for axis, step in enumerate(steps):
        plus = [t, mu, phi]
        minus = [t, mu, phi]
        plus[axis] += step
        minus[axis] -= step
        fp = arrival_map(*plus, eta, beta)
        fm = arrival_map(*minus, eta, beta)
        columns.append([(fp[k] - fm[k]) / (2.0 * step)
                        for k in range(3)])
    matrix = [[columns[col][row] for col in range(3)]
              for row in range(3)]
    return abs(determinant3(matrix))


class DimensionlessOrbitConvergenceTests(unittest.TestCase):
    def test_jacobian_across_dimensionless_orbit_family(self):
        # Broad, declared parameter sweep: no one fixture is called calibrated.
        for eta in (1.25, 2.0, 4.0):
            for beta in (0.02, 0.15, 0.60):
                max_relative = 0.0
                for phase in (0.13, 1.2, 2.8, 5.7):
                    t = phase / OMEGA
                    for mu in (0.16, 0.32, 0.61, 0.9):
                        for phi in (0.2, 1.7, 3.4, 5.8):
                            exact = analytic_jacobian(
                                t, mu, phi, eta, beta)
                            numeric = finite_difference_jacobian(
                                t, mu, phi, eta, beta)
                            max_relative = max(
                                max_relative,
                                abs(numeric - exact) / exact)
                self.assertLess(
                    max_relative, 2.0e-6,
                    f"eta={eta}, beta={beta}, relerr={max_relative}")

    def test_sub_propagation_speed_keeps_retarded_map_orientation(self):
        # For beta<1 the source speed is below c. The bracket in J must
        # remain positive over the sampled orbit/directions.
        for eta in (1.01, 1.25, 2.0, 4.0):
            for beta in (0.001, 0.1, 0.5, 0.9):
                for phase in [TAU * i / 64 for i in range(64)]:
                    t = phase / OMEGA
                    for mu in (0.01, 0.1, 0.5, 1.0):
                        for phi in [TAU * j / 16 for j in range(16)]:
                            self.assertGreater(
                                analytic_jacobian(t, mu, phi, eta, beta),
                                0.0)

    def test_orbit_clearance_and_speed_normalization(self):
        for eta in (1.001, 1.01, 1.25, 2.0, 4.0):
            for beta in (0.001, 0.1, 0.5, 0.9):
                min_height = min(
                    orbit(PERIOD * i / 4096, eta, beta)[2]
                    for i in range(4096))
                self.assertAlmostEqual(min_height, eta - 1.0, places=6)
                x, y, z, vx, vy, vz, c = orbit(0.0, eta, beta)
                self.assertAlmostEqual(
                    math.hypot(vx, vz) / c, beta, places=14)


if __name__ == "__main__":
    unittest.main(verbosity=2)
