from pathlib import Path
p=Path('src/connection_geometry.rs')
t=p.read_text().replace('/// metadata. This is retained solely for migration compatibility.\n///\n', '')
p.write_text(t)
