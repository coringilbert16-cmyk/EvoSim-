#[cfg(test)]
mod tests {
    use crate::architecture::{default_architecture, JUVENILE_LINEAR_SCALE};
    use crate::resources::default_catalog;

    #[test]
    fn developmental_realization_uses_a_discrete_juvenile_analog() {
        let architecture = default_architecture();
        let catalog = default_catalog();
        let adult = architecture.adult_construction_target().unwrap();
        // Juvenile realization is allowed to use fewer rigid constituents while
        // preserving the same construction-anchor mechanism.
        let juvenile = architecture
            .developmental_target(JUVENILE_LINEAR_SCALE, &catalog)
            .unwrap();
        assert!(adult.is_connected());
        assert!(juvenile.elements.len() < adult.elements.len());
        assert_eq!(juvenile.anchor_elements.len(), adult.anchor_elements.len());
        assert!(juvenile.is_connected());
    }
}
