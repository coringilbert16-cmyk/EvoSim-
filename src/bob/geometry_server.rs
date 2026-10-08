use crate::geometry_reference_library::{open_default_library, GeometryFormation};
use crate::resources::default_catalog;
use axum::{
    extract::Query,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use tower_http::cors::CorsLayer;

#[derive(Deserialize, Default)]
struct FormationQuery {
    signature: Option<String>,
}
#[derive(Deserialize, Default)]
struct ListQuery {
    offset: Option<usize>,
    limit: Option<usize>,
    search: Option<String>,
    size: Option<String>,
}
fn matches(f: &GeometryFormation, s: Option<&str>, z: Option<&str>) -> bool {
    if let Some(z) = z {
        let n = f.constituents.len();
        if !match z {
            "1" => n == 1,
            "2" => n == 2,
            "3" => n == 3,
            "4" => n == 4,
            "5" => n >= 5,
            _ => true,
        } {
            return false;
        }
    }
    if let Some(s) = s {
        let q = s.to_lowercase();
        return f.signature.to_lowercase().contains(&q)
            || f.constituents
                .iter()
                .any(|c| c.resource.to_lowercase().contains(&q));
    }
    true
}
async fn index() -> impl IntoResponse {
    (
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Html(include_str!("../ui/geometry_library.html")),
    )
}
async fn script() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        include_str!("../ui/geometry_library.js"),
    )
}
async fn formations(Query(q): Query<ListQuery>) -> impl IntoResponse {
    let cat = default_catalog();
    let Ok(formations) = crate::geometry_reference_library::GeometryLibrary::load_formations_only(
        "geometry_library/data",
        &cat,
    ) else {
        return Json(json!({"error":"library_unavailable"}));
    };
    let library_entries = formations.len();
    let mut all: Vec<GeometryFormation> = formations
        .into_iter()
        .filter(|f| matches(f, q.search.as_deref(), q.size.as_deref()))
        .collect();
    all.sort_by(|a, b| {
        a.constituent_count()
            .cmp(&b.constituent_count())
            .then_with(|| a.signature.cmp(&b.signature))
    });
    let total = all.len();
    let off = q.offset.unwrap_or(0).min(total);
    let lim = q.limit.unwrap_or(250).clamp(1, 1000);
    let rows = all
        .into_iter()
        .skip(off)
        .take(lim)
        .map(|f| {
            json!({
                "signature": f.signature,
                "constituent_count": f.constituents.len(),
                "bond_count": f.bonds.len(),
                "resources": f
                    .constituents
                    .iter()
                    .map(|c| c.resource.clone())
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    Json(
        json!({"total":total,"library_entries":library_entries,"formations":rows,"resource_count":cat.len()}),
    )
}
async fn formation(Query(q): Query<FormationQuery>) -> impl IntoResponse {
    let Some(sig) = q.signature else {
        return Json(json!({"error":"missing_signature"}));
    };
    let cat = default_catalog();
    let Ok(formations) = crate::geometry_reference_library::GeometryLibrary::load_formations_only(
        "geometry_library/data",
        &cat,
    ) else {
        return Json(json!({"error":"library_unavailable"}));
    };
    let Some(f) = formations.into_iter().find(|f| f.signature == sig) else {
        return Json(json!({"error":"not_found"}));
    };
    let mut constituents = Vec::with_capacity(f.constituents.len());
    for c in &f.constituents {
        let Some(resource) = cat.iter().find(|r| r.name == c.resource) else {
            return Json(
                json!({"error":"catalog_resource_missing","resource":c.resource,"signature":f.signature}),
            );
        };
        constituents.push(json!({
            "resource":c.resource,
            "placement":c.placement,
            "physical_state":resource.physical_state,
            "form":resource.shape.form.clone()
        }));
    }
    Json(json!({
        "formation":{
            "schema_version":f.schema_version,
            "signature":f.signature,
            "constituents":constituents,
            "bonds":f.bonds
        },
        "contact_families":[],
        "fluid_boundary_families":[],
        "rigid_contact_families":[],
        "rigid_point_contact_families":[],
        "rigid_vertex_contact_families":[]
    }))
}
pub async fn run() {
    let app = Router::new()
        .route("/", get(index))
        .route("/geometry", get(index))
        .route("/geometry_library.js", get(script))
        .route("/geometry/api/formations", get(formations))
        .route("/geometry/api/formation", get(formation))
        .layer(CorsLayer::permissive());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("Geometry library viewer: http://{addr}/geometry");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("could not bind geometry viewer");
    axum::serve(listener, app)
        .await
        .expect("geometry viewer stopped");
}