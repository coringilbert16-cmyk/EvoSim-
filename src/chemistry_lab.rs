//! Dedicated experimental workbench backed by the shared EvoSim engine.

use crate::{
    environment::ActiveMaterialField,
    physical_material::PhysicalMaterial,
    resources::{InternalBond, Material},
    simulation::Simulation,
    state::{EnergyLedger, Environment},
    structure::Placement,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaboratoryConfig {
    pub width: f64,
    pub height: f64,
    pub cell_size: f64,
    pub ticks_per_second: f64,
    pub seed: u64,
}

impl Default for LaboratoryConfig {
    fn default() -> Self {
        Self {
            width: 100.0,
            height: 100.0,
            cell_size: 5.0,
            ticks_per_second: 20.0,
            seed: 42,
        }
    }
}

impl LaboratoryConfig {
    fn validate(&self) -> Result<(), &'static str> {
        if !self.width.is_finite() || self.width <= 0.0 || self.width > 10_000.0 {
            return Err("width must be in (0, 10000]");
        }
        if !self.height.is_finite() || self.height <= 0.0 || self.height > 10_000.0 {
            return Err("height must be in (0, 10000]");
        }
        if !self.cell_size.is_finite() || self.cell_size < 1.0 || self.cell_size > 500.0 {
            return Err("cell_size must be in [1, 500]");
        }
        if !self.ticks_per_second.is_finite()
            || self.ticks_per_second <= 0.0
            || self.ticks_per_second > 10_000.0
        {
            return Err("ticks_per_second must be in (0, 10000]");
        }
        Ok(())
    }
}

pub struct Laboratory {
    pub config: LaboratoryConfig,
    simulation: Simulation,
}

