//! Dedicated experimental workbench backed by the shared EvoSim engine.

use crate::{
    environment::ActiveMaterialField,
    physical_material::PhysicalMaterial,
    resources::{InternalBond, Material},
    state::{EnergyLedger, Environment, Simulation},
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LaboratoryFlowConfig {
    /// Mean current direction in degrees; zero points right and 90 points down.
    pub direction_degrees: f64,
    /// Flow speed multiplier. Zero disables ambient transport.
    pub speed: f64,
    /// Blend between a uniform current (0) and the native S-shaped field (1).
    pub strength: f64,
    /// Local recirculation intensity around the two S-bend vortices.
    pub vortex_strength: f64,
}

impl Default for LaboratoryFlowConfig {
    fn default() -> Self {
        Self {
            direction_degrees: 0.0,
            speed: 1.0,
            strength: 1.0,
            vortex_strength: 1.0,
        }
    }
}

impl LaboratoryFlowConfig {
    fn validate(self) -> Result<(), &'static str> {
        if !self.direction_degrees.is_finite()
            || !(-360.0..=360.0).contains(&self.direction_degrees)
        {
            return Err("flow direction must be in [-360, 360] degrees");
        }
        if !self.speed.is_finite() || !(0.0..=10.0).contains(&self.speed) {
            return Err("flow speed must be in [0, 10]");
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return Err("flow strength must be in [0, 1]");
        }
        if !self.vortex_strength.is_finite()
            || !(0.0..=2.0).contains(&self.vortex_strength)
        {
            return Err("local vortex strength must be in [0, 2]");
        }
        Ok(())
    }

    fn engine_config(self) -> crate::environment_flow::FlowFieldConfig {
        crate::environment_flow::FlowFieldConfig {
            direction_degrees: self.direction_degrees,
            speed: self.speed,
            strength: self.strength,
            vortex_strength: self.vortex_strength,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaboratoryConfig {
    pub width: f64,
    pub height: f64,
    pub cell_size: f64,
    pub ticks_per_second: f64,
    pub seed: u64,
    #[serde(default)]
    pub flow: LaboratoryFlowConfig,
}

impl Default for LaboratoryConfig {
    fn default() -> Self {
        Self {
            width: 100.0,
            height: 100.0,
            cell_size: 5.0,
            ticks_per_second: 20.0,
            seed: 42,
            flow: LaboratoryFlowConfig::default(),
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
        self.flow.validate()?;
        Ok(())
    }
}

pub struct Laboratory {
    pub config: LaboratoryConfig,
    simulation: Simulation,
    run_generation: u64,
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
            flow_field: config.flow.engine_config(),
        };
        Ok(Self {
            config,
            simulation,
            run_generation: 0,
        })
    }

    /// Replace the ambient vector field immediately, including during a live run.
    pub fn configure_flow(&mut self, flow: LaboratoryFlowConfig) -> Result<(), &'static str> {
        flow.validate()?;
        self.config.flow = flow;
        self.simulation.flow_field = flow.engine_config();
        Ok(())
    }

    pub fn place_element(&mut self, resource: &str, x: f64, y: f64) -> Result<u64, &'static str> {
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
        let (anchor_x, anchor_y) = (anchor.x, anchor.y);
        self.place_material(material, placements, anchor_x, anchor_y)
    }

    fn place_material(
        &mut self,
        material: Material,
        placements: Vec<Placement>,
        x: f64,
        y: f64,
    ) -> Result<u64, &'static str> {
        if self.simulation.tick != 0 || self.simulation.running {
            return Err("initial material placement requires a paused tick-zero experiment");
        }
        let identity = {
            let catalog = &self.simulation.environment.catalog;
            let physical = PhysicalMaterial::realized(material, placements, catalog)
                .ok_or("physical realization or bond geometry validation failed")?;
            let field = &mut self.simulation.environment.field;
            if !field.deposit_physical(x, y, physical, catalog) {
                return Err("environment rejected material");
            }
            field.assign_missing_material_identities();
            let index = field
                .index_for_position(x, y)
                .ok_or("invalid destination")?;
            field.cells[index]
                .physical_materials
                .last()
                .map(|material| material.environmental_identity)
                .ok_or("identity assignment failed")?
        };

        // Placed bonded molecules are part of the experiment's initial
        // condition, not a reaction. Move the tick-zero structural baseline
        // with that explicitly supplied initial material so its bond potential
        // is not mistaken for energy created by the simulator.
        let initial_structural_energy = self
            .simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(|material| {
                material
                    .internal_connections
                    .as_ref()
                    .map(|bonds| -bonds.iter().map(|bond| bond.bond_energy).sum::<f64>())
                    .unwrap_or(0.0)
            })
            .sum::<f64>();
        if !initial_structural_energy.is_finite() {
            return Err("initial structural energy is not finite");
        }
        self.simulation.energy_ledger.total_structural_energy_held = initial_structural_energy;
        self.simulation.energy_ledger.initial_structural_energy_held = initial_structural_energy;
        Ok(identity)
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
                    .map(|placement| [placement.x, placement.y, placement.rotation_radians])
                    .collect(),
                bond_count: material.internal_connections.as_ref().map_or(0, Vec::len),
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
            running: self.simulation.running,
            resources: self
                .simulation
                .environment
                .catalog
                .iter()
                .map(|resource| resource.name.clone())
                .collect(),
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
            flow: self.config.flow,
        }
    }
}

