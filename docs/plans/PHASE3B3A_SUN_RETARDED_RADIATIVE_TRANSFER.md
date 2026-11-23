# Phase 3B.3a — Sun Retarded Radiative Transfer

**Status:** Analytical derivation and test specification. No source implementation or production coupling is authorized.

**Parent design:** [Phase 3B.3 physical source-to-field coupling](PHASE3B3_PHYSICAL_SOURCE_COUPLING.md)  
**Approved geometry/clock:** [Unified Energy and Continuous Wave-Field Specification](../design/unified-energy-physics-specification.md)

## 1. Scope

Derive the mapping from a moving external isotropic source's emitted energy to its arrival-time and area distribution on the infinite 2D plane. This is the radiation-transport layer only. It does not determine how incident radiation excites the scalar wave field.

The source follows the approved orbit with height strictly positive. Let its project-plane trajectory be

\[
\mathbf{s}(\tau)=(x_s(\tau),y_s(\tau),z_s(\tau)),\qquad z_s(\tau)>0,
\]

where \(\tau\) is emission time and \(P(\tau)\) is total emitted energy per project tick.

The approved orbit is a circle in a vertical plane perpendicular to the world plane, with period 24,000 ticks. For derivation and test implementation, retain it symbolically as

\[
\mathbf{s}(\tau)=\mathbf{s}_c
+R\cos(\Omega\tau+\theta_0)\,\mathbf e_x
+(z_c+R\sin(\Omega\tau+\theta_0))\,\mathbf e_z,
\qquad \Omega=\frac{2\pi}{24000}.
\]

Here \(R\), center height \(z_c\), horizontal center, plane orientation, and phase origin \(\theta_0\) remain uncalibrated project parameters. Require \(z_c>R\) so the source remains strictly above the plane. This parameterization is a coordinate form for the approved orbit, not a choice of numerical radius, height, or phase.

The 440-cycle/day sun carrier is a separate phase variable from the 24,000-tick orbital phase. The transport map pushes forward the nonnegative emitted-power envelope \(P(\tau)\); it does not itself define the carrier's field waveform or convert carrier amplitude into positive radiative power. The radiation-to-field constitutive stage must specify that relationship without treating a signed carrier as negative emitted energy.

**Frame convention required for this derivation:** rays are emitted isotropically in the project frame, with constant propagation speed \(c\) in that frame. No relativistic Doppler/aberration correction is included. This is a project-native transport convention, not a claim about a physical relativistic sun. If the specification instead intends isotropy in a moving source's rest frame, the emission angular distribution and power transformation must be derived separately before implementation.

## 2. Ray parameterization

Parameterize downward rays by:
- \(\mu=-n_z\in(0,1]\), the downward direction cosine;
- azimuth \(\phi\in[0,2\pi)\);
- horizontal direction \(\mathbf e_\phi=(\cos\phi,\sin\phi)\).

The ray direction is

\[
\mathbf n=(\sqrt{1-\mu^2}\cos\phi,
\sqrt{1-\mu^2}\sin\phi,-\mu).
\]

A ray emitted at \(\tau\) intersects \(z=0\) after

\[
\Delta t(\tau,\mu)=\frac{z_s(\tau)}{c\mu}.
\]

Thus its arrival time and plane intersection are

\[
T(\tau,\mu)=\tau+\frac{z_s(\tau)}{c\mu},
\]

\[
\mathbf X(\tau,\mu,\phi)
=\mathbf s_{xy}(\tau)
+\frac{z_s(\tau)\sqrt{1-\mu^2}}{\mu}\mathbf e_\phi.
\]

The source's horizontal motion and changing height both affect the arrival map. Evaluating a stationary-source flux formula at a retarded source position is not sufficient to account for arrival-time compression/dilation.

## 3. Emitted energy measure and arrival Jacobian

Under project-frame isotropic emission, emitted energy into solid angle \(d\Omega=d\phi\,d\mu\) on the downward hemisphere is

\[
dE_{\downarrow}
=\frac{P(\tau)}{4\pi}\,d\tau\,d\mu\,d\phi.
\]

The map \((\tau,\mu,\phi)\mapsto(T,X,Y)\) pushes this measure onto the plane's area-and-arrival-time measure. Its absolute Jacobian is

\[
J(\tau,\mu,\phi)
=\left|\frac{\partial(X,Y,T)}
{\partial(\tau,\mu,\phi)}\right|
=\frac{z_s(\tau)^2}{c\mu^3}
\left[
c+\mu\dot z_s(\tau)
-\sqrt{1-\mu^2}
\left(\dot x_s(\tau)\cos\phi+
\dot y_s(\tau)\sin\phi\right)
\right].
\]

For a source speed strictly less than \(c\), the bracket is positive: the velocity projection term is bounded in magnitude by source speed, while the remaining factor is at least \(c-|\dot{\mathbf s}|\). This prevents a zero-Jacobian caustic in this parameterization. The implementation should still validate the bound and reject unsupported source trajectories.

For auditability, let \(A(\mu)=\sqrt{1-\mu^2}/\mu\) and \(R=z_sA\). The coordinate derivatives are

\[
\begin{aligned}
X_\tau&=\dot x_s+\dot z_sA\cos\phi,&
Y_\tau&=\dot y_s+\dot z_sA\sin\phi,\\
X_\mu&=z_sA'\cos\phi,&
Y_\mu&=z_sA'\sin\phi,\\
X_\phi&=-R\sin\phi,&Y_\phi&=R\cos\phi,\\
T_\tau&=1+\frac{\dot z_s}{c\mu},&
T_\mu&=-\frac{z_s}{c\mu^2},&
T_\phi&=0,
\end{aligned}
\qquad
A'(\mu)=-\frac{1}{\mu^2\sqrt{1-\mu^2}}.
\]