impl Laboratory {
    pub fn new(config: LaboratoryConfig) -> Result<Self, &'static str> {
        config.validate()?;
        let environment = Environment {
            width: config.width,
            height: config.height,
            field: ActiveMaterialField::new(config.width, config.height, config.cell_size),
            catalog: crate::resources::default_catalog(),
            chemistry_usable_energy: 0.0,
            resonant_energy: Default::default(),
        };
        let simulation = Simulation {
            tick: 0,
            ticks_per_second: config.ticks_per_second,
            running: false,
            organisms: Vec::new(),
            environment,
            active_transformations: Vec::new(),
            chemical_reaction_accumulation: Default::default(),
            chemistry_library: crate::chemistry_library::open_default_library()
                .expect("chemistry library must open"),
            decomposing_bodies: Vec::new(),
            energy_ledger: EnergyLedger::default(),
            next_organism_id: 1,
            next_transformation_id: 1,
            rng: ChaCha8Rng::seed_from_u64(config.seed),
            decision_parameters: crate::decision::DecisionParameters::default(),
            seed_scale_reference: None,
            world_tone: None,
            world_tone_cycle: None,
            hotspot: None,
            environmental_chemistry_diagnostics: Default::default(),
            environmental_chemistry_events: Vec::new(),
            environmental_local_resonance_samples: Vec::new(),
            record_local_resonance_diagnostics: false,
            record_environmental_molecular_history: true,
        };
        Ok(Self { config, simulation })
    }

    pub fn place_element(
        &mut self,
        resource: &str,
        x: f64,
        y: f64,
    ) -> Result<u64, &'static str> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= self.config.width
            || y >= self.config.height
        {
            return Err("placement must be inside laboratory bounds");
        }
        self.place_material(
            Material::free_base(resource.to_string(), 1.0),
            vec![Placement {
                x,
                y,
                rotation_radians: 0.0,
            }],
            x,
            y,
        )
    }

    pub fn place_molecule(
        &mut self,
        parts: Vec<String>,
        bonds: Vec<[usize; 2]>,
        placements: Vec<[f64; 3]>,
    ) -> Result<u64, &'static str> {
        if parts.is_empty() || parts.len() > 64 || placements.len() != parts.len() {
            return Err("molecule needs 1–64 parts and one placement per part");
        }
        if parts.iter().any(|name| {
            !self
                .simulation
                .environment
                .catalog
                .iter()
                .any(|resource| resource.name == *name)
        }) {
            return Err("molecule contains an unknown resource type");
        }
        if placements.iter().any(|placement| {
            !placement[0].is_finite()
                || !placement[1].is_finite()
                || !placement[2].is_finite()
                || placement[0] < 0.0
                || placement[1] < 0.0
                || placement[0] >= self.config.width
                || placement[1] >= self.config.height
        }) {
            return Err("all constituent placements must be inside laboratory bounds");
        }

        let mut internal_bonds = Vec::with_capacity(bonds.len());
        for [part_a, part_b] in bonds {
            let bond = InternalBond { part_a, part_b };
            if !bond.is_valid_for(parts.len())
                || internal_bonds.contains(&bond)
                || internal_bonds
                    .iter()
                    .any(|prior: &InternalBond| prior.part_a == part_b && prior.part_b == part_a)
            {
                return Err("molecule contains an invalid or duplicate bond");
            }
            internal_bonds.push(bond);
        }

        let material = Material {
            parts: parts.into_iter().map(|name| (name, 1.0)).collect(),
            internal_bonds,
        };
        let placements = placements
            .into_iter()
            .map(|placement| Placement {
                x: placement[0],
                y: placement[1],
                rotation_radians: placement[2],
            })
            .collect::<Vec<_>>();
        let anchor = placements.first().ok_or("molecule has no placement")?;
        self.place_material(material, placements, anchor.x, anchor.y)
    }

    fn place_material(
        &mut self,
        material: Material,
        placements: Vec<Placement>,
        x: f64,
        y: f64,
    ) -> Result<u64, &'static str> {
        let catalog = &self.simulation.environment.catalog;
        let physical = PhysicalMaterial::realized(material, placements, catalog)
            .ok_or("physical realization or bond geometry validation failed")?;
        let field = &mut self.simulation.environment.field;
        if !field.deposit_physical(x, y, physical, catalog) {
            return Err("environment rejected material");
        }
        field.assign_missing_material_identities();
        let index = field.index_for_position(x, y).ok_or("invalid destination")?;
        field.cells[index]
            .physical_materials
            .last()
            .map(|material| material.environmental_identity)
            .ok_or("identity assignment failed")
    }

    pub fn step(&mut self, count: u32) -> Result<(), &'static str> {
        if count == 0 || count > 100_000 {
            return Err("count must be in [1, 100000]");
        }
        for _ in 0..count {
            self.simulation.step();
        }
        Ok(())
    }

    pub fn snapshot(&self) -> LaboratorySnapshot {
        let materials = self
            .simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(|material| MaterialSnapshot {
                identity: material.environmental_identity,
                parts: material.material.parts.clone(),
                placements: material
                    .placements
                    .clone()
                    .unwrap_or_default()
                    .iter()
                    .map(|placement| {
                        [
                            placement.x,
                            placement.y,
                            placement.rotation_radians,
                        ]
                    })
                    .collect(),
                bond_count: material
                    .internal_connections
                    .as_ref()
                    .map_or(0, Vec::len),
                bonds: material
                    .material
                    .internal_bonds
                    .iter()
                    .map(|bond| [bond.part_a, bond.part_b])
                    .collect(),
            })
            .collect();
        LaboratorySnapshot {
            tick: self.simulation.tick,
            width: self.config.width,
            height: self.config.height,
            material_count: self
                .simulation
                .environment
                .field
                .cells
                .iter()
                .map(|cell| cell.physical_materials.len())
                .sum(),
            materials,
            chemistry_usable_energy: self.simulation.environment.chemistry_usable_energy,
            resonant_energy: self.simulation.environment.resonant_energy.total_energy(),
            organisms: self.simulation.organisms.len(),
        }
    }
}

#[derive(Serialize)]
pub struct LaboratorySnapshot {
    pub tick: u64,
    pub width: f64,
    pub height: f64,
    pub material_count: usize,
    pub materials: Vec<MaterialSnapshot>,
    pub chemistry_usable_energy: f64,
    pub resonant_energy: Option<f64>,
    pub organisms: usize,
}

#[derive(Serialize)]
pub struct MaterialSnapshot {
    pub identity: u64,
    pub parts: Vec<(String, f64)>,
    pub placements: Vec<[f64; 3]>,
    pub bond_count: usize,
    pub bonds: Vec<[usize; 2]>,
}

#[derive(Clone)]
struct LabState(Arc<Mutex<Laboratory>>);

#[derive(Deserialize)]
struct PlaceRequest {
    resource: String,
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
struct MoleculeRequest {
    parts: Vec<String>,
    bonds: Vec<[usize; 2]>,
    placements: Vec<[f64; 3]>,
}

#[derive(Deserialize)]
struct StepRequest {
    count: Option<u32>,
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn state(State(state): State<LabState>) -> Json<LaboratorySnapshot> {
    Json(state.0.lock().await.snapshot())
}

async fn reset(
    State(state): State<LabState>,
    Json(config): Json<LaboratoryConfig>,
) -> Result<Json<LaboratorySnapshot>, (StatusCode, String)> {
    let replacement =
        Laboratory::new(config).map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    let snapshot = replacement.snapshot();
    *state.0.lock().await = replacement;
    Ok(Json(snapshot))
}

async fn place(
    State(state): State<LabState>,
    Json(request): Json<PlaceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let identity = state
        .0
        .lock()
        .await
        .place_element(&request.resource, request.x, request.y)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(serde_json::json!({ "identity": identity })))
}

async fn molecule(
    State(state): State<LabState>,
    Json(request): Json<MoleculeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let identity = state
        .0
        .lock()
        .await
        .place_molecule(request.parts, request.bonds, request.placements)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(serde_json::json!({ "identity": identity })))
}