#[derive(Serialize)]
pub struct LaboratorySnapshot {
    pub tick: u64,
    pub running: bool,
    pub resources: Vec<String>,
    pub width: f64,
    pub height: f64,
    pub material_count: usize,
    pub materials: Vec<MaterialSnapshot>,
    pub chemistry_usable_energy: f64,
    pub resonant_energy: Option<f64>,
    pub organisms: usize,
    pub flow: LaboratoryFlowConfig,
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
    let mut replacement =
        Laboratory::new(config).map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    let snapshot = replacement.snapshot();
    let mut current = state.0.lock().await;
    replacement.run_generation = current.run_generation.wrapping_add(1);
    *current = replacement;
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

async fn start(State(state): State<LabState>) -> Json<LaboratorySnapshot> {
    let generation = {
        let mut laboratory = state.0.lock().await;
        if laboratory.simulation.running {
            None
        } else {
            laboratory.simulation.running = true;
            laboratory.run_generation = laboratory.run_generation.wrapping_add(1);
            Some(laboratory.run_generation)
        }
    };
    if let Some(generation) = generation {
        let shared = state.0.clone();
        tokio::spawn(async move {
            loop {
                let interval = {
                    let laboratory = shared.lock().await;
                    if !laboratory.simulation.running || laboratory.run_generation != generation {
                        break;
                    }
                    std::time::Duration::from_secs_f64(1.0 / laboratory.config.ticks_per_second)
                };
                tokio::time::sleep(interval).await;
                let mut laboratory = shared.lock().await;
                if !laboratory.simulation.running || laboratory.run_generation != generation {
                    break;
                }
                laboratory.simulation.step();
            }
        });
    }
    Json(state.0.lock().await.snapshot())
}

async fn pause(State(state): State<LabState>) -> Json<LaboratorySnapshot> {
    let mut laboratory = state.0.lock().await;
    laboratory.simulation.running = false;
    laboratory.run_generation = laboratory.run_generation.wrapping_add(1);
    Json(laboratory.snapshot())
}

async fn configure_flow(
    State(state): State<LabState>,
    Json(flow): Json<LaboratoryFlowConfig>,
) -> Result<Json<LaboratorySnapshot>, (StatusCode, String)> {
    let mut laboratory = state.0.lock().await;
    laboratory
        .configure_flow(flow)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(laboratory.snapshot()))
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
    let app_state = LabState(Arc::new(Mutex::new(laboratory)));
    let app = Router::new()
        .route("/", get(index))
        .route("/api/state", get(state))
        .route("/api/reset", post(reset))
        .route("/api/place", post(place))
        .route("/api/molecule", post(molecule))
        .route("/api/run", post(start))
        .route("/api/pause", post(pause))
        .route("/api/step", post(step))
        .route("/api/flow", post(configure_flow))
        .with_state(app_state);
    let address = std::env::var("EVOSIM_LAB_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:3030".into())
        .parse::<SocketAddr>()
        .expect("valid EVOSIM_LAB_ADDRESS");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("lab listener binds");
    println!("EvoSim Chemical Laboratory: http://{address}");
    axum::serve(listener, app).await.expect("lab server failed");
}

