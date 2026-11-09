#![expect(
    dead_code,
    reason = "Stage 3 broad phase is being parity-tested before production cutover"
)]
//! Shared environmental material snapshot and read-only pair traversal.
//!
//! The all-pairs window remains a conservative reference oracle. A separate
//! spatial window derives realized-material bounds and queries intersecting
//! field cells, preserving unordered COMBINE pairs and directed DISPLACEMENT
//! roles. Reaction-specific endpoint geometry, activation, scheduling,
//! funding, and commits remain in their existing subsystems.

use crate::environment::ActiveMaterialField;
use crate::environmental_interaction::BondRef;
use crate::environmental_interaction::ConstituentRef;
use crate::environmental_interaction::ExistingBondRecord;
use crate::environmental_interaction::MaterialIdentity;
use crate::environmental_interaction::MaterialLocator;
use crate::environmental_interaction::MaterialRecord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MaterialPairRecord {
    pub(crate) left: usize,
    pub(crate) right: usize,
    pub(crate) identity_left: MaterialIdentity,
    pub(crate) identity_right: MaterialIdentity,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct EnvironmentalMaterialSnapshot {
    pub(crate) pass_id: u64,
    /// Coarse field mutation epoch captured with this pass snapshot. It is an
    /// invalidation token, not a per-material topology or state version.
    pub(crate) field_revision: u64,
    pub(crate) materials: Vec<MaterialRecord>,
    pub(crate) existing_bonds: Vec<ExistingBondRecord>,
    pub(crate) cell_visits: u64,
    pub(crate) material_visits: u64,
    pub(crate) unassigned_materials: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PairWindowCoverage {
    pub(crate) known_pairs: u64,
    pub(crate) visited_pairs: u64,
    pub(crate) deferred_pairs: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaterialSpatialBounds {
    pub(crate) min_x: f64,
    pub(crate) max_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
}

impl MaterialSpatialBounds {
    fn expanded(self, padding: f64) -> Option<Self> {
        if !padding.is_finite()
            || padding < 0.0
            || !self.min_x.is_finite()
            || !self.max_x.is_finite()
            || !self.min_y.is_finite()
            || !self.max_y.is_finite()
            || self.min_x > self.max_x
            || self.min_y > self.max_y
        {
            return None;
        }
        Some(Self {
            min_x: self.min_x - padding,
            max_x: self.max_x + padding,
            min_y: self.min_y - padding,
            max_y: self.max_y + padding,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct MaterialBoundsProgress {
    material_index: usize,
    next_constituent: usize,
    valid: bool,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl MaterialBoundsProgress {
    fn new(material_index: usize, valid: bool) -> Self {
        Self {
            material_index,
            next_constituent: 0,
            valid,
            min_x: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            min_y: f64::INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    fn finish(self) -> Option<MaterialSpatialBounds> {
        (self.valid
            && self.min_x.is_finite()
            && self.max_x.is_finite()
            && self.min_y.is_finite()
            && self.max_y.is_finite()
            && self.min_x <= self.max_x
            && self.min_y <= self.max_y)
            .then_some(MaterialSpatialBounds {
                min_x: self.min_x,
                max_x: self.max_x,
                min_y: self.min_y,
                max_y: self.max_y,
            })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct MaterialBoundsCursorPage {
    pub(crate) material_headers_inspected: u64,
    pub(crate) constituent_inspections: u64,
    pub(crate) visited_work: u64,
    pub(crate) deferred_work: u64,
    pub(crate) completed_materials: u64,
    pub(crate) complete: bool,
    /// Available once after every material header and constituent has been
    /// visited. The vector is aligned with the snapshot material records.
    pub(crate) bounds: Option<Vec<Option<MaterialSpatialBounds>>>,
}

/// Resumable preparation of conservative material bounds. Each call performs
/// at most work_limit units: one material-header validation or one
/// constituent shape/placement inspection. It never restores a structure or
/// performs constituent penetration checks.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaterialBoundsCursor {
    pass_id: u64,
    field_revision: u64,
    next_material: usize,
    current: Option<MaterialBoundsProgress>,
    bounds: Option<Vec<Option<MaterialSpatialBounds>>>,
    catalog_indices: std::collections::BTreeMap<String, usize>,
    catalog_valid: bool,
    catalog_len: usize,
    known_work: u64,
    visited_work: u64,
    complete: bool,
}

impl MaterialBoundsCursor {
    pub(crate) fn new(
        snapshot: &EnvironmentalMaterialSnapshot,
        catalog: &[crate::resources::BaseResource],
    ) -> Self {
        let mut catalog_indices = std::collections::BTreeMap::new();
        let mut catalog_valid = true;
        for (index, resource) in catalog.iter().enumerate() {
            if resource.name.is_empty()
                || catalog_indices
                    .insert(resource.name.clone(), index)
                    .is_some()
            {
                catalog_valid = false;
            }
        }
        Self {
            pass_id: snapshot.pass_id,
            field_revision: snapshot.field_revision,
            next_material: 0,
            current: None,
            bounds: Some(vec![None; snapshot.materials.len()]),
            catalog_indices,
            catalog_valid,
            catalog_len: catalog.len(),
            known_work: snapshot
                .materials
                .iter()
                .fold(snapshot.materials.len() as u64, |total, material| {
                    total.saturating_add(material.constituent_count as u64)
                }),
            visited_work: 0,
            complete: snapshot.materials.is_empty(),
        }
    }

    pub(crate) fn next_page(
        &mut self,
        snapshot: &EnvironmentalMaterialSnapshot,
        field: &ActiveMaterialField,
        catalog: &[crate::resources::BaseResource],
        work_limit: usize,
    ) -> MaterialBoundsCursorPage {
        if snapshot.pass_id != self.pass_id
            || snapshot.field_revision != self.field_revision
            || field.revision != self.field_revision
            || (!self.complete
                && self
                    .bounds
                    .as_ref()
                    .is_none_or(|bounds| snapshot.materials.len() != bounds.len()))
            || catalog.len() != self.catalog_len
        {
            return MaterialBoundsCursorPage {
                deferred_work: self.known_work.saturating_sub(self.visited_work),
                ..MaterialBoundsCursorPage::default()
            };
        }

        let mut page = MaterialBoundsCursorPage::default();
        while page.visited_work < work_limit as u64 && !self.complete {
            if self.current.is_none() {
                let material_index = self.next_material;
                let Some(record) = snapshot.materials.get(material_index) else {
                    self.complete = true;
                    break;
                };
                self.next_material += 1;
                page.material_headers_inspected = page.material_headers_inspected.saturating_add(1);
                page.visited_work = page.visited_work.saturating_add(1);
                self.visited_work = self.visited_work.saturating_add(1);

                let physical = field
                    .cells
                    .get(record.locator.cell_index)
                    .and_then(|cell| cell.physical_materials.get(record.locator.material_index));
                let valid = self.catalog_valid
                    && physical.is_some_and(|physical| {
                        physical.environmental_identity == record.identity.0
                            && !physical.material.parts.is_empty()
                            && physical.material.parts.len() == record.constituent_count
                            && physical.placements.as_ref().is_some_and(|placements| {
                                placements.len() == record.constituent_count
                            })
                            && physical
                                .realized_shapes
                                .as_ref()
                                .is_none_or(|shapes| shapes.len() == record.constituent_count)
                    });
                self.current = Some(MaterialBoundsProgress::new(material_index, valid));
                continue;
            }

            let mut progress = self.current.take().expect("current bounds work exists");
            let record = &snapshot.materials[progress.material_index];
            let physical = field
                .cells
                .get(record.locator.cell_index)
                .and_then(|cell| cell.physical_materials.get(record.locator.material_index));
            if progress.valid {
                let resolved = physical.and_then(|physical| {
                    if physical.environmental_identity != record.identity.0 {
                        return None;
                    }
                    let (name, amount) = physical.material.parts.get(progress.next_constituent)?;
                    let resource_index = *self.catalog_indices.get(name)?;
                    let resource = catalog.get(resource_index)?;
                    if name.is_empty()
                        || !amount.is_finite()
                        || *amount < 1.0
                        || *amount >= u64::MAX as f64
                        || amount.fract() != 0.0
                        || ((*amount - 1.0).abs() > 1e-9
                            && !(physical.material.parts.len() == 1
                                && physical.material.internal_bonds.is_empty()
                                && resource.physical_state
                                    == crate::resources::PhysicalState::Fluid))
                    {
                        return None;
                    }
                    let placement = *physical
                        .placements
                        .as_ref()?
                        .get(progress.next_constituent)?;
                    if !placement.x.is_finite()
                        || !placement.y.is_finite()
                        || !placement.rotation_radians.is_finite()
                    {
                        return None;
                    }
                    let shape = match physical.realized_shapes.as_ref() {
                        Some(shapes) => shapes.get(progress.next_constituent)?,
                        None => &resource.shape,
                    };
                    if !shape.is_valid()
                        || (shape != &resource.shape
                            && resource.physical_state != crate::resources::PhysicalState::Fluid)
                    {
                        return None;
                    }
                    if resource.physical_state == crate::resources::PhysicalState::Fluid {
                        let area = |shape: &crate::resources::Shape| match &shape.form {
                            crate::resources::Form::Circle { radius } => {
                                Some(std::f64::consts::PI * radius * radius)
                            }
                            crate::resources::Form::Fluid { nominal_area, .. } => {
                                Some(*nominal_area)
                            }
                            _ => None,
                        };
                        let default_area = area(&resource.shape)?;
                        let realized_area = area(shape)?;
                        let expected_area = default_area * *amount;
                        if !expected_area.is_finite()
                            || (realized_area - expected_area).abs() > 1e-9 * expected_area.max(1.0)
                        {
                            return None;
                        }
                    }
                    let radius = shape.form.bounding_radius();
                    (radius.is_finite() && radius >= 0.0).then_some((
                        placement.x,
                        placement.y,
                        radius,
                    ))
                });
                if let Some((x, y, radius)) = resolved {
                    progress.min_x = progress.min_x.min(x - radius);
                    progress.max_x = progress.max_x.max(x + radius);
                    progress.min_y = progress.min_y.min(y - radius);
                    progress.max_y = progress.max_y.max(y + radius);
                } else {
                    progress.valid = false;
                }
            }

            progress.next_constituent += 1;
            page.constituent_inspections = page.constituent_inspections.saturating_add(1);
            page.visited_work = page.visited_work.saturating_add(1);
            self.visited_work = self.visited_work.saturating_add(1);
            if progress.next_constituent >= record.constituent_count {
                let material_index = progress.material_index;
                self.bounds
                    .as_mut()
                    .expect("bounds remain available until completion")[material_index] =
                    progress.finish();
                page.completed_materials = page.completed_materials.saturating_add(1);
            } else {
                self.current = Some(progress);
            }
        }

        self.complete = self.current.is_none() && self.next_material >= snapshot.materials.len();
        page.deferred_work = self.known_work.saturating_sub(self.visited_work);
        page.complete = self.complete;
        if self.complete {
            page.bounds = self.bounds.take();
        }
        page
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PairOrientation {
    /// One pair, deduplicated across both possible spatial query directions.
    Unordered,
    /// Every anchor-to-target direction. DISPLACEMENT needs both roles.
    Directed,
    /// Only forward field-locator pairs. Backward cell/material slots are
    /// skipped without consuming work, matching the legacy COMBINE scan.
    ForwardOrdered,
}

impl EnvironmentalMaterialSnapshot {
    /// Build one pass-local ownership snapshot. Topology generation is scoped
    /// to this immutable snapshot; it is not a fabricated persistent version.
    pub(crate) fn from_field(field: &ActiveMaterialField, pass_id: u64) -> Self {
        let mut snapshot = Self {
            pass_id,
            field_revision: field.revision,
            cell_visits: field.cells.len() as u64,
            ..Self::default()
        };
        for (cell_index, cell) in field.cells.iter().enumerate() {
            for (material_index, physical) in cell.physical_materials.iter().enumerate() {
                snapshot.material_visits = snapshot.material_visits.saturating_add(1);
                let identity = MaterialIdentity(physical.environmental_identity);
                if identity.0 == 0 {
                    snapshot.unassigned_materials = snapshot.unassigned_materials.saturating_add(1);
                    continue;
                }
                let topology_generation = 0;
                snapshot.materials.push(MaterialRecord {
                    identity,
                    locator: MaterialLocator {
                        cell_index,
                        material_index,
                    },
                    topology_generation,
                    state_version: 0,
                    constituent_count: physical.material.parts.len(),
                });
                if let Some(bonds) = physical.internal_connections.as_ref() {
                    for (bond_index, bond) in bonds.iter().enumerate() {
                        let (Some(constituent_a), Some(constituent_b)) = (
                            (bond.part_a < physical.material.parts.len()).then_some(bond.part_a),
                            (bond.part_b < physical.material.parts.len()).then_some(bond.part_b),
                        ) else {
                            continue;
                        };
                        snapshot.existing_bonds.push(ExistingBondRecord {
                            material: identity,
                            topology_generation,
                            bond_index,
                            constituent_a: ConstituentRef {
                                material: identity,
                                topology_generation,
                                local_index: constituent_a,
                            },
                            endpoint_a: bond.endpoint_a,
                            constituent_b: ConstituentRef {
                                material: identity,
                                topology_generation,
                                local_index: constituent_b,
                            },
                            endpoint_b: bond.endpoint_b,
                            bond_energy: bond.bond_energy,
                        });
                    }
                }
            }
        }
        snapshot.materials.sort_by_key(|material| material.identity);
        snapshot.existing_bonds.sort_by_key(|bond| bond.reference());
        snapshot
    }

    pub(crate) fn known_pair_count(&self) -> u64 {
        let count = self.materials.len() as u64;
        count.saturating_mul(count.saturating_sub(1)) / 2
    }

    /// Count all ordered, non-self material pairs. This is the conservative
    /// reference universe for role-sensitive DISPLACEMENT discovery.
    pub(crate) fn known_directed_pair_count(&self) -> u64 {
        let count = self.materials.len() as u64;
        count.saturating_mul(count.saturating_sub(1))
    }

    /// Return a deterministic window over every anchor-to-incoming direction.
    /// Each ordered pair appears once; the reverse direction is a distinct role.
    pub(crate) fn directed_pair_window(
        &self,
        start: u64,
        limit: usize,
    ) -> (Vec<MaterialPairRecord>, PairWindowCoverage) {
        let known_pairs = self.known_directed_pair_count();
        let start = start.min(known_pairs);
        let count = (known_pairs - start).min(limit as u64);
        let mut pairs = Vec::with_capacity(count as usize);
        let mut ordinal = 0u64;
        for anchor in 0..self.materials.len() {
            for incoming in 0..self.materials.len() {
                if anchor == incoming {
                    continue;
                }
                if ordinal >= start && pairs.len() < count as usize {
                    let a = &self.materials[anchor];
                    let b = &self.materials[incoming];
                    pairs.push(MaterialPairRecord {
                        left: anchor,
                        right: incoming,
                        identity_left: a.identity,
                        identity_right: b.identity,
                    });
                }
                ordinal = ordinal.saturating_add(1);
                if pairs.len() == count as usize {
                    break;
                }
            }
            if pairs.len() == count as usize {
                break;
            }
        }
        let visited_pairs = pairs.len() as u64;
        (
            pairs,
            PairWindowCoverage {
                known_pairs,
                visited_pairs,
                deferred_pairs: known_pairs.saturating_sub(start + visited_pairs),
            },
        )
    }

    /// Return a deterministic window over all unordered material pairs.
    /// `start` is a pair ordinal, not a material index or cell locator.
    pub(crate) fn pair_window(
        &self,
        start: u64,
        limit: usize,
    ) -> (Vec<MaterialPairRecord>, PairWindowCoverage) {
        let known_pairs = self.known_pair_count();
        let start = start.min(known_pairs);
        let count = (known_pairs - start).min(limit as u64);
        let mut pairs = Vec::with_capacity(count as usize);
        let mut ordinal = 0u64;
        for left in 0..self.materials.len() {
            for right in left + 1..self.materials.len() {
                if ordinal >= start && pairs.len() < count as usize {
                    let a = &self.materials[left];
                    let b = &self.materials[right];
                    pairs.push(MaterialPairRecord {
                        left,
                        right,
                        identity_left: a.identity,
                        identity_right: b.identity,
                    });
                }
                ordinal = ordinal.saturating_add(1);
                if pairs.len() == count as usize {
                    break;
                }
            }
            if pairs.len() == count as usize {
                break;
            }
        }
        let visited_pairs = pairs.len() as u64;
        (
            pairs,
            PairWindowCoverage {
                known_pairs,
                visited_pairs,
                deferred_pairs: known_pairs.saturating_sub(start + visited_pairs),
            },
        )
    }

    /// Build exact broad-phase bounds by restoring each snapshot material
    /// with the same origin pose used by the live environmental reaction paths.
    /// Invalid, unrealized, or stale locators produce no spatial bounds and
    /// therefore cannot create a false candidate.
    pub(crate) fn spatial_bounds_from_field(
        &self,
        field: &ActiveMaterialField,
        catalog: &[crate::resources::BaseResource],
    ) -> Vec<Option<MaterialSpatialBounds>> {
        self.materials
            .iter()
            .map(|record| {
                let physical = field
                    .cells
                    .get(record.locator.cell_index)?
                    .physical_materials
                    .get(record.locator.material_index)?;
                if physical.environmental_identity != record.identity.0
                    || !physical.is_realized()
                    || physical.material.is_empty()
                    || !physical.material.is_valid_for_catalog(catalog)
                {
                    return None;
                }
                let mut structure = crate::structure::OrganismStructure::new();
                crate::material_restoration::restore_material(
                    &mut structure,
                    physical,
                    crate::structure::Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                    catalog,
                )?;
                let body = crate::organism_geometry::OrganismBodyGeometry::from_all_realized_units(
                    &structure,
                )?;
                Some(MaterialSpatialBounds {
                    min_x: body.min_x,
                    max_x: body.max_x,
                    min_y: body.min_y,
                    max_y: body.max_y,
                })
            })
            .collect()
    }

    /// Enumerate spatially plausible material pairs using the same cell-bound
    /// query used by the live COMBINE and DISPLACEMENT discovery paths.
    ///
    /// The bounds slice is aligned with self.materials and must be computed from
    /// each restored material's complete realized geometry. The cell query is
    /// only a broad-phase filter; constituent endpoint geometry remains
    /// authoritative. COMBINE requests unordered pairs, while DISPLACEMENT
    /// requests both directed anchor/incoming orientations.
    pub(crate) fn spatial_pair_window(
        &self,
        field: &ActiveMaterialField,
        bounds: &[Option<MaterialSpatialBounds>],
        padding: f64,
        start: u64,
        limit: usize,
        orientation: PairOrientation,
    ) -> (Vec<MaterialPairRecord>, PairWindowCoverage) {
        if bounds.len() != self.materials.len() {
            return (
                Vec::new(),
                PairWindowCoverage {
                    known_pairs: 0,
                    visited_pairs: 0,
                    deferred_pairs: 0,
                },
            );
        }
        let mut locator_to_material = std::collections::BTreeMap::new();
        for (index, material) in self.materials.iter().enumerate() {
            locator_to_material.insert(
                (material.locator.cell_index, material.locator.material_index),
                index,
            );
        }
        let mut unordered_candidates = std::collections::BTreeMap::new();
        let mut directed_candidates = Vec::new();
        for (anchor_index, anchor) in self.materials.iter().enumerate() {
            let Some(expanded) = bounds[anchor_index].and_then(|value| value.expanded(padding))
            else {
                continue;
            };
            let target_cells = field.cells_intersecting_bounds(
                expanded.min_x,
                expanded.max_x,
                expanded.min_y,
                expanded.max_y,
            );
            for cell_index in target_cells {
                let Some(cell) = field.cells.get(cell_index) else {
                    continue;
                };
                for target_slot in 0..cell.physical_materials.len() {
                    let Some(&target_index) = locator_to_material.get(&(cell_index, target_slot))
                    else {
                        continue;
                    };
                    if target_index == anchor_index {
                        continue;
                    }
                    let target = &self.materials[target_index];
                    if orientation == PairOrientation::Unordered {
                        // Query both material bounds before canonicalizing. A
                        // physically overlapping target may occupy a cell that
                        // is reached only by the target's expanded bounds.
                        // Keeping the union avoids a direction-dependent miss.
                        let anchor_locator =
                            (anchor.locator.cell_index, anchor.locator.material_index);
                        let target_locator =
                            (target.locator.cell_index, target.locator.material_index);
                        let (left, right) = if anchor_locator < target_locator {
                            (anchor_index, target_index)
                        } else {
                            (target_index, anchor_index)
                        };
                        unordered_candidates
                            .entry((left, right))
                            .or_insert_with(|| {
                                let left_material = &self.materials[left];
                                let right_material = &self.materials[right];
                                MaterialPairRecord {
                                    left,
                                    right,
                                    identity_left: left_material.identity,
                                    identity_right: right_material.identity,
                                }
                            });
                    } else {
                        let anchor_locator =
                            (anchor.locator.cell_index, anchor.locator.material_index);
                        let target_locator =
                            (target.locator.cell_index, target.locator.material_index);
                        if orientation == PairOrientation::ForwardOrdered
                            && anchor_locator >= target_locator
                        {
                            continue;
                        }
                        directed_candidates.push(MaterialPairRecord {
                            left: anchor_index,
                            right: target_index,
                            identity_left: anchor.identity,
                            identity_right: target.identity,
                        });
                    }
                }
            }
        }
        let candidates = match orientation {
            PairOrientation::Unordered => unordered_candidates.into_values().collect::<Vec<_>>(),
            PairOrientation::Directed | PairOrientation::ForwardOrdered => directed_candidates,
        };
        let known_pairs = candidates.len() as u64;
        let start = start.min(known_pairs);
        let count = (known_pairs - start).min(limit as u64) as usize;
        let pairs = candidates
            .into_iter()
            .skip(start as usize)
            .take(count)
            .collect::<Vec<_>>();
        let visited_pairs = pairs.len() as u64;
        (
            pairs,
            PairWindowCoverage {
                known_pairs,
                visited_pairs,
                deferred_pairs: known_pairs.saturating_sub(start + visited_pairs),
            },
        )
    }
}

/// Work-bounded cursor over the exhaustive material-pair universe. Unlike
/// `spatial_pair_window`, this never constructs the complete candidate set.
/// Bounds are supplied by a separately prepared snapshot phase; this cursor
/// accounts for every pair predicate it evaluates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpatialPairCursor {
    pass_id: u64,
    field_revision: u64,
    orientation: PairOrientation,
    next_left: usize,
    next_right: usize,
    visited_pair_checks: u64,
    complete: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpatialCursorPage {
    pub(crate) pairs: Vec<MaterialPairRecord>,
    pub(crate) known_pair_checks: u64,
    pub(crate) visited_pair_checks: u64,
    pub(crate) deferred_pair_checks: u64,
    pub(crate) complete: bool,
}

impl SpatialPairCursor {
    pub(crate) fn new(
        snapshot: &EnvironmentalMaterialSnapshot,
        orientation: PairOrientation,
    ) -> Self {
        let next_right = 1;
        Self {
            pass_id: snapshot.pass_id,
            field_revision: snapshot.field_revision,
            orientation,
            next_left: 0,
            next_right,
            visited_pair_checks: 0,
            complete: snapshot.materials.len() < 2,
        }
    }

    /// Evaluate at most `work_limit` material-pair predicates and emit at most
    /// `output_limit` candidates. A page can be empty but incomplete; callers
    /// must retain the cursor until `complete` is true.
    ///
    /// COMBINE's unordered predicate uses the union of both expanded-bounds
    /// cell queries. DISPLACEMENT evaluates every ordered anchor/incoming pair.
    pub(crate) fn next_page(
        &mut self,
        snapshot: &EnvironmentalMaterialSnapshot,
        field: &ActiveMaterialField,
        bounds: &[Option<MaterialSpatialBounds>],
        padding: f64,
        work_limit: usize,
        output_limit: usize,
    ) -> SpatialCursorPage {
        let known_pair_checks = match self.orientation {
            PairOrientation::Unordered => snapshot.known_pair_count(),
            PairOrientation::Directed | PairOrientation::ForwardOrdered => {
                snapshot.known_directed_pair_count()
            }
        };
        if snapshot.pass_id != self.pass_id
            || snapshot.field_revision != self.field_revision
            || field.revision != self.field_revision
            || bounds.len() != snapshot.materials.len()
        {
            return SpatialCursorPage {
                known_pair_checks,
                complete: false,
                ..SpatialCursorPage::default()
            };
        }

        let mut page = SpatialCursorPage {
            known_pair_checks,
            ..SpatialCursorPage::default()
        };
        if self.complete {
            page.complete = true;
            page.visited_pair_checks = self.visited_pair_checks;
            return page;
        }

        while page.visited_pair_checks < work_limit as u64
            && page.pairs.len() < output_limit
            && !self.complete
        {
            let left = self.next_left;
            let right = self.next_right;
            let left_record = &snapshot.materials[left];
            let right_record = &snapshot.materials[right];
            let left_bounds = bounds[left].and_then(|value| value.expanded(padding));
            let right_bounds = bounds[right].and_then(|value| value.expanded(padding));
            let left_reaches_right = left_bounds.is_some_and(|value| {
                bounds_include_cell(field, value, right_record.locator.cell_index)
            });
            let candidate = match self.orientation {
                PairOrientation::Directed => left_reaches_right,
                PairOrientation::ForwardOrdered => {
                    left_reaches_right
                        && (
                            left_record.locator.cell_index,
                            left_record.locator.material_index,
                        ) < (
                            right_record.locator.cell_index,
                            right_record.locator.material_index,
                        )
                }
                PairOrientation::Unordered => {
                    left_reaches_right
                        || right_bounds.is_some_and(|value| {
                            bounds_include_cell(field, value, left_record.locator.cell_index)
                        })
                }
            };
            if candidate {
                page.pairs.push(MaterialPairRecord {
                    left,
                    right,
                    identity_left: left_record.identity,
                    identity_right: right_record.identity,
                });
            }
            page.visited_pair_checks = page.visited_pair_checks.saturating_add(1);
            self.visited_pair_checks = self.visited_pair_checks.saturating_add(1);
            self.advance(snapshot.materials.len());
        }
        page.deferred_pair_checks = known_pair_checks.saturating_sub(self.visited_pair_checks);
        page.complete = self.complete;
        page
    }

    fn advance(&mut self, material_count: usize) {
        match self.orientation {
            PairOrientation::Unordered => {
                self.next_right += 1;
                if self.next_right >= material_count {
                    self.next_left += 1;
                    self.next_right = self.next_left + 1;
                }
                self.complete = self.next_left + 1 >= material_count;
            }
            PairOrientation::Directed | PairOrientation::ForwardOrdered => {
                self.next_right += 1;
                if self.next_right == self.next_left {
                    self.next_right += 1;
                }
                if self.next_right >= material_count {
                    self.next_left += 1;
                    self.next_right = 0;
                }
                self.complete = self.next_left >= material_count;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BoundsCellRange {
    min_col: usize,
    max_col: usize,
    row_start: isize,
    row_count: usize,
}

impl BoundsCellRange {
    fn new(field: &ActiveMaterialField, bounds: MaterialSpatialBounds) -> Option<Self> {
        if field.width_cells == 0
            || field.height_cells == 0
            || !field.cell_size.is_finite()
            || field.cell_size <= 0.0
        {
            return None;
        }
        let min_col = ((bounds.min_x / field.cell_size).floor() as isize).max(0) as usize;
        let max_col = ((bounds.max_x / field.cell_size).floor() as isize)
            .min(field.width_cells.saturating_sub(1) as isize)
            .max(0) as usize;
        if min_col > max_col {
            return None;
        }
        let row_start = (bounds.min_y / field.cell_size).floor() as isize;
        let row_end = (bounds.max_y / field.cell_size).floor() as isize;
        let row_count = row_end.saturating_sub(row_start).saturating_add(1) as usize;
        let (row_start, row_count) = if row_count >= field.height_cells {
            (0, field.height_cells)
        } else {
            (row_start, row_count)
        };
        Some(Self {
            min_col,
            max_col,
            row_start,
            row_count,
        })
    }

    fn cell_index(self, field: &ActiveMaterialField, row_offset: usize, col: usize) -> usize {
        let row =
            (self.row_start + row_offset as isize).rem_euclid(field.height_cells as isize) as usize;
        row * field.width_cells + col
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct IndexedSpatialCursorPage {
    pub(crate) pairs: Vec<MaterialPairRecord>,
    /// Work includes anchor initialization, cell visits, and material-slot
    /// inspections. Each unit is charged before its corresponding operation.
    pub(crate) visited_work: u64,
    pub(crate) visited_anchor_initializations: u64,
    pub(crate) visited_cell_checks: u64,
    pub(crate) visited_material_checks: u64,
    /// Number of anchor materials not yet fully traversed. This is a
    /// continuation indicator, not a count of deferred candidate pairs.
    pub(crate) deferred_anchors: u64,
    pub(crate) complete: bool,
}

/// Spatial-cell traversal that visits only cells reached by each material's
/// expanded bounds, then inspects the physical materials actually stored in
/// those cells. It preserves the existing field query semantics (X clamped,
/// Y wrapped) and resumes at the exact cell/material slot across pages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IndexedSpatialCursor {
    pass_id: u64,
    field_revision: u64,
    orientation: PairOrientation,
    field_width_cells: usize,
    field_height_cells: usize,
    field_cell_size_bits: u64,
    field_cell_count: usize,
    anchor_index: usize,
    anchors_visited: usize,
    anchor_initialized: bool,
    range: Option<BoundsCellRange>,
    row_offset: usize,
    col: usize,
    target_slot: usize,
    cell_loaded: bool,
    complete: bool,
}

impl IndexedSpatialCursor {
    pub(crate) fn new(
        snapshot: &EnvironmentalMaterialSnapshot,
        field: &ActiveMaterialField,
        orientation: PairOrientation,
    ) -> Self {
        Self::new_rotated(snapshot, field, orientation, 0)
    }

    pub(crate) fn new_rotated(
        snapshot: &EnvironmentalMaterialSnapshot,
        field: &ActiveMaterialField,
        orientation: PairOrientation,
        start_index: usize,
    ) -> Self {
        let material_count = snapshot.materials.len();
        Self {
            pass_id: snapshot.pass_id,
            field_revision: field.revision,
            orientation,
            field_width_cells: field.width_cells,
            field_height_cells: field.height_cells,
            field_cell_size_bits: field.cell_size.to_bits(),
            field_cell_count: field.cells.len(),
            anchor_index: if material_count == 0 {
                0
            } else {
                start_index % material_count
            },
            anchors_visited: 0,
            anchor_initialized: false,
            range: None,
            row_offset: 0,
            col: 0,
            target_slot: 0,
            cell_loaded: false,
            complete: material_count == 0,
        }
    }

    pub(crate) fn next_page(
        &mut self,
        snapshot: &EnvironmentalMaterialSnapshot,
        field: &ActiveMaterialField,
        bounds: &[Option<MaterialSpatialBounds>],
        padding: f64,
        work_limit: usize,
        output_limit: usize,
    ) -> IndexedSpatialCursorPage {
        if snapshot.pass_id != self.pass_id
            || snapshot.field_revision != self.field_revision
            || field.revision != self.field_revision
            || bounds.len() != snapshot.materials.len()
            || field.width_cells != self.field_width_cells
            || field.height_cells != self.field_height_cells
            || field.cell_size.to_bits() != self.field_cell_size_bits
            || field.cells.len() != self.field_cell_count
        {
            return IndexedSpatialCursorPage {
                deferred_anchors: snapshot.materials.len().saturating_sub(self.anchor_index) as u64,
                ..IndexedSpatialCursorPage::default()
            };
        }

        let mut page = IndexedSpatialCursorPage::default();
        while page.visited_work < work_limit as u64
            && page.pairs.len() < output_limit
            && !self.complete
        {
            if self.anchors_visited >= snapshot.materials.len() {
                self.complete = true;
                break;
            }

            if !self.anchor_initialized {
                page.visited_work = page.visited_work.saturating_add(1);
                page.visited_anchor_initializations =
                    page.visited_anchor_initializations.saturating_add(1);
                let expanded = bounds[self.anchor_index].and_then(|value| value.expanded(padding));
                self.range = expanded.and_then(|value| BoundsCellRange::new(field, value));
                self.anchor_initialized = true;
                self.row_offset = 0;
                self.col = self.range.map_or(0, |range| range.min_col);
                self.target_slot = 0;
                self.cell_loaded = false;
                if self.range.is_none_or(|range| range.row_count == 0) {
                    self.advance_anchor(snapshot.materials.len());
                }
                continue;
            }

            let Some(range) = self.range else {
                self.advance_anchor(snapshot.materials.len());
                continue;
            };
            if self.row_offset >= range.row_count {
                self.advance_anchor(snapshot.materials.len());
                continue;
            }

            if !self.cell_loaded {
                let cell_index = range.cell_index(field, self.row_offset, self.col);
                if self.orientation == PairOrientation::ForwardOrdered
                    && cell_index < snapshot.materials[self.anchor_index].locator.cell_index
                {
                    self.advance_cell(range);
                    continue;
                }
                page.visited_work = page.visited_work.saturating_add(1);
                page.visited_cell_checks = page.visited_cell_checks.saturating_add(1);
                self.cell_loaded = true;
                self.target_slot = 0;
                if field
                    .cells
                    .get(cell_index)
                    .is_none_or(|cell| cell.physical_materials.is_empty())
                {
                    self.advance_cell(range);
                }
                continue;
            }

            let cell_index = range.cell_index(field, self.row_offset, self.col);
            let Some(cell) = field.cells.get(cell_index) else {
                self.advance_cell(range);
                continue;
            };
            if self.target_slot >= cell.physical_materials.len() {
                self.advance_cell(range);
                continue;
            }

            let target_slot = self.target_slot;
            self.target_slot += 1;
            let anchor = &snapshot.materials[self.anchor_index];
            if self.orientation == PairOrientation::ForwardOrdered
                && (cell_index, target_slot)
                    <= (anchor.locator.cell_index, anchor.locator.material_index)
            {
                continue;
            }
            page.visited_work = page.visited_work.saturating_add(1);
            page.visited_material_checks = page.visited_material_checks.saturating_add(1);

            let physical = &cell.physical_materials[target_slot];
            if physical.environmental_identity == 0 {
                continue;
            }
            let identity = MaterialIdentity(physical.environmental_identity);
            let Ok(target_index) = snapshot
                .materials
                .binary_search_by_key(&identity, |record| record.identity)
            else {
                continue;
            };
            let target_record = &snapshot.materials[target_index];
            if target_record.locator.cell_index != cell_index
                || target_record.locator.material_index != target_slot
                || target_index == self.anchor_index
            {
                continue;
            }

            match self.orientation {
                PairOrientation::Directed | PairOrientation::ForwardOrdered => {
                    page.pairs.push(MaterialPairRecord {
                        left: self.anchor_index,
                        right: target_index,
                        identity_left: anchor.identity,
                        identity_right: target_record.identity,
                    });
                }
                PairOrientation::Unordered => {
                    // Snapshot materials are identity-sorted. Emit a pair on
                    // its lower-identity anchor pass when that anchor reaches
                    // the higher-identity target. If only the higher anchor's
                    // bounds reach the lower target, emit it on that reverse
                    // pass. When both queries reach, the lower anchor already
                    // emitted it, so the reverse pass is suppressed without
                    // retaining an O(candidate-pairs) seen set.
                    let (left, right) = if self.anchor_index < target_index {
                        (self.anchor_index, target_index)
                    } else {
                        (target_index, self.anchor_index)
                    };
                    let lower_anchor_reaches_higher = bounds[left]
                        .and_then(|value| value.expanded(padding))
                        .is_some_and(|value| {
                            bounds_include_cell(
                                field,
                                value,
                                snapshot.materials[right].locator.cell_index,
                            )
                        });
                    if self.anchor_index < target_index || !lower_anchor_reaches_higher {
                        page.pairs.push(MaterialPairRecord {
                            left,
                            right,
                            identity_left: snapshot.materials[left].identity,
                            identity_right: snapshot.materials[right].identity,
                        });
                    }
                }
            }
        }

        self.complete = self.anchors_visited >= snapshot.materials.len();
        page.deferred_anchors = snapshot
            .materials
            .len()
            .saturating_sub(self.anchors_visited) as u64;
        page.complete = self.complete;
        page
    }

    fn advance_cell(&mut self, range: BoundsCellRange) {
        self.cell_loaded = false;
        self.target_slot = 0;
        self.col += 1;
        if self.col > range.max_col {
            self.row_offset += 1;
            self.col = range.min_col;
        }
    }

    fn advance_anchor(&mut self, material_count: usize) {
        self.anchors_visited = self.anchors_visited.saturating_add(1);
        if self.anchors_visited < material_count {
            self.anchor_index = (self.anchor_index + 1) % material_count;
        }
        self.anchor_initialized = false;
        self.range = None;
        self.row_offset = 0;
        self.col = 0;
        self.target_slot = 0;
        self.cell_loaded = false;
        self.complete = self.anchors_visited >= material_count;
    }
}

/// O(1) equivalent of membership in `cells_intersecting_bounds`. It preserves
/// the current field semantics (X clamped, Y wrapped) without allocating or
/// traversing the full cell-query vector for each pair.
fn bounds_include_cell(
    field: &ActiveMaterialField,
    bounds: MaterialSpatialBounds,
    cell_index: usize,
) -> bool {
    if field.width_cells == 0
        || field.height_cells == 0
        || field.cell_size <= 0.0
        || cell_index >= field.cells.len()
    {
        return false;
    }
    let min_col = ((bounds.min_x / field.cell_size).floor() as isize).max(0) as usize;
    let max_col = ((bounds.max_x / field.cell_size).floor() as isize)
        .min(field.width_cells.saturating_sub(1) as isize)
        .max(0) as usize;
    if min_col > max_col {
        return false;
    }
    let col = cell_index % field.width_cells;
    if col < min_col || col > max_col {
        return false;
    }
    let row = cell_index / field.width_cells;
    let row_start = (bounds.min_y / field.cell_size).floor() as isize;
    let row_end = (bounds.max_y / field.cell_size).floor() as isize;
    let row_count = row_end.saturating_sub(row_start).saturating_add(1) as usize;
    if row_count >= field.height_cells {
        return true;
    }
    let normalized_start = row_start.rem_euclid(field.height_cells as isize) as usize;
    let offset = (row + field.height_cells - normalized_start) % field.height_cells;
    offset < row_count
}

impl EnvironmentalMaterialSnapshot {
    pub(crate) fn bond_reference_is_current(&self, reference: BondRef) -> bool {
        self.existing_bonds
            .iter()
            .any(|bond| bond.reference() == reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::Material;

    fn physical(identity: u64, name: &str) -> PhysicalMaterial {
        PhysicalMaterial {
            material: Material::free_base(name.to_owned(), 1.0),
            environmental_identity: identity,
            placements: None,
            internal_connections: None,
            owner_relative_origin: None,
            realized_shapes: None,
        }
    }

    #[test]
    fn snapshot_visits_field_once_and_retains_stable_identity_across_locators() {
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        field.cells[1]
            .physical_materials
            .push(physical(20, "Carbon"));
        field.cells[1]
            .physical_materials
            .push(physical(10, "Oxygen"));
        field.cells[3]
            .physical_materials
            .push(physical(30, "Hydrogen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 77);
        assert_eq!(snapshot.pass_id, 77);
        assert_eq!(snapshot.cell_visits, 8);
        assert_eq!(snapshot.material_visits, 3);
        assert_eq!(snapshot.materials.len(), 3);
        assert_eq!(snapshot.materials[0].identity, MaterialIdentity(10));
        assert_eq!(snapshot.materials[0].locator.cell_index, 1);
        assert_eq!(snapshot.materials[1].identity, MaterialIdentity(20));
        assert_eq!(snapshot.materials[2].identity, MaterialIdentity(30));
    }

    #[test]
    fn pair_windows_cover_every_unordered_pair_without_duplicates_or_gaps() {
        let mut field = ActiveMaterialField::new(50.0, 10.0, 10.0);
        for identity in 1..=5 {
            field.cells[0]
                .physical_materials
                .push(physical(identity, "Carbon"));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 1);
        assert_eq!(snapshot.known_pair_count(), 10);
        let (first, first_coverage) = snapshot.pair_window(0, 4);
        let (second, second_coverage) = snapshot.pair_window(4, 4);
        let (third, third_coverage) = snapshot.pair_window(8, 4);
        assert_eq!(first.len(), 4);
        assert_eq!(second.len(), 4);
        assert_eq!(third.len(), 2);
        assert_eq!(first_coverage.deferred_pairs, 6);
        assert_eq!(second_coverage.deferred_pairs, 2);
        assert_eq!(third_coverage.deferred_pairs, 0);
        let mut identities = first
            .into_iter()
            .chain(second)
            .chain(third)
            .map(|pair| (pair.identity_left, pair.identity_right))
            .collect::<Vec<_>>();
        identities.sort();
        identities.dedup();
        assert_eq!(identities.len(), 10);
        assert!(identities.iter().all(|(a, b)| a < b));
    }

    #[test]
    fn directed_reference_windows_cover_both_roles_without_self_pairs_or_gaps() {
        let mut field = ActiveMaterialField::new(50.0, 10.0, 10.0);
        for identity in 1..=5 {
            field.cells[0]
                .physical_materials
                .push(physical(identity, "Carbon"));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 2);
        assert_eq!(snapshot.known_directed_pair_count(), 20);

        let (first, first_coverage) = snapshot.directed_pair_window(0, 7);
        let (second, second_coverage) = snapshot.directed_pair_window(7, 7);
        let (third, third_coverage) = snapshot.directed_pair_window(14, 7);
        assert_eq!(first.len(), 7);
        assert_eq!(second.len(), 7);
        assert_eq!(third.len(), 6);
        assert_eq!(first_coverage.deferred_pairs, 13);
        assert_eq!(second_coverage.deferred_pairs, 6);
        assert_eq!(third_coverage.deferred_pairs, 0);

        let pairs = first
            .into_iter()
            .chain(second)
            .chain(third)
            .collect::<Vec<_>>();
        let mut identities = pairs
            .iter()
            .map(|pair| (pair.identity_left, pair.identity_right))
            .collect::<Vec<_>>();
        identities.sort();
        identities.dedup();
        assert_eq!(identities.len(), 20);
        assert!(identities
            .iter()
            .all(|(anchor, incoming)| anchor != incoming));
        for identity in 1..=5 {
            for other in 1..=5 {
                if identity == other {
                    continue;
                }
                assert!(identities.contains(&(MaterialIdentity(identity), MaterialIdentity(other))));
            }
        }
    }

    #[test]
    fn spatial_candidate_sets_match_directed_reference_filtered_by_field_queries() {
        let mut field = ActiveMaterialField::new(60.0, 40.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(40, "Carbon"));
        field.cells[0]
            .physical_materials
            .push(physical(10, "Oxygen"));
        field.cells[1]
            .physical_materials
            .push(physical(30, "Hydrogen"));
        field.cells[14]
            .physical_materials
            .push(physical(20, "Carbon"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 8);
        // Snapshot materials are sorted by environmental identity (10, 20, 30, 40),
        // so keep the fixture bounds in that same order rather than insertion order.
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 39.0,
                max_x: 51.0,
                min_y: 19.0,
                max_y: 31.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 9.0,
                max_x: 21.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: -1.0,
                max_x: 11.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
        ];
        let padding = 2.1;

        let mut expected_directed = std::collections::BTreeSet::new();
        let mut oracle_start = 0;
        loop {
            let (window, coverage) = snapshot.directed_pair_window(oracle_start, 3);
            for pair in window {
                let incoming = &snapshot.materials[pair.right];
                let Some(expanded) = bounds[pair.left].and_then(|value| value.expanded(padding))
                else {
                    continue;
                };
                let target_cells = field.cells_intersecting_bounds(
                    expanded.min_x,
                    expanded.max_x,
                    expanded.min_y,
                    expanded.max_y,
                );
                if target_cells.contains(&incoming.locator.cell_index) {
                    expected_directed.insert((pair.identity_left, pair.identity_right));
                }
            }
            oracle_start += coverage.visited_pairs;
            if oracle_start >= coverage.known_pairs {
                break;
            }
        }

        let (actual_directed, directed_coverage) = snapshot.spatial_pair_window(
            &field,
            &bounds,
            padding,
            0,
            usize::MAX,
            PairOrientation::Directed,
        );
        let actual_directed = actual_directed
            .into_iter()
            .map(|pair| (pair.identity_left, pair.identity_right))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual_directed, expected_directed);
        assert_eq!(
            directed_coverage.known_pairs,
            expected_directed.len() as u64
        );

        let expected_unordered = expected_directed
            .iter()
            .map(|(a, b)| if a < b { (*a, *b) } else { (*b, *a) })
            .collect::<std::collections::BTreeSet<_>>();
        let (actual_unordered, unordered_coverage) = snapshot.spatial_pair_window(
            &field,
            &bounds,
            padding,
            0,
            usize::MAX,
            PairOrientation::Unordered,
        );
        let actual_unordered = actual_unordered
            .into_iter()
            .map(|pair| {
                if pair.identity_left < pair.identity_right {
                    (pair.identity_left, pair.identity_right)
                } else {
                    (pair.identity_right, pair.identity_left)
                }
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual_unordered, expected_unordered);
        assert_eq!(
            unordered_coverage.known_pairs,
            expected_unordered.len() as u64
        );
    }

    #[test]
    fn directed_reference_empty_and_single_material_windows_are_empty() {
        let empty = EnvironmentalMaterialSnapshot::default();
        let (empty_pairs, empty_coverage) = empty.directed_pair_window(0, 10);
        assert!(empty_pairs.is_empty());
        assert_eq!(empty_coverage.known_pairs, 0);

        let mut field = ActiveMaterialField::new(10.0, 10.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(1, "Carbon"));
        let single = EnvironmentalMaterialSnapshot::from_field(&field, 3);
        let (single_pairs, single_coverage) = single.directed_pair_window(0, 10);
        assert!(single_pairs.is_empty());
        assert_eq!(single_coverage.known_pairs, 0);
    }

    #[test]
    fn spatial_pairs_prune_distant_materials_and_keep_displacement_directions() {
        let mut field = ActiveMaterialField::new(100.0, 100.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(10, "Carbon"));
        field.cells[11]
            .physical_materials
            .push(physical(20, "Oxygen"));
        field.cells[99]
            .physical_materials
            .push(physical(30, "Hydrogen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 3);
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: 9.0,
                max_x: 11.0,
                min_y: 9.0,
                max_y: 11.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 9.0,
                max_x: 11.0,
                min_y: 9.0,
                max_y: 11.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 89.0,
                max_x: 91.0,
                min_y: 89.0,
                max_y: 91.0,
            }),
        ];
        let (unordered, coverage) = snapshot.spatial_pair_window(
            &field,
            &bounds,
            0.0,
            0,
            usize::MAX,
            PairOrientation::Unordered,
        );
        assert_eq!(coverage.known_pairs, 1);
        assert_eq!(unordered.len(), 1);
        assert_eq!(
            (unordered[0].identity_left, unordered[0].identity_right),
            (MaterialIdentity(10), MaterialIdentity(20))
        );

        let (directed, directed_coverage) = snapshot.spatial_pair_window(
            &field,
            &bounds,
            0.0,
            0,
            usize::MAX,
            PairOrientation::Directed,
        );
        assert_eq!(directed_coverage.known_pairs, 2);
        assert_eq!(directed.len(), 2);
        assert!(directed.iter().any(|pair| {
            pair.identity_left == MaterialIdentity(10)
                && pair.identity_right == MaterialIdentity(20)
        }));
        assert!(directed.iter().any(|pair| {
            pair.identity_left == MaterialIdentity(20)
                && pair.identity_right == MaterialIdentity(10)
        }));
    }

    #[test]
    fn unordered_spatial_pair_uses_union_of_both_material_queries() {
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(10, "Carbon"));
        field.cells[1]
            .physical_materials
            .push(physical(20, "Oxygen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 6);
        // Only the higher-locator material's expanded bounds reach the
        // lower-locator material's cell. Unordered discovery must not lose it.
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: 0.0,
                max_x: 1.0,
                min_y: 0.0,
                max_y: 1.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 0.0,
                max_x: 11.0,
                min_y: 0.0,
                max_y: 1.0,
            }),
        ];
        let (pairs, coverage) = snapshot.spatial_pair_window(
            &field,
            &bounds,
            0.0,
            0,
            usize::MAX,
            PairOrientation::Unordered,
        );
        assert_eq!(coverage.known_pairs, 1);
        assert_eq!(pairs.len(), 1);
        assert_eq!(
            (pairs[0].identity_left, pairs[0].identity_right),
            (MaterialIdentity(10), MaterialIdentity(20))
        );
    }

    #[test]
    fn spatial_pairs_preserve_existing_vertical_wrap_and_horizontal_bounds() {
        let mut field = ActiveMaterialField::new(40.0, 40.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(10, "Carbon"));
        field.cells[12]
            .physical_materials
            .push(physical(20, "Oxygen"));
        field.cells[3]
            .physical_materials
            .push(physical(30, "Hydrogen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 5);
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: -1.0,
                max_x: 1.0,
                min_y: -1.0,
                max_y: 1.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_y: 29.0,
                max_y: 31.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 29.0,
                max_x: 31.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
        ];
        let (pairs, coverage) = snapshot.spatial_pair_window(
            &field,
            &bounds,
            0.0,
            0,
            usize::MAX,
            PairOrientation::Unordered,
        );
        assert_eq!(coverage.known_pairs, 1);
        assert_eq!(pairs.len(), 1);
        assert_eq!(
            (pairs[0].identity_left, pairs[0].identity_right),
            (MaterialIdentity(10), MaterialIdentity(20))
        );
    }

    #[test]
    fn spatial_pair_windows_report_deferred_work_without_losing_candidates() {
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        for identity in 1..=3 {
            field.cells[0]
                .physical_materials
                .push(physical(identity, "Carbon"));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 4);
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_y: 0.0,
                max_y: 10.0,
            });
            3
        ];
        let (_, first) =
            snapshot.spatial_pair_window(&field, &bounds, 0.0, 0, 1, PairOrientation::Unordered);
        let (_, second) =
            snapshot.spatial_pair_window(&field, &bounds, 0.0, 1, 8, PairOrientation::Unordered);
        assert_eq!(first.known_pairs, 3);
        assert_eq!(first.visited_pairs, 1);
        assert_eq!(first.deferred_pairs, 2);
        assert_eq!(second.visited_pairs, 2);
        assert_eq!(second.deferred_pairs, 0);
    }

    #[test]
    fn bounded_cursor_pages_match_spatial_reference_for_both_orientations() {
        let mut field = ActiveMaterialField::new(60.0, 40.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(40, "Carbon"));
        field.cells[0]
            .physical_materials
            .push(physical(10, "Oxygen"));
        field.cells[1]
            .physical_materials
            .push(physical(30, "Hydrogen"));
        field.cells[14]
            .physical_materials
            .push(physical(20, "Carbon"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 81);
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 39.0,
                max_x: 51.0,
                min_y: 19.0,
                max_y: 31.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 9.0,
                max_x: 21.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: -1.0,
                max_x: 11.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
        ];
        for orientation in [
            PairOrientation::Unordered,
            PairOrientation::Directed,
            PairOrientation::ForwardOrdered,
        ] {
            let (reference, _) =
                snapshot.spatial_pair_window(&field, &bounds, 2.1, 0, usize::MAX, orientation);
            let expected_pairs = reference.iter().map(|pair| match orientation {
                PairOrientation::Directed | PairOrientation::ForwardOrdered => {
                    (pair.identity_left, pair.identity_right)
                }
                PairOrientation::Unordered if pair.identity_left < pair.identity_right => {
                    (pair.identity_left, pair.identity_right)
                }
                PairOrientation::Unordered => (pair.identity_right, pair.identity_left),
            });
            let expected = expected_pairs.collect::<std::collections::BTreeSet<_>>();
            let mut cursor = SpatialPairCursor::new(&snapshot, orientation);
            let mut actual = std::collections::BTreeSet::new();
            let mut total_visited = 0;
            let mut pages = 0;
            loop {
                let page = cursor.next_page(&snapshot, &field, &bounds, 2.1, 3, 2);
                assert!(page.visited_pair_checks <= 3);
                assert!(page.pairs.len() <= 2);
                total_visited += page.visited_pair_checks;
                let page_pairs = page.pairs.into_iter().map(|pair| match orientation {
                    PairOrientation::Directed | PairOrientation::ForwardOrdered => {
                    (pair.identity_left, pair.identity_right)
                }
                    PairOrientation::Unordered if pair.identity_left < pair.identity_right => {
                        (pair.identity_left, pair.identity_right)
                    }
                    PairOrientation::Unordered => (pair.identity_right, pair.identity_left),
                });
                actual.extend(page_pairs);
                pages += 1;
                if page.complete {
                    break;
                }
                assert!(pages < 100);
            }
            let expected_checks = match orientation {
                PairOrientation::Unordered => snapshot.known_pair_count(),
                PairOrientation::Directed | PairOrientation::ForwardOrdered => {
                    snapshot.known_directed_pair_count()
                }
            };
            assert_eq!(total_visited, expected_checks);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn bounded_cursor_rejects_snapshot_refresh_and_empty_pages_keep_continuation() {
        let mut field = ActiveMaterialField::new(30.0, 10.0, 10.0);
        for identity in 1..=4 {
            field.cells[0]
                .physical_materials
                .push(physical(identity, "Carbon"));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 9);
        let bounds = vec![None; 4];
        let mut cursor = SpatialPairCursor::new(&snapshot, PairOrientation::Directed);
        let first = cursor.next_page(&snapshot, &field, &bounds, 0.0, 2, 8);
        assert!(first.pairs.is_empty());
        assert!(!first.complete);
        assert_eq!(first.visited_pair_checks, 2);

        let refreshed = EnvironmentalMaterialSnapshot::from_field(&field, 10);
        let rejected = cursor.next_page(&refreshed, &field, &bounds, 0.0, 8, 8);
        assert!(rejected.pairs.is_empty());
        assert!(!rejected.complete);
        assert_eq!(rejected.visited_pair_checks, 0);
    }

    #[test]
    fn indexed_cursor_matches_spatial_reference_with_bounded_work_and_both_orientations() {
        let mut field = ActiveMaterialField::new(100.0, 100.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(40, "Carbon"));
        field.cells[0]
            .physical_materials
            .push(physical(10, "Oxygen"));
        field.cells[1]
            .physical_materials
            .push(physical(30, "Hydrogen"));
        field.cells[14]
            .physical_materials
            .push(physical(20, "Carbon"));
        field.cells[99]
            .physical_materials
            .push(physical(50, "Nitrogen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 811);
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 39.0,
                max_x: 51.0,
                min_y: 19.0,
                max_y: 31.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 9.0,
                max_x: 21.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: -1.0,
                max_x: 11.0,
                min_y: 0.0,
                max_y: 10.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 89.0,
                max_x: 91.0,
                min_y: 89.0,
                max_y: 91.0,
            }),
        ];

        for orientation in [
            PairOrientation::Unordered,
            PairOrientation::Directed,
            PairOrientation::ForwardOrdered,
        ] {
            let (reference, _) =
                snapshot.spatial_pair_window(&field, &bounds, 2.1, 0, usize::MAX, orientation);
            let expected = reference
                .into_iter()
                .map(|pair| match orientation {
                    PairOrientation::Directed | PairOrientation::ForwardOrdered => {
                    (pair.identity_left, pair.identity_right)
                }
                    PairOrientation::Unordered if pair.identity_left < pair.identity_right => {
                        (pair.identity_left, pair.identity_right)
                    }
                    PairOrientation::Unordered => (pair.identity_right, pair.identity_left),
                })
                .collect::<std::collections::BTreeSet<_>>();

            let mut cursor = IndexedSpatialCursor::new(&snapshot, &field, orientation);
            let mut actual = Vec::new();
            let mut total_work = 0;
            let mut total_cells = 0;
            let mut total_materials = 0;
            let mut pages = 0;
            loop {
                let page = cursor.next_page(&snapshot, &field, &bounds, 2.1, 3, 2);
                assert!(page.visited_work <= 3);
                assert!(page.pairs.len() <= 2);
                assert_eq!(
                    page.visited_work,
                    page.visited_anchor_initializations
                        + page.visited_cell_checks
                        + page.visited_material_checks
                );
                total_work += page.visited_work;
                total_cells += page.visited_cell_checks;
                total_materials += page.visited_material_checks;
                actual.extend(
                    page.pairs
                        .into_iter()
                        .map(|pair| (pair.identity_left, pair.identity_right)),
                );
                pages += 1;
                if page.complete {
                    assert_eq!(page.deferred_anchors, 0);
                    break;
                }
                assert!(pages < 200);
            }
            actual.sort_unstable();
            assert_eq!(actual, expected.into_iter().collect::<Vec<_>>());
            assert!(total_cells > 0);
            assert!(total_materials > 0);
            assert!(total_work > 0);
        }
    }

    #[test]
    fn indexed_cursor_matches_spatial_reference_across_cell_edges_and_page_sizes() {
        let mut field = ActiveMaterialField::new(60.0, 40.0, 10.0);
        let placements = [
            (70, 0),
            (10, 0),
            (60, 5),
            (20, 6),
            (50, 11),
            (30, 17),
            (40, 23),
        ];
        for (identity, cell_index) in placements {
            field.cells[cell_index]
                .physical_materials
                .push(physical(identity, "Carbon"));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 815);
        let bounds = snapshot
            .materials
            .iter()
            .map(|record| {
                let (x, y) = field.cell_center(record.locator.cell_index);
                let radius = match record.identity.0 % 4 {
                    0 => 0.25,
                    1 => 5.0,
                    2 => 12.0,
                    _ => 25.0,
                };
                Some(MaterialSpatialBounds {
                    min_x: x - radius,
                    max_x: x + radius,
                    min_y: y - radius,
                    max_y: y + radius,
                })
            })
            .collect::<Vec<_>>();

        for orientation in [
            PairOrientation::Unordered,
            PairOrientation::Directed,
            PairOrientation::ForwardOrdered,
        ] {
            let (reference, _) =
                snapshot.spatial_pair_window(&field, &bounds, 2.1, 0, usize::MAX, orientation);
            let expected = reference
                .into_iter()
                .map(|pair| match orientation {
                    PairOrientation::Directed | PairOrientation::ForwardOrdered => {
                    (pair.identity_left, pair.identity_right)
                }
                    PairOrientation::Unordered if pair.identity_left < pair.identity_right => {
                        (pair.identity_left, pair.identity_right)
                    }
                    PairOrientation::Unordered => (pair.identity_right, pair.identity_left),
                })
                .collect::<std::collections::BTreeSet<_>>();

            for work_limit in 1..=5 {
                for output_limit in 1..=3 {
                    let mut cursor = IndexedSpatialCursor::new(&snapshot, &field, orientation);
                    let mut actual = Vec::new();
                    let mut pages = 0;
                    loop {
                        let page = cursor.next_page(
                            &snapshot,
                            &field,
                            &bounds,
                            2.1,
                            work_limit,
                            output_limit,
                        );
                        assert!(page.visited_work <= work_limit as u64);
                        assert!(page.pairs.len() <= output_limit);
                        for pair in page.pairs {
                            actual.push((pair.identity_left, pair.identity_right));
                        }
                        pages += 1;
                        if page.complete {
                            assert_eq!(page.deferred_anchors, 0);
                            break;
                        }
                        assert!(pages < 2_000, "cursor failed to make bounded progress");
                    }
                    actual.sort_unstable();
                    assert_eq!(
                        actual,
                        expected.iter().copied().collect::<Vec<_>>(),
                        "{orientation:?}/{work_limit}/{output_limit}"
                    );
                }
            }
        }
    }

    #[test]
    fn indexed_cursor_prunes_pair_work_for_sparse_materials() {
        let mut field = ActiveMaterialField::new(100.0, 100.0, 10.0);
        let mut bounds = Vec::new();
        for identity in 1..=30 {
            let cell_index = (identity as usize - 1) * 3;
            let (x, y) = field.cell_center(cell_index);
            field.cells[cell_index]
                .physical_materials
                .push(physical(identity, "Carbon"));
            bounds.push(Some(MaterialSpatialBounds {
                min_x: x - 0.25,
                max_x: x + 0.25,
                min_y: y - 0.25,
                max_y: y + 0.25,
            }));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 814);
        for orientation in [
            PairOrientation::Unordered,
            PairOrientation::Directed,
            PairOrientation::ForwardOrdered,
        ] {
            let exhaustive = match orientation {
                PairOrientation::Unordered => snapshot.known_pair_count(),
                PairOrientation::Directed => snapshot.known_directed_pair_count(),
                PairOrientation::ForwardOrdered => snapshot.known_pair_count(),
            };
            let mut cursor = IndexedSpatialCursor::new(&snapshot, &field, orientation);
            let mut visited_work = 0;
            let mut pages = 0;
            loop {
                let page = cursor.next_page(&snapshot, &field, &bounds, 0.0, 7, 4);
                assert!(page.visited_work <= 7);
                visited_work += page.visited_work;
                pages += 1;
                if page.complete {
                    break;
                }
                assert!(pages < 100);
            }
            assert!(visited_work < exhaustive);
        }
    }

    #[test]
    fn indexed_cursor_empty_pages_resume_and_reject_stale_field_or_snapshot() {
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        for identity in 1..=3 {
            field.cells[0]
                .physical_materials
                .push(physical(identity, "Carbon"));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 812);
        let bounds = vec![None; 3];
        let mut cursor = IndexedSpatialCursor::new(&snapshot, &field, PairOrientation::Directed);
        let first = cursor.next_page(&snapshot, &field, &bounds, 0.0, 1, 8);
        assert!(first.pairs.is_empty());
        assert!(!first.complete);
        assert_eq!(first.visited_work, 1);

        let refreshed = EnvironmentalMaterialSnapshot::from_field(&field, 813);
        let stale = cursor.next_page(&refreshed, &field, &bounds, 0.0, 8, 8);
        assert!(stale.pairs.is_empty());
        assert!(!stale.complete);
        assert_eq!(stale.visited_work, 0);

        let mut changed_field = field.clone();
        changed_field.mark_changed_at_index(0);
        let stale_field = cursor.next_page(&snapshot, &changed_field, &bounds, 0.0, 8, 8);
        assert!(stale_field.pairs.is_empty());
        assert!(!stale_field.complete);
        assert_eq!(stale_field.visited_work, 0);
    }

    #[test]
    fn bounds_cell_range_matches_existing_field_query_membership() {
        let field = ActiveMaterialField::new(40.0, 40.0, 10.0);
        let bounds_cases = [
            MaterialSpatialBounds {
                min_x: -1.0,
                max_x: 11.0,
                min_y: -1.0,
                max_y: 11.0,
            },
            MaterialSpatialBounds {
                min_x: 9.0,
                max_x: 21.0,
                min_y: 29.0,
                max_y: 51.0,
            },
            MaterialSpatialBounds {
                min_x: 0.0,
                max_x: 100.0,
                min_y: -100.0,
                max_y: 100.0,
            },
            MaterialSpatialBounds {
                min_x: 40.0,
                max_x: 60.0,
                min_y: 0.0,
                max_y: 10.0,
            },
        ];
        for bounds in bounds_cases {
            let expected = field.cells_intersecting_bounds(
                bounds.min_x,
                bounds.max_x,
                bounds.min_y,
                bounds.max_y,
            );
            let range = BoundsCellRange::new(&field, bounds);
            let actual = range.map_or_else(Vec::new, |range| {
                let mut cells = Vec::new();
                for row_offset in 0..range.row_count {
                    for col in range.min_col..=range.max_col {
                        cells.push(range.cell_index(&field, row_offset, col));
                    }
                }
                cells
            });
            assert_eq!(actual, expected);
        }
    }

    fn realized_physical(
        identity: u64,
        material: Material,
        placements: Vec<crate::structure::Placement>,
        catalog: &[crate::resources::BaseResource],
    ) -> PhysicalMaterial {
        let mut physical = PhysicalMaterial::realized(material, placements, catalog)
            .expect("test material should have a valid physical realization");
        physical.environmental_identity = identity;
        physical
    }

    #[test]
    fn material_bounds_cursor_matches_restore_oracle_across_single_work_pages() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        field.cells[0].physical_materials.push(realized_physical(
            20,
            Material {
                parts: vec![("Carbon".to_owned(), 1.0), ("Oxygen".to_owned(), 1.0)],
                internal_bonds: Vec::new(),
            },
            vec![
                crate::structure::Placement {
                    x: 1.0,
                    y: 2.0,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 5.0,
                    y: -1.0,
                    rotation_radians: 0.4,
                },
            ],
            &catalog,
        ));
        field.cells[1].physical_materials.push(realized_physical(
            10,
            Material::free_base("Hydrogen", 1.0),
            vec![crate::structure::Placement {
                x: 14.0,
                y: 3.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        ));

        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 101);
        let expected = snapshot.spatial_bounds_from_field(&field, &catalog);
        let expected_work = snapshot.materials.len() as u64
            + snapshot
                .materials
                .iter()
                .map(|material| material.constituent_count as u64)
                .sum::<u64>();
        let mut cursor = MaterialBoundsCursor::new(&snapshot, &catalog);
        let mut visited = 0;
        let mut pages = 0;
        let actual = loop {
            let page = cursor.next_page(&snapshot, &field, &catalog, 1);
            assert!(page.visited_work <= 1);
            visited += page.visited_work;
            pages += 1;
            if page.complete {
                assert_eq!(page.deferred_work, 0);
                break page.bounds;
            }
            assert!(page.bounds.is_none());
            assert!(pages < 20);
        };
        assert_eq!(visited, expected_work);
        assert_eq!(
            actual.expect("completed cursor returns all bounds"),
            expected
        );
        let completed_again = cursor.next_page(&snapshot, &field, &catalog, 1);
        assert!(completed_again.complete);
        assert!(completed_again.bounds.is_none());
    }

    #[test]
    fn material_bounds_cursor_rejects_stale_snapshot_and_duplicate_catalog_names() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(20.0, 10.0, 10.0);
        field.cells[0].physical_materials.push(realized_physical(
            1,
            Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 1.0,
                y: 1.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        ));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 102);
        let mut cursor = MaterialBoundsCursor::new(&snapshot, &catalog);
        let refreshed = EnvironmentalMaterialSnapshot::from_field(&field, 103);
        let stale = cursor.next_page(&refreshed, &field, &catalog, 10);
        assert!(!stale.complete);
        assert_eq!(stale.visited_work, 0);

        let mut changed_field = field.clone();
        changed_field.mark_changed_at_index(0);
        let mut stale_field_cursor = MaterialBoundsCursor::new(&snapshot, &catalog);
        let stale_field = stale_field_cursor.next_page(&snapshot, &changed_field, &catalog, 10);
        assert!(!stale_field.complete);
        assert_eq!(stale_field.visited_work, 0);

        let mut duplicate_catalog = catalog.clone();
        duplicate_catalog.push(duplicate_catalog[0].clone());
        let mut invalid_cursor = MaterialBoundsCursor::new(&snapshot, &duplicate_catalog);
        let invalid = loop {
            let page = invalid_cursor.next_page(&snapshot, &field, &duplicate_catalog, 1);
            if page.complete {
                break page;
            }
        };
        assert_eq!(invalid.bounds, Some(vec![None]));
    }

    #[test]
    fn unassigned_materials_are_reported_not_given_fabricated_identity() {
        let mut field = ActiveMaterialField::new(10.0, 10.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(0, "Carbon"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 2);
        assert_eq!(snapshot.unassigned_materials, 1);
        assert!(snapshot.materials.is_empty());
        assert_eq!(snapshot.known_pair_count(), 0);
    }
}