async fn step(
    State(state): State<LabState>,
    Json(request): Json<StepRequest>,
) -> Result<Json<LaboratorySnapshot>, (StatusCode, String)> {
    let mut laboratory = state.0.lock().await;
    laboratory
        .step(request.count.unwrap_or(1))
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(laboratory.snapshot()))
}

pub async fn run() {
    let laboratory =
        Laboratory::new(LaboratoryConfig::default()).expect("default lab config valid");
    let state = LabState(Arc::new(Mutex::new(laboratory)));
    let app = Router::new()
        .route("/", get(index))
        .route("/api/state", get(state))
        .route("/api/reset", post(reset))
        .route("/api/place", post(place))
        .route("/api/molecule", post(molecule))
        .route("/api/step", post(step))
        .with_state(state);
    let address = std::env::var("EVOSIM_LAB_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:3030".into())
        .parse::<SocketAddr>()
        .expect("valid EVOSIM_LAB_ADDRESS");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("lab listener binds");
    println!("EvoSim Chemical Laboratory: http://{address}");
    axum::serve(listener, app)
        .await
        .expect("lab server failed");
}

const INDEX_HTML: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>EvoSim Chemical Laboratory</title><style>
:root{color-scheme:dark;font:15px system-ui;background:#081b2a;color:#e7f3fa}body{margin:0;display:grid;grid-template-rows:auto 1fr;min-height:100vh}header{padding:12px 18px;background:#102b40;border-bottom:1px solid #28506a;display:flex;gap:18px;align-items:center;flex-wrap:wrap}main{display:grid;grid-template-columns:minmax(0,1fr) 300px;min-height:0}canvas{width:100%;height:100%;background:#0b3147;display:block;touch-action:manipulation}aside{padding:16px;background:#102638;border-left:1px solid #28506a}button,select,input,textarea{background:#173c54;color:#e7f3fa;border:1px solid #46718a;border-radius:5px;padding:8px}#status{color:#9ec8df}#materials{font-size:13px;line-height:1.7;white-space:pre-wrap;overflow-wrap:anywhere}textarea{width:95%}@media(max-width:650px){main{grid-template-columns:1fr;grid-template-rows:minmax(55vh,1fr) auto}aside{border-left:0;border-top:1px solid #28506a}}
</style></head><body><header><strong>EvoSim · Chemical Laboratory</strong><button id="step">Step ×1</button><button id="step100">Step ×100</button><label>Width <input id="width" type="number" value="100" min="1" max="10000" style="width:65px"></label><label>Height <input id="height" type="number" value="100" min="1" max="10000" style="width:65px"></label><label>Seed <input id="seed" type="number" value="42" style="width:75px"></label><button id="reset">Reset world</button><label>Element <select id="element"></select></label><span id="status">Loading…</span></header><main><canvas id="world"></canvas><aside><h3>Experiment state</h3><div id="summary"></div><h3>Physical materials</h3><div id="materials"></div><h3>Place molecule (JSON)</h3><textarea id="molecule" rows="5">{"parts":["Carbon","Oxygen"],"bonds":[[0,1]],"placements":[[45,50,0],[47,50,0]]}</textarea><button id="placeMolecule">Place molecule</button><p>Tap the field to place the selected element. Molecules use the exact constituent coordinates in the editor above.</p></aside></main><script>
const canvas=document.querySelector('#world'),ctx=canvas.getContext('2d');let state=null;const palette={Carbon:'#9aa5ae',Oxygen:'#e04444',Hydrogen:'#dce8ee',Water:'#328ed0',Nitrogen:'#9b7045',Phosphorus:'#e3a34c',Sulfur:'#e2d34b'};
function resize(){const r=canvas.getBoundingClientRect(),d=devicePixelRatio||1;canvas.width=r.width*d;canvas.height=r.height*d;draw()}
function draw(){if(!state)return;const sx=canvas.width/state.width,sy=canvas.height/state.height;ctx.clearRect(0,0,canvas.width,canvas.height);ctx.strokeStyle='#17465d';ctx.lineWidth=1;for(let x=0;x<=state.width;x+=10){ctx.beginPath();ctx.moveTo(x*sx,0);ctx.lineTo(x*sx,canvas.height);ctx.stroke()}for(let y=0;y<=state.height;y+=10){ctx.beginPath();ctx.moveTo(0,y*sy);ctx.lineTo(canvas.width,y*sy);ctx.stroke()}for(const m of state.materials){ctx.strokeStyle='#a7d8ed';ctx.lineWidth=Math.max(1,Math.min(sx,sy)*.12);for(const bond of m.bonds){const a=m.placements[bond[0]],b=m.placements[bond[1]];if(!a||!b)continue;ctx.beginPath();ctx.moveTo(a[0]*sx,a[1]*sy);ctx.lineTo(b[0]*sx,b[1]*sy);ctx.stroke()}for(let i=0;i<m.parts.length;i++){const p=m.placements[i];if(!p)continue;ctx.beginPath();ctx.arc(p[0]*sx,p[1]*sy,Math.max(3,Math.min(sx,sy)*.65),0,Math.PI*2);ctx.fillStyle=palette[m.parts[i][0]]||'#ddd';ctx.fill();ctx.strokeStyle='#071521';ctx.stroke()}}}
async function refresh(){state=await(await fetch('/api/state')).json();document.querySelector('#summary').textContent='Tick: '+state.tick+' · Materials: '+state.material_count+' · Organisms: '+state.organisms;document.querySelector('#materials').textContent=state.materials.map(m=>'#'+m.identity+' '+m.parts.map(p=>p[0]).join('–')+' · bonds '+m.bond_count).join('\n')||'No materials placed';document.querySelector('#status').textContent='Ready';draw()}
canvas.addEventListener('click',async e=>{const r=canvas.getBoundingClientRect(),x=(e.clientX-r.left)/r.width*state.width,y=(e.clientY-r.top)/r.height*state.height;const response=await fetch('/api/place',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({resource:document.querySelector('#element').value,x,y})});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}await refresh()});
document.querySelector('#placeMolecule').onclick=async()=>{try{const payload=JSON.parse(document.querySelector('#molecule').value);const response=await fetch('/api/molecule',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}await refresh()}catch(error){document.querySelector('#status').textContent='Invalid molecule JSON: '+error.message}};
document.querySelector('#reset').onclick=async()=>{const payload={width:Number(document.querySelector('#width').value),height:Number(document.querySelector('#height').value),cell_size:5,ticks_per_second:20,seed:Number(document.querySelector('#seed').value)};const response=await fetch('/api/reset',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}await refresh()};
document.querySelector('#step').onclick=async()=>{await fetch('/api/step',{method:'POST',headers:{'content-type':'application/json'},body:'{"count":1}'});await refresh()};
document.querySelector('#step100').onclick=async()=>{await fetch('/api/step',{method:'POST',headers:{'content-type':'application/json'},body:'{"count":100}'});await refresh()};
(async()=>{const names=['Carbon','Oxygen','Hydrogen','Water','Nitrogen','Phosphorus','Sulfur'];document.querySelector('#element').innerHTML=names.map(n=>'<option>'+n+'</option>').join('');await refresh();new ResizeObserver(resize).observe(canvas)})();
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laboratory_starts_empty_and_without_organisms() {
        let laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        let snapshot = laboratory.snapshot();
        assert_eq!(snapshot.tick, 0);
        assert_eq!(snapshot.material_count, 0);
        assert_eq!(snapshot.organisms, 0);
        assert!(snapshot.materials.is_empty());
    }

    #[test]
    fn placed_elements_receive_stable_physical_identity() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        let identity = laboratory
            .place_element("Carbon", 20.0, 30.0)
            .expect("carbon placement");
        assert_ne!(identity, 0);
        let snapshot = laboratory.snapshot();
        assert_eq!(snapshot.material_count, 1);
        assert_eq!(snapshot.materials[0].identity, identity);
        assert_eq!(
            snapshot.materials[0].parts,
            vec![("Carbon".to_string(), 1.0)]
        );
    }

    #[test]
    fn laboratory_rejects_invalid_dimensions_and_out_of_bounds_placement() {
        let mut invalid = LaboratoryConfig::default();
        invalid.width = f64::NAN;
        assert!(Laboratory::new(invalid).is_err());

        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        assert!(laboratory.place_element("Carbon", 100.0, 10.0).is_err());
        assert!(laboratory
            .place_element("Unobtainium", 10.0, 10.0)
            .is_err());
    }

    #[test]
    fn laboratory_steps_the_shared_simulation_without_creating_an_organism() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory.step(3).expect("three physics steps");
        let snapshot = laboratory.snapshot();
        assert_eq!(snapshot.tick, 3);
        assert_eq!(snapshot.organisms, 0);
    }
}
