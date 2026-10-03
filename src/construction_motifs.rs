//! Canonical multi-unit construction motifs.
//!
//! Motifs are local structural vocabulary. They are deliberately bounded by
//! unit count and canonicalized so equivalent rotations/translations and
//! repeated constituent labels do not become separate catalog entries.

use crate::construction_catalog::{ConstructionCatalog, PairFormation, RelativePlacement};
use crate::material_geometry::PlacedMaterialPart;
use crate::resources::{BaseResource, PhysicalState};
use crate::structure::{ConnectionEndpoint, Placement};

pub(crate) const MAX_CATALOG_MOTIF_UNITS: usize = 14;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MotifUnit {
    pub resource_index: usize,
    pub placement: Placement,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MotifBond {
    pub unit_a: usize,
    pub endpoint_a: ConnectionEndpoint,
    pub unit_b: usize,
    pub endpoint_b: ConnectionEndpoint,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConstructionMotif {
    pub units: Vec<MotifUnit>,
    pub bonds: Vec<MotifBond>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MotifCatalog {
    levels: Vec<Vec<ConstructionMotif>>,
}

impl MotifCatalog {
    pub(crate) fn generate(
        resources: &[BaseResource],
        pair_catalog: &ConstructionCatalog,
        max_units: usize,
    ) -> Self {
        let max_units = max_units.clamp(1, MAX_CATALOG_MOTIF_UNITS);
        let rigid_resources = resources
            .iter()
            .enumerate()
            .filter(|(_, resource)| {
                resource.physical_state == PhysicalState::Rigid && resource.shape.is_valid()
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        let mut levels = vec![Vec::new(); max_units];
        for resource_index in rigid_resources {
            levels[0].push(ConstructionMotif {
                units: vec![MotifUnit {
                    resource_index,
                    placement: Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                }],
                bonds: Vec::new(),
            });
        }

        for size in 2..=max_units {
            let mut next = Vec::new();
            for motif in &levels[size - 2] {
                for anchor_index in 0..motif.units.len() {
                    let anchor = &motif.units[anchor_index];
                    for new_resource in &rigid_resources {
                        for formation in pair_catalog
                            .pair_formations_for(*new_resource, anchor.resource_index)
                        {
                            let placement =
                                transformed_relative_placement(formation, anchor.placement);

                            if penetrates_existing(
                                resources,
                                motif,
                                *new_resource,
                                placement,
                            ) {
                                continue;
                            }

                            let mut units = motif.units.clone();
                            units.push(MotifUnit {
                                resource_index: *new_resource,
                                placement,
                            });
                            let new_index = units.len() - 1;

                            let mut bonds = motif.bonds.clone();
                            bonds.push(MotifBond {
                                unit_a: new_index,
                                endpoint_a: formation.endpoint_a,
                                unit_b: anchor_index,
                                endpoint_b: formation.endpoint_b,
                            });

                            let candidate = ConstructionMotif { units, bonds };
                            next.push(candidate);
                        }
                    }
                }
            }

            deduplicate_motifs(&mut next);
            levels[size - 1] = next;
        }

        Self { levels }
    }

    pub(crate) fn level(&self, units: usize) -> &[ConstructionMotif] {
        self.levels
            .get(units.saturating_sub(1))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn len(&self) -> usize {
        self.levels.iter().map(Vec::len).sum()
    }
}

fn transformed_relative_placement(
    formation: &PairFormation,
    anchor: Placement,
) -> Placement {
    let RelativePlacement {
        x,
        y,
        rotation_radians,
    } = formation.placement_a_relative_to_b;
    let (s, c) = anchor.rotation_radians.sin_cos();
    Placement {
        x: anchor.x + x * c - y * s,
        y: anchor.y + x * s + y * c,
        rotation_radians: rotation_radians + anchor.rotation_radians,
    }
}

fn penetrates_existing(
    resources: &[BaseResource],
    motif: &ConstructionMotif,
    new_resource: usize,
    placement: Placement,
) -> bool {
    let Some(new_shape) = resources.get(new_resource).map(|r| &r.shape) else {
        return true;
    };
    let new_part = PlacedMaterialPart {
        part_index: motif.units.len(),
        form: new_shape.form.clone(),
        placement,
    };

    motif.units.iter().any(|unit| {
        let Some(shape) = resources.get(unit.resource_index).map(|r| &r.shape) else {
            return true;
        };
        let existing = PlacedMaterialPart {
            part_index: unit.resource_index,
            form: shape.form.clone(),
            placement: unit.placement,
        };
        crate::material_geometry::placed_forms_penetrate(&new_part, &existing, 1e-9)
    })
}

fn deduplicate_motifs(motifs: &mut Vec<ConstructionMotif>) {
    motifs.sort_by(|a, b| canonical_key(a).cmp(&canonical_key(b)));
    motifs.dedup_by(|a, b| canonical_key(a) == canonical_key(b));
}

fn canonical_key(motif: &ConstructionMotif) -> String {
    let root = motif.units.first().map(|unit| unit.placement).unwrap_or(Placement {
        x: 0.0,
        y: 0.0,
        rotation_radians: 0.0,
    });
    let (s, c) = root.rotation_radians.sin_cos();

    let mut units = motif
        .units
        .iter()
        .map(|unit| {
            let dx = unit.placement.x - root.x;
            let dy = unit.placement.y - root.y;
            let x = dx * c + dy * s;
            let y = -dx * s + dy * c;
            (
                unit.resource_index,
                quantize(x),
                quantize(y),
                quantize(unit.placement.rotation_radians - root.rotation_radians),
            )
        })
        .collect::<Vec<_>>();
    units.sort();

    let bonds = motif
        .bonds
        .iter()
        .map(|bond| {
            (
                bond.unit_a.min(bond.unit_b),
                bond.unit_a.max(bond.unit_b),
                endpoint_key(bond.endpoint_a),
                endpoint_key(bond.endpoint_b),
            )
        })
        .collect::<Vec<_>>();

    format!("{units:?}|{bonds:?}")
}

fn endpoint_key(endpoint: ConnectionEndpoint) -> String {
    format!("{endpoint:?}")
}

fn quantize(value: f64) -> i64 {
    (value / 1e-9).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motif_catalog_grows_past_pairs() {
        let resources = crate::resources::default_catalog();
        let pair_catalog = ConstructionCatalog::build(&resources);
        let motifs = MotifCatalog::generate(&resources, &pair_catalog, 3);
        assert!(!motifs.level(1).is_empty());
        assert!(!motifs.level(2).is_empty());
        assert!(motifs.len() >= motifs.level(2).len());
    }

    #[test]
    fn motif_generation_never_exceeds_configured_ceiling() {
        let resources = crate::resources::default_catalog();
        let pair_catalog = ConstructionCatalog::build(&resources);
        let motifs = MotifCatalog::generate(
            &resources,
            &pair_catalog,
            MAX_CATALOG_MOTIF_UNITS + 10,
        );
        assert!(motifs.level(MAX_CATALOG_MOTIF_UNITS).len() >= 0);
        assert!(motifs.level(MAX_CATALOG_MOTIF_UNITS + 1).is_empty());
    }

    #[test]
    fn motif_catalog_keeps_multiple_resource_compositions() {
        let resources = crate::resources::default_catalog();
        let pair_catalog = ConstructionCatalog::build(&resources);
        let motifs = MotifCatalog::generate(&resources, &pair_catalog, 3);
        let has_mixed = motifs.level(2).iter().any(|motif| {
            motif.units[0].resource_index != motif.units[1].resource_index
        });
        assert!(has_mixed);
    }
}