const INDEX_HTML: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>EvoSim Chemical Laboratory</title><style>
:root{color-scheme:dark;font:15px system-ui;background:#081b2a;color:#e7f3fa}body{margin:0;display:grid;grid-template-rows:auto 1fr;min-height:100vh}header{padding:12px 18px;background:#102b40;border-bottom:1px solid #28506a;display:flex;gap:18px;align-items:center;flex-wrap:wrap}main{display:grid;grid-template-columns:minmax(0,1fr) 300px;min-height:0}canvas{width:100%;height:100%;background:#0b3147;display:block;touch-action:manipulation}aside{padding:16px;background:#102638;border-left:1px solid #28506a}button,select,input,textarea{background:#173c54;color:#e7f3fa;border:1px solid #46718a;border-radius:5px;padding:8px}#status{color:#9ec8df}#materials{font-size:13px;line-height:1.7;white-space:pre-wrap;overflow-wrap:anywhere}textarea{width:95%}@media(max-width:650px){main{grid-template-columns:1fr;grid-template-rows:minmax(55vh,1fr) auto}aside{border-left:0;border-top:1px solid #28506a}}
</style></head><body><header><strong>EvoSim · Chemical Laboratory</strong><button id="run">Run</button><button id="pause">Pause</button><button id="step">Step ×1</button><button id="step100">Step ×100</button><label>Width <input id="width" type="number" value="100" min="1" max="10000" style="width:65px"></label><label>Height <input id="height" type="number" value="100" min="1" max="10000" style="width:65px"></label><label>Seed <input id="seed" type="number" value="42" style="width:75px"></label><button id="reset">Reset world</button><label>Element <select id="element"></select></label><span id="status">Loading…</span></header><main><canvas id="world"></canvas><aside><h3>Experiment state</h3><div id="summary"></div><h3>Ambient flow field</h3><label>Direction <input id="flowDirection" type="range" min="-180" max="180" step="1" value="0"><output id="flowDirectionValue">0°</output></label><br><label>Speed <input id="flowSpeed" type="range" min="0" max="5" step="0.1" value="1"><output id="flowSpeedValue">1.0</output></label><br><label>Flow structure <input id="flowStrength" type="range" min="0" max="1" step="0.05" value="1"><output id="flowStrengthValue">1.00</output></label><br><label>Local vortices <input id="flowVortices" type="range" min="0" max="2" step="0.1" value="1"><output id="flowVorticesValue">1.0</output></label><p>Changes apply immediately, including during a running experiment. Strength 0 is a uniform current; strength 1 restores the S-shaped field.</p><h3>Physical materials</h3><div id="materials"></div><h3>Place molecule (JSON)</h3><textarea id="molecule" rows="5">{"parts":["Carbon","Oxygen"],"bonds":[[0,1]],"placements":[[45,50,0],[47,50,0]]}</textarea><button id="placeMolecule">Place molecule</button><p>Tap the field to place the selected element. Molecules use the exact constituent coordinates in the editor above.</p></aside></main><script>
const canvas=document.querySelector('#world'),ctx=canvas.getContext('2d');let state=null;const palette={Carbon:'#9aa5ae',Oxygen:'#e04444',Hydrogen:'#dce8ee',Water:'#328ed0',Nitrogen:'#9b7045',Phosphorus:'#e3a34c',Sulfur:'#e2d34b'};
function resize(){const r=canvas.getBoundingClientRect(),d=devicePixelRatio||1;canvas.width=r.width*d;canvas.height=r.height*d;draw()}
function flowVector(x,y){const f=state.flow,phase=Math.PI*2*x/state.width,center=state.height*.5+state.height*.23*Math.sin(phase),slope=state.height*.23*Math.PI*2/state.width*Math.cos(phase),dy=((y-center+state.height*.5)%state.height+state.height)%state.height-state.height*.5,scale=Math.max(state.height*.16,1),envelope=Math.exp(-.5*(dy/scale)**2),tl=Math.hypot(1,slope),curved={x:(1/tl)*(.12+.88*envelope),y:(slope/tl)*(.12+.88*envelope)},a=f.direction_degrees*Math.PI/180;let vx=Math.cos(a)*(1-f.strength)+curved.x*f.strength,vy=Math.sin(a)*(1-f.strength)+curved.y*f.strength;const radius=Math.max(state.height*.13,1);for(const [fraction,sign] of [[.25,1],[.75,-1]]){const cx=state.width*fraction,cy=state.height*.5+state.height*.23*Math.sin(Math.PI*2*fraction),dx=x-cx,ddy=((y-cy+state.height*.5)%state.height+state.height)%state.height-state.height*.5,w=Math.exp(-(dx*dx+ddy*ddy)/(2*radius*radius))*sign*1.2*f.vortex_strength*f.strength;vx+=-ddy/radius*w;vy+=dx/radius*w}return {x:vx*f.speed,y:vy*f.speed}}
function drawFlow(sx,sy){if(!state.flow.speed)return;ctx.strokeStyle='rgba(117,200,232,.48)';ctx.fillStyle='rgba(117,200,232,.58)';ctx.lineWidth=1.3;for(let y=state.height/8;y<state.height;y+=state.height/4)for(let x=state.width/10;x<state.width;x+=state.width/5){const v=flowVector(x,y),l=Math.hypot(v.x,v.y);if(l<.02)continue;const ux=v.x/l,uy=v.y/l,len=Math.min(10,3+l*2),x1=x*sx,y1=y*sy,x2=x1+ux*len*sx,y2=y1+uy*len*sy;ctx.beginPath();ctx.moveTo(x1,y1);ctx.lineTo(x2,y2);ctx.stroke();ctx.beginPath();ctx.moveTo(x2,y2);ctx.lineTo(x2-ux*3*sx-uy*2*sy,y2-uy*3*sy+ux*2*sx);ctx.lineTo(x2-ux*3*sx+uy*2*sy,y2-uy*3*sy-ux*2*sx);ctx.closePath();ctx.fill()}}
function draw(){if(!state)return;const sx=canvas.width/state.width,sy=canvas.height/state.height;ctx.clearRect(0,0,canvas.width,canvas.height);ctx.strokeStyle='#17465d';ctx.lineWidth=1;for(let x=0;x<=state.width;x+=10){ctx.beginPath();ctx.moveTo(x*sx,0);ctx.lineTo(x*sx,canvas.height);ctx.stroke()}for(let y=0;y<=state.height;y+=10){ctx.beginPath();ctx.moveTo(0,y*sy);ctx.lineTo(canvas.width,y*sy);ctx.stroke()}drawFlow(sx,sy);for(const m of state.materials){ctx.strokeStyle='#a7d8ed';ctx.lineWidth=Math.max(1,Math.min(sx,sy)*.12);for(const bond of m.bonds){const a=m.placements[bond[0]],b=m.placements[bond[1]];if(!a||!b)continue;ctx.beginPath();ctx.moveTo(a[0]*sx,a[1]*sy);ctx.lineTo(b[0]*sx,b[1]*sy);ctx.stroke()}for(let i=0;i<m.parts.length;i++){const p=m.placements[i];if(!p)continue;ctx.beginPath();ctx.arc(p[0]*sx,p[1]*sy,Math.max(3,Math.min(sx,sy)*.65),0,Math.PI*2);ctx.fillStyle=palette[m.parts[i][0]]||'#ddd';ctx.fill();ctx.strokeStyle='#071521';ctx.stroke()}}}
async function refresh(){state=await(await fetch('/api/state')).json();document.querySelector('#summary').textContent='Tick: '+state.tick+' · '+(state.running?'Running':'Paused')+' · Materials: '+state.material_count+' · Organisms: '+state.organisms+' · Usable: '+state.chemistry_usable_energy.toFixed(3)+' · Resonance: '+(state.resonant_energy===null?'n/a':state.resonant_energy.toFixed(3));document.querySelector('#materials').textContent=state.materials.map(m=>'#'+m.identity+' '+m.parts.map(p=>p[0]).join('–')+' · bonds '+m.bond_count).join('\n')||'No materials placed';syncFlowControls();document.querySelector('#status').textContent='Ready';draw()}
canvas.addEventListener('click',async e=>{const r=canvas.getBoundingClientRect(),x=(e.clientX-r.left)/r.width*state.width,y=(e.clientY-r.top)/r.height*state.height;const response=await fetch('/api/place',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({resource:document.querySelector('#element').value,x,y})});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}await refresh()});
document.querySelector('#placeMolecule').onclick=async()=>{try{const payload=JSON.parse(document.querySelector('#molecule').value);const response=await fetch('/api/molecule',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}await refresh()}catch(error){document.querySelector('#status').textContent='Invalid molecule JSON: '+error.message}};
function syncFlowControls(){if(!state)return;const f=state.flow;for(const [id,key] of [['flowDirection','direction_degrees'],['flowSpeed','speed'],['flowStrength','strength'],['flowVortices','vortex_strength']]){const el=document.querySelector('#'+id);if(document.activeElement!==el)el.value=f[key]}document.querySelector('#flowDirectionValue').textContent=Number(f.direction_degrees).toFixed(0)+'°';document.querySelector('#flowSpeedValue').textContent=Number(f.speed).toFixed(1);document.querySelector('#flowStrengthValue').textContent=Number(f.strength).toFixed(2);document.querySelector('#flowVorticesValue').textContent=Number(f.vortex_strength).toFixed(1)}
async function updateFlow(){const flow={direction_degrees:Number(document.querySelector('#flowDirection').value),speed:Number(document.querySelector('#flowSpeed').value),strength:Number(document.querySelector('#flowStrength').value),vortex_strength:Number(document.querySelector('#flowVortices').value)};for(const [id,key] of [['flowDirection','direction_degrees'],['flowSpeed','speed'],['flowStrength','strength'],['flowVortices','vortex_strength']])document.querySelector('#'+id+'Value').textContent=flow[key].toFixed(id==='flowStrength'?2:id==='flowDirection'?0:1)+(id==='flowDirection'?'°':'');const response=await fetch('/api/flow',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(flow)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}state=await response.json();draw()}
for(const id of ['flowDirection','flowSpeed','flowStrength','flowVortices'])document.querySelector('#'+id).addEventListener('change',updateFlow);
document.querySelector('#reset').onclick=async()=>{const payload={width:Number(document.querySelector('#width').value),height:Number(document.querySelector('#height').value),cell_size:5,ticks_per_second:20,seed:Number(document.querySelector('#seed').value),flow:state.flow};const response=await fetch('/api/reset',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload)});if(!response.ok){document.querySelector('#status').textContent=await response.text();return}await refresh()};
document.querySelector('#run').onclick=async()=>{await fetch('/api/run',{method:'POST'});await refresh()};
document.querySelector('#pause').onclick=async()=>{await fetch('/api/pause',{method:'POST'});await refresh()};
document.querySelector('#step').onclick=async()=>{await fetch('/api/step',{method:'POST',headers:{'content-type':'application/json'},body:'{"count":1}'});await refresh()};
document.querySelector('#step100').onclick=async()=>{await fetch('/api/step',{method:'POST',headers:{'content-type':'application/json'},body:'{"count":100}'});await refresh()};
(async()=>{await refresh();document.querySelector('#element').innerHTML=state.resources.map(n=>'<option>'+n+'</option>').join('');new ResizeObserver(resize).observe(canvas);setInterval(()=>{if(state&&state.running)refresh()},250)})();
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laboratory_starts_empty_and_without_organisms() {
        let laboratory = Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
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
    fn molecule_placement_preserves_the_requested_physical_bond_graph() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        let identity = laboratory
            .place_molecule(
                vec!["Carbon".into(), "Hydrogen".into()],
                vec![[0, 1]],
                vec![[20.0, 20.0, 0.1], [21.0, 20.0, 0.4]],
            )
            .expect("valid carbon-hydrogen geometry");
        let snapshot = laboratory.snapshot();
        let molecule = snapshot
            .materials
            .iter()
            .find(|material| material.identity == identity)
            .expect("placed molecule is observable");
        assert_eq!(molecule.parts.len(), 2);
        assert_eq!(molecule.bonds, vec![[0, 1]]);
        assert_eq!(molecule.bond_count, 1);
        assert_eq!(molecule.placements.len(), 2);
        assert!(
            laboratory
                .simulation
                .energy_inventory_snapshot()
                .is_balanced_without_creation(1e-9),
            "pre-placed bond potential belongs to the tick-zero initial condition"
        );
    }

    #[test]
    fn laboratory_rejects_material_injection_after_the_experiment_starts() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory.step(1).expect("first tick");
        assert!(laboratory.place_element("Carbon", 10.0, 10.0).is_err());
    }

    #[test]
    fn laboratory_rejects_invalid_dimensions_and_out_of_bounds_placement() {
        let mut invalid = LaboratoryConfig::default();
        invalid.width = f64::NAN;
        assert!(Laboratory::new(invalid).is_err());

        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        assert!(laboratory.place_element("Carbon", 100.0, 10.0).is_err());
        assert!(laboratory.place_element("Unobtainium", 10.0, 10.0).is_err());
    }

    #[test]
    fn laboratory_flow_can_be_changed_while_running_and_is_reflected_in_state() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        laboratory.simulation.running = true;
        let flow = LaboratoryFlowConfig {
            direction_degrees: 90.0,
            speed: 2.5,
            strength: 0.0,
            vortex_strength: 0.0,
        };
        laboratory
            .configure_flow(flow)
            .expect("valid live flow change");
        assert_eq!(laboratory.snapshot().flow, flow);
        assert_eq!(
            laboratory.simulation.flow_field,
            flow.engine_config()
        );
        assert!(laboratory.simulation.running);
    }

    #[test]
    fn laboratory_rejects_invalid_flow_without_changing_the_current_field() {
        let mut laboratory =
            Laboratory::new(LaboratoryConfig::default()).expect("valid lab config");
        let original = laboratory.config.flow;
        let invalid = LaboratoryFlowConfig {
            speed: f64::NAN,
            ..original
        };
        assert!(laboratory.configure_flow(invalid).is_err());
        assert_eq!(laboratory.config.flow, original);
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
