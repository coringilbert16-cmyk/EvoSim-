// Thin crate entrypoint used to compile the experimental COMBINE modules
// alongside the existing simulation without duplicating main.rs.
mod combine;
mod structural_combine;

include!("main.rs");
