// Active material field: fixed-resolution 2D grid holding ecological material stock.
use crate::material_transfer::take_whole_unstructured;
use crate::resources::{merge_parts, Material};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CELL_SIZE: f64 = 25.0;
pub const DEFAULT_DIFFUSION_FRACTION: f64 = 0.05;
pub const MATERIAL_EPSILON: f64 = 1e-9;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldCell {
    /// Ecological stock occupying this field cell. Unstructured stock may be
    /// aggregated; structured material remains an intact physical object.
    pub materials: Vec<Material>,
}
impl FieldCell {
    pub fn empty() -> Self { Self { materials: Vec::new() } }
    pub fn total_amount(&self) -> f64 { self.materials.iter().map(Material::total_amount).sum() }
    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals = Vec::new();
        for material in &self.materials {
            for component in material.composition() {
                if let Some(existing) = totals.iter_mut().find(|(n, _)| n == &component.resource) { existing.1 += component.amount; }
                else { totals.push((component.resource.clone(), component.amount)); }
            }
            if let Some(structure) = material.structure() {
                for constituent in &structure.constituents {
                    if let Some(existing) = totals.iter_mut().find(|(n, _)| n == &constituent.resource) { existing.1 += 1.0; }
                    else { totals.push((constituent.resource.clone(), 1.0)); }
                }
            }
        }
        totals
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActiveMaterialField { pub cell_size: f64, pub width_cells: usize, pub height_cells: usize, pub cells: Vec<FieldCell> }
impl ActiveMaterialField {
    pub fn new(world_width:f64, world_height:f64, cell_size:f64)->Self { let cell_size=cell_size.max(1.0); let width_cells=(world_width/cell_size).ceil().max(1.0)as usize; let height_cells=(world_height/cell_size).ceil().max(1.0)as usize; let cells=(0..width_cells*height_cells).map(|_|FieldCell::empty()).collect(); Self{cell_size,width_cells,height_cells,cells} }
    pub fn row_col_for_position(&self,x:f64,y:f64)->Option<(usize,usize)>{if !x.is_finite()||!y.is_finite()||x<0.0||y<0.0{return None}let col=(x/self.cell_size).floor()as usize;let row=(y/self.cell_size).floor()as usize;if col>=self.width_cells||row>=self.height_cells{return None}Some((row,col))}
    pub fn index_for_position(&self,x:f64,y:f64)->Option<usize>{self.row_col_for_position(x,y).map(|(r,c)|r*self.width_cells+c)}
    fn row_col_for_index(&self,index:usize)->(usize,usize){(index/self.width_cells,index%self.width_cells)}
    pub fn cell_center(&self,index:usize)->(f64,f64){let(r,c)=self.row_col_for_index(index);((c as f64+0.5)*self.cell_size,(r as f64+0.5)*self.cell_size)}
    pub fn cells_within_radius(&self,x:f64,y:f64,radius:f64)->Vec<usize>{if !x.is_finite()||!y.is_finite()||!radius.is_finite()||radius<0.0||self.cells.is_empty()||self.width_cells==0||self.height_cells==0{return Vec::new()}let min_x=(x-radius).max(0.0);let max_x=x+radius;let min_y=(y-radius).max(0.0);let max_y=y+radius;let min_col=(min_x/self.cell_size).floor()as usize;let max_col=((max_x/self.cell_size).floor()as usize).min(self.width_cells.saturating_sub(1));let min_row=(min_y/self.cell_size).floor()as usize;let max_row=((max_y/self.cell_size).floor()as usize).min(self.height_cells.saturating_sub(1));if min_col>=self.width_cells||min_row>=self.height_cells||min_col>max_col||min_row>max_row{return Vec::new()}let rs=radius*radius;let mut out=Vec::new();for row in min_row..=max_row{for col in min_col..=max_col{let i=row*self.width_cells+col;let(cx,cy)=self.cell_center(i);let dx=cx-x;let dy=cy-y;if dx*dx+dy*dy<=rs{out.push(i)}}}out}
    pub fn neighbor_indices(&self,index:usize)->Vec<usize>{let(r,c)=self.row_col_for_index(index);let mut out=Vec::with_capacity(4);if r>0{out.push((r-1)*self.width_cells+c)}if r+1<self.height_cells{out.push((r+1)*self.width_cells+c)}if c>0{out.push(r*self.width_cells+c-1)}if c+1<self.width_cells{out.push(r*self.width_cells+c+1)}out}
    pub fn deposit(&mut self,x:f64,y:f64,material:Material)->bool{match self.index_for_position(x,y){Some(i)=>{self.deposit_at_index(i,material);true},None=>false}}
    pub fn deposit_at_index(&mut self,index:usize,material:Material){if material.is_empty()||!material.is_valid(){return}let cell=&mut self.cells[index];if material.is_structured(){cell.materials.push(material);return}if let Some(existing)=cell.materials.iter_mut().find(|m|!m.is_structured()){let mut composition=existing.composition().to_vec();composition.extend_from_slice(material.composition());existing.composition=merge_parts(&composition);}else{cell.materials.push(material)}}
    pub fn take_at(&mut self,x:f64,y:f64,material_index:usize,amount:f64)->Option<Material>{let i=self.index_for_position(x,y)?;self.take_at_index(i,material_index,amount)}
    pub fn take_at_index(&mut self,index:usize,material_index:usize,amount:f64)->Option<Material>{let material=self.cells.get_mut(index)?.materials.get_mut(material_index)?;let requested=if amount.is_finite()&&amount>=1.0{amount.floor()as usize}else{return None};let taken=take_whole_unstructured(material,requested)?;self.cells[index].materials.retain(|m|!m.is_empty());Some(taken)}
    pub fn take_for_acquisition(&mut self,index:usize)->Option<Material>{let cell=self.cells.get_mut(index)?;if let Some(i)=cell.materials.iter().position(|m|m.is_structured()&&m.is_valid()&&!m.is_empty()){return Some(cell.materials.swap_remove(i))}let i=cell.materials.iter().position(|m|!m.is_structured()&&!m.is_empty()&&m.is_valid())?;let p=cell.materials[i].composition().iter().position(|c|c.amount.is_finite()&&c.amount>=1.0&&c.amount.fract().abs()<=MATERIAL_EPSILON)?;let name=cell.materials[i].composition()[p].resource.clone();cell.materials[i].composition[p].amount-=1.0;cell.materials[i].composition.retain(|c|c.amount>MATERIAL_EPSILON);let m=Material::free_base(name,1.0);cell.materials.retain(|m|!m.is_empty());Some(m)}
    pub fn diffuse_step(&mut self,fraction:f64){let fraction=fraction.clamp(0.0,1.0);if fraction<=0.0{return}let n=self.cells.len();let mut outgoing:Vec<Vec<Material>>=(0..n).map(|_|Vec::new()).collect();for(i,out)in outgoing.iter_mut().enumerate(){let neighbors=self.neighbor_indices(i);if neighbors.is_empty(){continue}let count=self.cells[i].materials.len();for j in 0..count{if self.cells[i].materials[j].is_structured(){continue}let total=self.cells[i].materials[j].total_amount();let flow=(total*fraction).floor()as usize;if flow>0{if let Some(piece)=take_whole_unstructured(&mut self.cells[i].materials[j],flow){out.push(piece)}}}self.cells[i].materials.retain(|m|!m.is_empty())}for(i,out)in outgoing.iter_mut().enumerate(){let neighbors=self.neighbor_indices(i);for m in out.drain(..){distribute_evenly(self,m,&neighbors)}}}
    pub fn total_material(&self)->Vec<(String,f64)>{let mut totals=Vec::new();for cell in &self.cells{for(name,amount)in cell.total_material(){if let Some(e)=totals.iter_mut().find(|(n,_)|n==&name){e.1+=amount}else{totals.push((name,amount))}}}totals}
    pub fn total_amount(&self)->f64{self.cells.iter().map(FieldCell::total_amount).sum()}
}
fn distribute_evenly(field:&mut ActiveMaterialField,mut mat:Material,neighbors:&[usize]){if neighbors.is_empty(){return}let total=mat.total_amount().floor()as usize;let base=total/neighbors.len();let rem=total%neighbors.len();for(k,&i)in neighbors.iter().enumerate(){let count=base+usize::from(k<rem);if count==0{continue}let piece=take_whole_unstructured(&mut mat,count);if let Some(piece)=piece{if !piece.is_empty(){field.deposit_at_index(i,piece)}}}}
