from pathlib import Path
p=Path('src/connection_geometry.rs')
t=p.read_text()
old='''/// Transform a point whose normal is supplied by the legacy serialized connection\n/// metadata. This is retained solely for migration compatibility.\n\n'''
if old not in t:
    raise SystemExit('stale legacy geometry comment block not found')
t=t.replace(old,'',1)
p.write_text(t)
