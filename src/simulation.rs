                            if let Some(mut transformation) = Self::try_start_transformation(
                                &mut organisms[index],
                                &environment.catalog,
                                &mut self.next_transformation_id,
                                &selected,
                            ) {
                                transformation.pending_experience =
                                    Some(crate::memory::PendingTransformationExperience {
                                        perceptions,
                                        needs,
                                        before_energy,
                                        before_stress,
                                        before_developmental_realization,
                                        material_transformed: transformation
                                            .stored_material
                                            .as_ref()
                                            .map(|instance| instance.material.total_amount())
                                            .unwrap_or(0.0),
                                    });