(() => {
  const panel = document.createElement('aside');
  panel.id = 'organism-inspector';
  panel.style.cssText = [
    'position:fixed','top:56px','right:16px','width:280px','max-height:calc(100vh - 80px)','overflow:auto','box-sizing:border-box','padding:14px','background:#fff','border:1px solid #888','border-radius:6px','box-shadow:0 3px 12px rgba(0,0,0,.15)','font:13px/1.45 sans-serif','color:#111','display:none','z-index:10'
  ].join(';');
  document.body.appendChild(panel);
  function number(value) { return Number.isFinite(value) ? value.toFixed(3) : '—'; }
  function row(label, value) { const div = document.createElement('div'); div.style.cssText = 'display:flex;justify-content:space-between;gap:12px;padding:3px 0;border-bottom:1px solid #eee;'; const left = document.createElement('span'); left.textContent = label; left.style.fontWeight = '600'; const right = document.createElement('span'); right.textContent = value; right.style.textAlign = 'right'; div.append(left, right); return div; }
  function renderOrganism(organism) {
    panel.replaceChildren();
    const heading = document.createElement('div'); heading.textContent = `Organism ${organism.id}`; heading.style.cssText = 'font-size:17px;font-weight:700;margin-bottom:10px;'; panel.appendChild(heading);
    panel.append(row('Position', `${number(organism.x)}, ${number(organism.y)}`),row('Units', String(organism.unit_count ?? 0)),row('Bonds', String(organism.bond_count ?? 0)),row('Width', number(organism.max_x - organism.min_x)),row('Height', number(organism.max_y - organism.min_y)),row('Visible parts', String(organism.silhouette?.length ?? 0)));
    const forms = {};
    for (const part of organism.silhouette || []) { let name = 'Unknown'; if (part.form?.Circle) name = 'Circle'; else if (part.form?.Line) name = 'Line'; else if (part.form?.Rectangle) name = 'Rectangle'; else if (part.form?.RegularPolygon) name = `${part.form.RegularPolygon.sides}-gon`; else if (part.form?.Polygon) name = 'Polygon'; else if (part.form?.Fluid) name = 'Fluid'; forms[name] = (forms[name] || 0) + 1; }
    const geometryHeading = document.createElement('div'); geometryHeading.textContent = 'Geometry'; geometryHeading.style.cssText = 'font-size:14px;font-weight:700;margin:12px 0 4px;'; panel.appendChild(geometryHeading);
    const names = Object.keys(forms).sort(); if (!names.length) panel.appendChild(row('Forms', 'None observed')); else for (const name of names) panel.appendChild(row(name, String(forms[name])));
    panel.style.display = 'block';
  }
  function showError(message) { panel.replaceChildren(); const heading = document.createElement('div'); heading.textContent = 'Organism inspector'; heading.style.cssText = 'font-size:17px;font-weight:700;margin-bottom:8px;'; const text = document.createElement('div'); text.textContent = message; panel.append(heading, text); panel.style.display = 'block'; }
  const originalFetch = window.fetch.bind(window);
  window.fetch = async (...args) => { const response = await originalFetch(...args); const requestUrl = typeof args[0] === 'string' ? args[0] : args[0]?.url || ''; const match = requestUrl.match(/\/observation\/organism\/([^/?#]+)/); if (!match) return response; try { const data = await response.clone().json(); const organism = data?.payload?.Organism; if (response.ok && organism) renderOrganism(organism); else if (response.status === 404) showError('Organism no longer exists.'); } catch (_) { showError('Could not read organism observation.'); } return response; };
})();
