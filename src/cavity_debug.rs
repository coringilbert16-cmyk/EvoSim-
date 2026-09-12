#[cfg(test)]
mod tests {
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;

    #[test]
    fn print_seed_geometry() {
        let catalog = default_catalog();
        let structure = initial_genome().structural_blueprint.realize(&catalog).unwrap();
        for (i, unit) in structure.units.iter().enumerate() {
            println!("unit {i}: placement=({:.6},{:.6}) rotation={:.6} shape={:?}", unit.placement.x, unit.placement.y, unit.placement.rotation_radians, unit.shape(&catalog));
        }
        for (i, bond) in structure.bonds.iter().enumerate() {
            println!("bond {i}: {:?} -> {:?}", bond.endpoint_a, bond.endpoint_b);
        }
        panic!("temporary seed geometry diagnostic — inspect realized placement and bond output");
    }
}
