from pathlib import Path
import re

p = Path('src/resources.rs')
t = p.read_text()
marker = '#[derive(Serialize, Deserialize, Clone, Copy, Debug)]\npub struct ResourceBaselines'
if 'pub struct Shape' not in t:
    shape = '''#[derive(Serialize, Deserialize, Clone, Debug)]\npub struct Shape {\n    pub form: Form,\n}\n\nimpl Shape {\n    pub fn is_valid(&self) -> bool {\n        self.form.is_valid()\n    }\n}\n\n'''
    t = t.replace(marker, shape + marker, 1)
t = t.replace('*sides as usize', 'sides.to_owned() as usize')
t = t.replace('*nominal_area', 'nominal_area.to_owned()')
p.write_text(t)

p = Path('src/construction_runtime.rs')
t = p.read_text().replace('*candidate_length / 2.0', 'candidate_length.to_owned() / 2.0')
p.write_text(t)

p = Path('src/surface_geometry.rs')
t = p.read_text().replace('line_endpoint_toward(*length, target_x, target_y)', 'line_endpoint_toward(length.to_owned(), target_x, target_y)')
p.write_text(t)

p = Path('src/connection_geometry.rs')
t = p.read_text()
t = re.sub(r'(?m)^(    #\[test\]\n){2,}', '    #[test]\n', t)
t = t.replace('use std::f64::consts::{FRAC_PI_2, PI};', 'use std::f64::consts::FRAC_PI_2;')
p.write_text(t)
print('geometry cleanup repair complete')
