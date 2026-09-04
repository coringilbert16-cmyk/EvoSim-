use crate::resources::BaseResource;

/// Legacy checkpoint geometry utility retained only as a geometry helper.
/// It is not the organism's structural authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorePairKind {
    Cm,
    Ch,
    Cs,
}

impl CorePairKind {
    pub const F_SEQUENCE: [Self; 6] = [Self::Cm, Self::Ch, Self::Cs, Self::Cm, Self::Ch, Self::Cs];
    pub fn carbon(self) -> &'static str { "Carbon" }
    pub fn partner(self) -> &'static str { match self { Self::Cm => "Methane", Self::Ch => "Hydrogen", Self::Cs => "Sulfur" } }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoreConstituentPlacement {
    pub resource_name: &'static str,
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoreUnitPlacement {
    pub kind: CorePairKind,
    pub center_x: f64,
    pub center_y: f64,
    pub radial_angle_radians: f64,
    pub carbon: CoreConstituentPlacement,
    pub partner: CoreConstituentPlacement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreGeometry {
    pub units: Vec<CoreUnitPlacement>,
    pub cavity_radius: f64,
    pub outer_radius: f64,
    pub ring_radius: f64,
}

pub fn build_f_core(catalog: &[BaseResource]) -> Option<CoreGeometry> {
    let carbon_radius = resource_bounding_radius(catalog, "Carbon")?;
    let partner_radii = [resource_bounding_radius(catalog, "Methane")?, resource_bounding_radius(catalog, "Hydrogen")?, resource_bounding_radius(catalog, "Sulfur")?];
    let pair_outer_envelopes = partner_radii.map(|partner| { let separation = carbon_radius + partner; separation / 2.0 + partner });
    let pair_inner_envelopes = partner_radii.map(|partner| { let separation = carbon_radius + partner; separation / 2.0 + carbon_radius });
    let indices = [0usize, 1, 2, 0, 1, 2];
    let ring_radius = solve_ring_radius(&pair_outer_envelopes, &indices)?;
    let max_inner = pair_inner_envelopes.iter().copied().fold(0.0_f64, f64::max);
    let max_outer = pair_outer_envelopes.iter().copied().fold(0.0_f64, f64::max);
    let cavity_radius = ring_radius - max_inner;
    if cavity_radius <= 0.0 { return None; }
    let outer_radius = ring_radius + max_outer;
    let mut units = Vec::with_capacity(6);
    let mut angle = std::f64::consts::FRAC_PI_2;
    for index in 0..6 {
        let kind = CorePairKind::F_SEQUENCE[index];
        let partner_radius = partner_radii[indices[index]];
        let separation = carbon_radius + partner_radius;
        let (radial_x, radial_y) = (angle.cos(), angle.sin());
        let center_x = ring_radius * radial_x;
        let center_y = ring_radius * radial_y;
        units.push(CoreUnitPlacement { kind, center_x, center_y, radial_angle_radians: angle,
            carbon: CoreConstituentPlacement { resource_name: kind.carbon(), x: center_x - separation / 2.0 * radial_x, y: center_y - separation / 2.0 * radial_y, rotation_radians: angle },
            partner: CoreConstituentPlacement { resource_name: kind.partner(), x: center_x + separation / 2.0 * radial_x, y: center_y + separation / 2.0 * radial_y, rotation_radians: angle } });
        let next = indices[(index + 1) % 6];
        let current = indices[index];
        angle -= 2.0 * ((pair_outer_envelopes[current] + pair_outer_envelopes[next]) / (2.0 * ring_radius)).asin();
    }
    Some(CoreGeometry { units, cavity_radius, outer_radius, ring_radius })
}

fn solve_ring_radius(envelopes: &[f64; 3], sequence: &[usize; 6]) -> Option<f64> {
    let max_sum = (0..6).map(|i| envelopes[sequence[i]] + envelopes[sequence[(i + 1) % 6]]).fold(0.0_f64, f64::max);
    let mut low = max_sum / 2.0;
    let mut high = max_sum;
    for _ in 0..100 {
        let radius = (low + high) / 2.0;
        let angle_sum = (0..6).map(|i| { let sum = envelopes[sequence[i]] + envelopes[sequence[(i + 1) % 6]]; 2.0 * (sum / (2.0 * radius)).asin() }).sum::<f64>();
        if angle_sum > std::f64::consts::TAU { low = radius; } else { high = radius; }
    }
    let radius = (low + high) / 2.0;
    if radius.is_finite() && radius > 0.0 { Some(radius) } else { None }
}

fn resource_bounding_radius(catalog: &[BaseResource], name: &str) -> Option<f64> {
    catalog.iter().find(|resource| resource.name == name).map(|resource| resource.shape.form.bounding_radius())
}
