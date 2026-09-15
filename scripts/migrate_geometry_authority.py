from pathlib import Path
import re

def remove_fn(text, name):
    m = re.search(r'(?m)^\s*(?:pub(?:\(crate\))?\s+)?fn\s+' + re.escape(name) + r'\s*\(', text)
    if not m: return text
    start = text.rfind('\n', 0, m.start()) + 1
    brace = text.find('{', m.end()); depth = 0
    for i in range(brace, len(text)):
        if text[i] == '{': depth += 1
        elif text[i] == '}':
            depth -= 1
            if depth == 0:
                end = i + 1
                if end < len(text) and text[end] == '\n': end += 1
                return text[:start] + text[end:]
    raise RuntimeError(name)

def remove_type(text, marker):
    start = text.index(marker); brace = text.find('{', start); depth = 0
    for i in range(brace, len(text)):
        if text[i] == '{': depth += 1
        elif text[i] == '}':
            depth -= 1
            if depth == 0:
                end = i + 1
                if end < len(text) and text[end] == '\n': end += 1
                return text[:start] + text[end:]
    raise RuntimeError(marker)

p=Path('src/resources.rs'); t=p.read_text()
a=t.index('#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]\npub struct ConnectionPoint')
b=t.index('#[derive(Serialize, Deserialize, Clone, Copy, Debug)]\npub struct ResourceBaselines',a)
t=t[:a]+t[b:]
t=remove_fn(t,'connection_sites')
for n in ['line_has_exactly_two_endpoint_connection_points','water_is_fluid_but_has_circle_default_geometry','every_polygonal_resource_has_one_connection_point_per_corner','polygon_connection_points_correspond_to_actual_vertices','connection_points_are_valid_where_present','circle_has_no_finite_connection_point_list','connection_point_has_no_independent_strength_field']:
    t=remove_fn(t,n)
t=t.replace('            assert_eq!(\n                restored.shape.connection_sites(),\n                resource.shape.connection_sites()\n            );\n','')
p.write_text(t)

p=Path('src/structure.rs'); t=p.read_text()
t=t.replace('use crate::resources::{BaseResource, ConnectionPoint, ConnectionSites, Material};','use crate::resources::{BaseResource, Material};')
t=remove_fn(t,'connection_sites')
for n in ['connection_site','available_connection_sites','component_connection_sites']: t=remove_fn(t,n)
try: t=remove_type(t,'#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub struct ConnectionSiteRef')
except ValueError: pass
old='''            Self::Boundary { .. } => match unit.connection_sites(catalog)? {\n                ConnectionSites::Circumference { radius } => Some(ConnectionRegion::Boundary {\n                    center_x: unit.placement.x,\n                    center_y: unit.placement.y,\n                    radius,\n                }),\n                _ => None,\n            },\n            Self::Fluid { .. } => match unit.connection_sites(catalog)? {\n                ConnectionSites::Undetermined => {\n                    let radius = unit.shape(catalog)?.form.bounding_radius();\n                    Some(ConnectionRegion::Fluid {\n                        center_x: unit.placement.x,\n                        center_y: unit.placement.y,\n                        effective_radius: radius,\n                    })\n                }\n                _ => None,\n            },'''
new='''            Self::Boundary { .. } => {\n                let crate::resources::Form::Circle { radius } = unit.shape(catalog)?.form else { return None; };\n                Some(ConnectionRegion::Boundary { center_x: unit.placement.x, center_y: unit.placement.y, radius })\n            }\n            Self::Fluid { .. } => {\n                let radius = unit.shape(catalog)?.form.bounding_radius();\n                Some(ConnectionRegion::Fluid { center_x: unit.placement.x, center_y: unit.placement.y, effective_radius: radius })\n            },'''
t=t.replace(old,new)
old='''            Self::Boundary { angle_radians } => {\n                let ConnectionSites::Circumference { radius } = unit.connection_sites(catalog)?\n                else {\n                    return None;\n                };\n                let (nx, ny) = (angle_radians.cos(), angle_radians.sin());'''
new='''            Self::Boundary { angle_radians } => {\n                let crate::resources::Form::Circle { radius } = unit.shape(catalog)?.form else { return None; };\n                let (nx, ny) = (angle_radians.cos(), angle_radians.sin());'''
t=t.replace(old,new)
p.write_text(t)

p=Path('src/connection_geometry.rs'); t=p.read_text()
t=t.replace('use crate::resources::{ConnectionPoint, ConnectionSites, Shape};','use crate::resources::Shape;')
for n in ['transform_connection_point','transform_connection_regions','legacy_transform_remains_only_a_migration_adapter','distance_is_euclidean','directly_facing_normals_have_maximum_compatibility','perpendicular_surfaces_have_zero_compatibility','continuous_sites_map_to_regions']:
    t=remove_fn(t,n)
try: t=remove_fn(t,'cp')
except Exception: pass
t=t.replace('''        let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0);\n        let b = transform_connection_point(cp(0.0, 0.0, 0.0), 1.0, 0.0, 0.0);''','''        let a = WorldConnectionPoint { x: 0.0, y: 0.0, normal_x: 1.0, normal_y: 0.0 };\n        let b = WorldConnectionPoint { x: 1.0, y: 0.0, normal_x: -1.0, normal_y: 0.0 };''')
p.write_text(t)

p=Path('src/contact.rs'); t=p.read_text()
t=t.replace('    facing_compatibility, point_distance, rigid_endpoint_world_point, transform_connection_point,','    facing_compatibility, point_distance, rigid_endpoint_world_point,')
t=t.replace('use crate::resources::{ConnectionPoint, Form};','use crate::resources::Form;')
for n in ['transform_point','world_connection_point','connection_points_contact','connection_point_distance']: t=remove_fn(t,n)
p.write_text(t)
print('geometry authority migration complete')
