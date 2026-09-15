from pathlib import Path
p=Path('src/rigid_boundary.rs')
t=p.read_text()
t=t.replace('assert!((ny + 2.0_f64.sqrt() / 2.0).abs() < 1e-10);\n    }\n    #[test]\n    fn square_corner_normal_is_physical_bisector()', 'assert!((ny - 2.0_f64.sqrt() / 2.0).abs() < 1e-10);\n    }\n    #[test]\n    fn square_corner_normal_is_physical_bisector()', 1)
p.write_text(t)
