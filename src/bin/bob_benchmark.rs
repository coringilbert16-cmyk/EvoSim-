use evosim::geometry_reference_library::{
    GeometryLibrary, LiveFamilyResolution, LiveGeometryInterface, LiveGeometryQuery,
};
use evosim::resources::default_catalog;
use std::env;
use std::hint::black_box;
use std::time::Instant;

const DEFAULT_ITERATIONS: u64 = 200_000;
const WARMUP_ITERATIONS: u64 = 10_000;

fn bench<F>(name: &str, iterations: u64, mut query: F)
where
    F: FnMut() -> bool,
{
    for _ in 0..WARMUP_ITERATIONS {
        black_box(query());
    }

    let start = Instant::now();
    let mut hits = 0u64;
    for _ in 0..iterations {
        if black_box(query()) {
            hits += 1;
        }
    }
    let elapsed = start.elapsed();
    let ns = elapsed.as_secs_f64() * 1e9 / iterations as f64;
    let qps = iterations as f64 / elapsed.as_secs_f64();

    println!(
        "{name}: {iterations} queries in {:?} | {:.1} ns/query | {:.0} queries/sec | {} positive resolutions",
        elapsed, ns, qps, hits
    );
}

fn edge_query(library: &GeometryLibrary) -> Option<LiveGeometryInterface> {
    let family = library.rigid_contact_families().next()?;
    let formation = library.get(&family.formation_signature)?;
    let anchor = formation.constituents.get(family.anchor_constituent)?;
    let anchor_parameter =
        (family.anchor_parameter_start + family.anchor_parameter_end) * 0.5;

    Some(LiveGeometryInterface {
        interface_class: "rigid_edge",
        signature: String::new(),
        query: Some(LiveGeometryQuery::RigidEdge {
            a_material: family.candidate_resource.clone(),
            a_edge: family.candidate_edge,
            a_parameter: 0,
            a_rotation: (family.candidate_rotation_radians * 1_000_000_000.0) as i64,
            b_material: anchor.resource.clone(),
            b_edge: family.anchor_edge,
            b_parameter: (anchor_parameter * 1_000_000_000.0) as i64,
            b_rotation: 0,
        }),
    })
}

fn point_query(library: &GeometryLibrary) -> Option<LiveGeometryInterface> {
    let family = library.rigid_point_contact_families().next()?;
    let formation = library.get(&family.formation_signature)?;
    let anchor = formation.constituents.get(family.anchor_constituent)?;
    let anchor_parameter =
        (family.anchor_parameter_start + family.anchor_parameter_end) * 0.5;

    Some(LiveGeometryInterface {
        interface_class: "rigid_point",
        signature: String::new(),
        query: Some(LiveGeometryQuery::RigidPoint {
            line_material: family.candidate_resource.clone(),
            line_point: family.candidate_endpoint,
            edge_material: anchor.resource.clone(),
            edge: family.anchor_edge,
            edge_parameter: (anchor_parameter * 1_000_000_000.0) as i64,
        }),
    })
}

fn vertex_query(library: &GeometryLibrary) -> Option<LiveGeometryInterface> {
    let family = library.rigid_vertex_contact_families().next()?;
    let formation = library.get(&family.formation_signature)?;
    let anchor = formation.constituents.get(family.anchor_constituent)?;
    let anchor_parameter =
        (family.anchor_parameter_start + family.anchor_parameter_end) * 0.5;

    Some(LiveGeometryInterface {
        interface_class: "rigid_vertex",
        signature: String::new(),
        query: Some(LiveGeometryQuery::RigidVertex {
            corner_material: family.candidate_resource.clone(),
            corner_point: family.candidate_vertex,
            edge_material: anchor.resource.clone(),
            edge: family.anchor_edge,
            edge_parameter: (anchor_parameter * 1_000_000_000.0) as i64,
        }),
    })
}

fn run_query_benchmark(
    name: &str,
    library: &GeometryLibrary,
    query: Option<LiveGeometryInterface>,
    iterations: u64,
) {
    match query {
        Some(query) => bench(name, iterations, || {
            library.resolve_persistent_interface(&query)
                != LiveFamilyResolution::Unresolved
        }),
        None => println!("{name}: no persisted family available"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let root = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("geometry_library/data");
    let iterations = args
        .get(2)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_ITERATIONS);

    let catalog = default_catalog();
    let load_start = Instant::now();
    let library = GeometryLibrary::open(root, &catalog)?;
    let load_elapsed = load_start.elapsed();

    println!("Bob benchmark");
    println!("root: {root}");
    println!("load time: {:?}", load_elapsed);
    println!("formations: {}", library.len());
    println!("rigid edge families: {}", library.rigid_contact_families().count());
    println!("rigid point families: {}", library.rigid_point_contact_families().count());
    println!("rigid vertex families: {}", library.rigid_vertex_contact_families().count());
    println!("iterations: {iterations}");

    run_query_benchmark("rigid_edge", &library, edge_query(&library), iterations);
    run_query_benchmark("rigid_point", &library, point_query(&library), iterations);
    run_query_benchmark("rigid_vertex", &library, vertex_query(&library), iterations);

    Ok(())
}