Taking the determinant of these derivatives gives the stated \(J\). The azimuth coordinate degenerates for the exactly vertical ray \(\mu=1\); use the continuous limit there (the single ray has zero solid-angle measure). The grazing limit \(\mu\to0^+\) is treated as a limiting arrival distribution, not discarded.

Where the arrival map is one-to-one, the incident power density at the mapped event is

\[
q_{\odot}(T,\mathbf X)
=\frac{P(\tau)}{4\pi J(\tau,\mu,\phi)}.
\]

If an allowed trajectory or source model makes the map non-injective, the density must sum contributions from every preimage; it must not silently choose one ray branch. Units are \([q_{\odot}]=E/(L^2\,tick)\).

This expression is a pushforward of emitted energy. It includes the emission-to-arrival time Jacobian and horizontal source motion; it is not a wave-equation forcing.

## 4. Static-source check

For a stationary source at height \(h\), \(\dot{\mathbf s}=0\), and the expression reduces to

\[
J=\frac{h^2}{\mu^3},\qquad
q_{\odot}=\frac{P\mu^3}{4\pi h^2}.
\]

At plane radius \(r\), \(\mu=h/\sqrt{r^2+h^2}\), yielding

\[
q_{\odot}(r)=\frac{Ph}{4\pi(r^2+h^2)^{3/2}}.
\]

Integrating over the infinite plane:

\[
\int_{\mathbb R^2}q_{\odot}\,dA
=\int_0^{2\pi}\int_0^\infty
\frac{Ph}{4\pi(r^2+h^2)^{3/2}}r\,dr\,d\phi
=\frac P2.
\]

This is the first analytical reference test. It verifies inverse-square spreading plus incidence projection and the half-power normalization independently of the wave solver.

## 5. Global conservation and in-transit energy

For every emission time, an isotropic source emits exactly half its energy into the downward hemisphere. Because the plane is infinite and the source remains above it, every downward ray with \(\mu>0\) eventually intersects the plane. Therefore, over complete emission and arrival histories,

\[
E_{\mathrm{plane,received}}
=\frac12 E_{\mathrm{emitted}},
\qquad
E_{\mathrm{nonintercepted}}
=\frac12 E_{\mathrm{emitted}}.
\]

The second half remains in the external accounting domain; it is not planar field energy.

For a finite observation interval, received energy during that same interval need not equal half of energy emitted during it. The difference is radiation still in flight at the interval boundaries. Grazing rays \(\mu\to0^+\) have unbounded travel time, although the exactly horizontal set has zero solid-angle measure. The accounting must include the emitted-downward, not-yet-arrived energy inventory, or use an equivalent exact boundary term. Do not truncate grazing rays without reporting and conserving the omitted energy.

Define \(I_{\downarrow}(t)\) as the energy in downward rays that have been emitted but have not yet reached the plane at time \(t\). For any finite interval \([t_0,t_1]\), the exact transport reconciliation is

\[
E_{\downarrow,\mathrm{emit}}[t_0,t_1]
+I_{\downarrow}(t_0)
=E_{\mathrm{arrive}}[t_0,t_1]
+I_{\downarrow}(t_1).
\]

This explicitly accounts for radiation already in flight at the start and radiation still in flight at the end; no sign convention is left implicit. Separately,

\[
E_{\mathrm{emitted}}
=E_{\downarrow,\mathrm{emitted}}
+E_{\mathrm{upward,external}}.
\]

These transport balances close before any incident-radiation-to-field coupling is introduced.

## 6. Required reference tests

1. **Static analytic profile:** compare pointwise against \(Ph/(4\pi(r^2+h^2)^{3/2})\); integrate to \(P/2\) with declared quadrature error.
2. **Moving-source Jacobian:** compare the analytic determinant against an independent finite-difference determinant over trajectory, \(\mu\), and azimuth samples.
3. **Ray-pushforward agreement:** compare Jacobian-density integration against an independent deterministic emitted-ray binning/reference measure. State resolution error; do not treat histogram noise as conservation tolerance.
4. **Emission/arrival interval test:** verify energy delayed across both ends of a finite window is assigned to in-transit inventory.
5. **Full-orbit test:** once the approved orbit's still-open radius, center height, orientation, and phase origin are calibrated, run its 24,000-tick trajectory and verify total downward emitted energy, eventual plane arrival, and the external upward half independently of phase and sampling. Keep the 440-cycle/day carrier phase independent of orbital phase; transport conservation is tested on the emitted-power envelope unless a separately defined nonnegative carrier-modulated power law is approved.
6. **Trajectory safety:** reject height crossing/touching the plane, non-finite derivatives, unsupported superluminal project-frame source speed, singular/negative Jacobian, and invalid power without mutating source state.
7. **Grid independence:** integrate received energy over physical plane area and time; verify convergence under spatial/time refinement and translation. Do not normalize by object or molecule count.

The tests must be independent of Candidate A/B forcing and of chemistry.

## 7. Still unresolved after transport

This derivation answers how external radiative energy reaches the plane. It does **not** answer how that energy couples into the scalar wave field.

The next separate decision must specify a physical interface between incident radiation and the field: which fraction is reflected/transmitted, stored in a defined coupling state, converted to thermal energy, or delivered as signed field work. The local energy balance must close without treating incident flux as acceleration forcing, without a hidden efficiency factor, and without double-counting source energy.

No live sun injection, field forcing law, production ledger migration, or chemistry/material coupling is authorized by this document.
