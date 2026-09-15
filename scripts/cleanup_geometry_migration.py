from pathlib import Path
import re

p = Path('src/resources.rs')
t = p.read_text()
t = re.sub(r'(?m)^(    #\[test\]\n){2,}', '    #[test]\n', t)
p.write_text(t)

legacy = re.compile(r'\b(?:ConnectionPoint|ConnectionSites)\b')
for path in Path('src').glob('*.rs'):
    text = path.read_text()
    if legacy.search(text):
        raise SystemExit(f'legacy connection geometry remains in {path}')
print('one-authority geometry audit: no legacy connection-site types remain')
