from pathlib import Path

p=Path('src/connection_geometry.rs')
t=p.read_text().replace('/// metadata. This is retained solely for migration compatibility.\n///\n/// Derive and transform a rigid polygon vertex', '/// Derive and transform a rigid polygon vertex')
p.write_text(t)

p=Path('src/rigid_boundary.rs')
t=p.read_text().replace('use crate::resources::{default_catalog, Form, Shape};', 'use crate::resources::{Form, Shape};')
t=t.replace('    use super::*;\n', '    use super::*;\n    use crate::resources::default_catalog;\n', 1)
p.write_text(t)

p=Path('src/resources.rs')
t=p.read_text().replace('nominal_area.is_finite() && nominal_area.to_owned() > 0.0', 'nominal_area.is_finite() && *nominal_area > 0.0')
p.write_text(t)
