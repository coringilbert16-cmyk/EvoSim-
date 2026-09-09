// Settling returns active-field aggregate material to the matching reservoir.
use super::field::{ActiveMaterialField,MATERIAL_EPSILON};
use crate::material_transfer::take_whole_unstructured;
use super::reservoir::DeepReservoir;
pub const DEFAULT_SETTLING_FRACTION:f64=0.01;
pub const DEFAULT_SETTLING_INTERVAL_TICKS:u64=10;
pub fn apply_settling(field:&mut ActiveMaterialField,reservoir:&mut DeepReservoir,fraction:f64){let fraction=fraction.clamp(0.0,1.0);if fraction<=0.0{return}for field_index in 0..field.cells.len(){let reservoir_index=reservoir.reservoir_index_for_field_index(field,field_index);let mut retained=Vec::new();let materials=std::mem::take(&mut field.cells[field_index].materials);for mut material in materials{if material.is_structured(){retained.push(material);continue}let total=material.total_amount();if total<=MATERIAL_EPSILON{continue}let outflow=(total*fraction).floor()as usize;if outflow==0{retained.push(material);continue}if let Some(taken)=take_whole_unstructured(&mut material,outflow){for component in taken.composition(){reservoir.cells[reservoir_index].add(&component.resource,component.amount)}}if !material.is_empty(){retained.push(material)}}field.cells[field_index].materials=retained;}}
