// Read-only observation-layer rendering helpers.
// Resource identity comes from the simulation-side observation endpoint.
(async function initializeResourceVisualization() {
  const paletteState = { resources: new Map(), fieldCellSize: 25 };

  // Build the simulation workspace around the existing authoritative canvas.
  const app = document.getElementById('app');
  const originalHeader = document.querySelector('header');
  const status = document.getElementById('status');
  const keyboardHelp = document.getElementById('keyboard-help');
  const topbar = document.createElement('header');
  topbar.className = 'ev-topbar';
  const brand = document.createElement('div');
  brand.className = 'ev-brand';
  brand.innerHTML = '<span class="ev-brand-mark">E</span><span><strong>EVOSIM</strong><small>SIMULATION WORKSPACE</small></span>';
  const metrics = document.createElement('div');
  metrics.className = 'ev-top-metrics';
  metrics.innerHTML = '<span><small>TICK</small><strong id="ev-top-tick">—</strong></span><span><small>POPULATION</small><strong id="ev-top-population">—</strong></span><span><small>RATE</small><strong id="ev-top-rate">—</strong></span><span class="ev-run-state" id="ev-top-state">CONNECTING</span>';
  topbar.append(brand, metrics);
  if (status) { status.className = 'ev-connection-status'; topbar.append(status); }
  if (keyboardHelp) { keyboardHelp.className = 'ev-keyboard-help'; topbar.append(keyboardHelp); }
  originalHeader?.remove();
  app?.insertBefore(topbar, app.firstChild);

  const main = document.querySelector('main');
  const canvas = document.getElementById('world');
  const worldColumn = document.createElement('section');
  worldColumn.className = 'ev-world-column';
  const worldToolbar = document.createElement('div');
  worldToolbar.className = 'ev-world-toolbar';
  worldToolbar.innerHTML = '<span>WORLD VIEW</span><span class="ev-view-hint">Drag to pan · Scroll to zoom · Select an organism to inspect</span>';
  const viewport = document.createElement('div');
  viewport.className = 'ev-world-viewport';
  canvas?.parentNode?.removeChild(canvas);
  viewport.append(canvas);
  worldColumn.append(worldToolbar, viewport);

  const sidebar = document.createElement('aside');
  sidebar.className = 'ev-sidebar';
  sidebar.innerHTML = '<div class="ev-panel-heading">OBSERVATION</div><nav class="ev-sidebar-tabs"><button type="button" class="active" data-panel="overview">Overview</button><button type="button" data-panel="materials">Materials</button></nav><div id="ev-sidebar-content"></div>';
  main?.replaceChildren(sidebar, worldColumn);
  const sidebarContent = sidebar.querySelector('#ev-sidebar-content');
  const overviewContent = document.createElement('div');
  overviewContent.className = 'ev-overview';
  overviewContent.innerHTML = '<div class="ev-section-label">LIVE WORLD</div><div class="ev-stat-grid"><div><small>Current tick</small><strong id="ev-side-tick">—</strong></div><div><small>Population</small><strong id="ev-side-population">—</strong></div><div><small>Simulation rate</small><strong id="ev-side-rate">—</strong></div><div><small>History</small><strong id="ev-side-history">—</strong></div></div><div class="ev-note">The world view shows the live simulation. Select an organism to inspect its physical structure.</div>';
  const materialsContent = document.createElement('div');
  materialsContent.className = 'ev-materials';
  materialsContent.innerHTML = '<div class="ev-section-label">RESOURCE CATALOG</div><div class="ev-material-list" id="ev-material-list"><span class="ev-empty">Loading material palette…</span></div>';
  sidebarContent?.append(overviewContent);
  sidebar.querySelectorAll('[data-panel]').forEach(button => button.addEventListener('click', () => {
    sidebar.querySelectorAll('[data-panel]').forEach(item => item.classList.toggle('active', item === button));
    sidebarContent?.replaceChildren(button.dataset.panel === 'materials' ? materialsContent : overviewContent);
  }));

  function normalizePalette(payload) {
    const resources = Array.isArray(payload) ? payload : (payload.resources || []);
    for (const entry of resources) {
      if (!Array.isArray(entry) || entry.length !== 2) continue;
      paletteState.resources.set(entry[0], entry[1]);
    }
    if (!Array.isArray(payload) && Number.isFinite(payload.field_cell_size)) paletteState.fieldCellSize = payload.field_cell_size;
  }
  renderMaterialLegend();

  function appearanceFor(name) {
    return paletteState.resources.get(name) || { fill: '#AAAAAA', outline: '#666666', fill_opacity: 255 };
  }

  function renderMaterialLegend() {
    const list = document.getElementById('ev-material-list');
    if (!list) return;
    list.replaceChildren();
    const entries = [...paletteState.resources.entries()].sort(([a], [b]) => a.localeCompare(b));
    if (!entries.length) {
      const empty = document.createElement('span');
      empty.className = 'ev-empty';
      empty.textContent = 'No material palette available';
      list.append(empty);
      return;
    }
    for (const [name, appearance] of entries) {
      const row = document.createElement('div');
      row.className = 'ev-material-row';
      const swatch = document.createElement('span');
      swatch.className = 'ev-material-swatch';
      swatch.style.background = appearance.fill || '#888';
      swatch.style.borderColor = appearance.outline || '#666';
      const label = document.createElement('span');
      label.textContent = name;
      row.append(swatch, label);
      list.append(row);
    }
  }

  async function refreshWorkspaceMetrics() {
    try {
      const response = await fetch('/observation/status', { cache: 'no-store' });
      if (!response.ok) return;
      const state = await response.json();
      const tick = String(state.tick ?? '—');
      const population = String(state.population ?? '—');
      const configuredRate = Number.isFinite(state.ticks_per_second) ? state.ticks_per_second : null;
      const measuredRate = Number.isFinite(state.measured_ticks_per_second) ? state.measured_ticks_per_second : null;
      const rate = measuredRate == null ? '—' : `${measuredRate.toFixed(1)} t/s`;
      const rateDetail = configuredRate == null ? '—' : `${configuredRate.toFixed(1)} / ${measuredRate?.toFixed(1) ?? '—'} t/s`;
      const history = String(state.history_ticks?.length ?? 0);
      for (const id of ['ev-top-tick', 'ev-side-tick']) { const node = document.getElementById(id); if (node) node.textContent = tick; }
      for (const id of ['ev-top-population', 'ev-side-population']) { const node = document.getElementById(id); if (node) node.textContent = population; }
      const topRate = document.getElementById('ev-top-rate');
      if (topRate) topRate.textContent = rate;
      const sideRate = document.getElementById('ev-side-rate');
      if (sideRate) sideRate.textContent = rateDetail;
      const historyNode = document.getElementById('ev-side-history');
      if (historyNode) historyNode.textContent = history;
      const runState = document.getElementById('ev-top-state');
      if (runState) {
        runState.textContent = state.running ? 'RUNNING' : 'PAUSED';
        runState.classList.toggle('running', Boolean(state.running));
      }
    } catch (_) {
      // The observation stream and existing status indicator handle reconnects.
    }
  }

  function dominantResource(material) {
    const parts = material?.parts || material?.materials || [];
    if (!parts.length) return null;
    const totals = new Map();
    for (const [name, amount] of parts) {
      if (!Number.isFinite(amount) || amount <= 0) continue;
      totals.set(name, (totals.get(name) || 0) + amount);
    }
    let best = null;
    for (const [name, amount] of totals) if (!best || amount > best.amount) best = { name, amount };
    return best?.name || null;
  }

  function drawField(world, viewport) {
    if (!world) return;
    const size = paletteState.fieldCellSize;
    for (const cell of world.field || []) {
      const name = dominantResource(cell);
      if (!name) continue;
      const appearance = appearanceFor(name);
      const screen = camera.worldToScreen(cell.x, cell.y, viewport.width, viewport.height);
      const screenSize = size * camera.scale;
      if (screen.x < -screenSize || screen.y < -screenSize ||
          screen.x > viewport.width + screenSize || screen.y > viewport.height + screenSize) continue;
      ctx.save();
      ctx.fillStyle = appearance.fill;
      ctx.globalAlpha = Math.max(0, Math.min(1, appearance.fill_opacity / 255));
      if (screenSize < 3) {
        ctx.beginPath();
        ctx.arc(screen.x, screen.y, Math.max(0.75, screenSize * 0.35), 0, Math.PI * 2);
        ctx.fill();
      } else {
        ctx.fillRect(screen.x - screenSize / 2, screen.y - screenSize / 2, screenSize, screenSize);
      }
      ctx.restore();
    }
  }

  function drawWorldPoints(world, viewport) {
    for (const point of world.decomposing_bodies || []) {
      const p = camera.worldToScreen(point.x, point.y, viewport.width, viewport.height);
      if (p.x < -8 || p.y < -8 || p.x > viewport.width + 8 || p.y > viewport.height + 8) continue;
      ctx.save(); ctx.strokeStyle = '#8B8B8B'; ctx.setLineDash([3, 3]); ctx.beginPath();
      ctx.arc(p.x, p.y, 6, 0, Math.PI * 2); ctx.stroke(); ctx.restore();
    }
  }


  renderWorld = function resourceAwareWorldRender() {
    const world = worldPayload();
    const viewport = viewportSize();
    if (!world) return;
    drawField(world, viewport);
    renderHotspot(world.hotspot, viewport);
    drawWorldPoints(world, viewport);
    for (const organism of world.organisms || []) {
      const physical = organism.physical;
      if (!physical) continue;
      for (const part of physical.silhouette || []) {
        const bounds = formBounds(part.form, part.x, part.y, part.rotation_radians);
        const point = camera.worldToScreen(part.x, part.y, viewport.width, viewport.height);
        if (bounds) {
          const a = camera.worldToScreen(bounds.minX, bounds.minY, viewport.width, viewport.height);
          const b = camera.worldToScreen(bounds.maxX, bounds.maxY, viewport.width, viewport.height);
          if (b.x < 0 || b.y < 0 || a.x > viewport.width || a.y > viewport.height) continue;
          const projectedSize = Math.max(bounds.maxX - bounds.minX, bounds.maxY - bounds.minY) * camera.scale;
          if (projectedSize < 1.5) {
            ctx.save();
            ctx.fillStyle = '#d9e2ec';
            ctx.beginPath();
            ctx.arc(point.x, point.y, Math.max(0.7, projectedSize * 0.5), 0, Math.PI * 2);
            ctx.fill();
            ctx.restore();
            continue;
          }
        }
        drawObservedPart(part, viewport);
      }
      drawSelectionBounds(organism.id, physical, viewport);
    }
  };

  renderStructure = function resourceAwareStructureRender() {
    const structure = structurePayload();
    if (!structure) return;
    const viewport = viewportSize();
    ctx.save();
    for (const bond of structure.bonds || []) {
      if (!bond.endpoint_a || !bond.endpoint_b) continue;
      const a = camera.worldToScreen(bond.endpoint_a.x, bond.endpoint_a.y, viewport.width, viewport.height);
      const b = camera.worldToScreen(bond.endpoint_b.x, bond.endpoint_b.y, viewport.width, viewport.height);
      ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
    }
    ctx.restore();
    for (const unit of structure.units || []) {
      if (!unit.form || !unit.placement) continue;
      const rotation = unit.placement.rotation_radians || 0;
      const bounds = formBounds(unit.form, unit.placement.x, unit.placement.y, rotation);
      const point = camera.worldToScreen(unit.placement.x, unit.placement.y, viewport.width, viewport.height);
      if (bounds) {
        const a = camera.worldToScreen(bounds.minX, bounds.minY, viewport.width, viewport.height);
        const b = camera.worldToScreen(bounds.maxX, bounds.maxY, viewport.width, viewport.height);
        if (b.x < 0 || b.y < 0 || a.x > viewport.width || a.y > viewport.height) continue;
        const projectedSize = Math.max(bounds.maxX - bounds.minX, bounds.maxY - bounds.minY) * camera.scale;
        if (projectedSize < 1.5) {
          ctx.save();
          ctx.fillStyle = appearanceFor(dominantResource(unit.material)).fill;
          ctx.globalAlpha = 0.9;
          ctx.beginPath();
          ctx.arc(point.x, point.y, Math.max(0.7, projectedSize * 0.5), 0, Math.PI * 2);
          ctx.fill();
          ctx.restore();
          continue;
        }
      }
      const appearance = appearanceFor(dominantResource(unit.material));
      ctx.save(); ctx.translate(point.x, point.y); ctx.scale(camera.scale, camera.scale);
      ctx.fillStyle = appearance.fill; ctx.strokeStyle = appearance.outline; ctx.globalAlpha = appearance.fill_opacity / 255;
      drawForm(unit.form, 0, 0, rotation); ctx.restore();
    }
  };

  const originalFinishWorldSelection = finishWorldSelection;
  finishWorldSelection = function normalizedWorldSelection(point) {
    originalFinishWorldSelection(point);
    const validFocus = selectedIds.size === 1 && selectedIds.has(focusedOrganismId);
    if (!validFocus && focusedOrganismId !== null) {
      focusedOrganismId = selectedIds.size === 1 ? [...selectedIds][0] : null;
      render();
    }
  };

  const originalLoadObservation = loadObservation;
  let lastObservationTick = null;
  let lastObservationViewKey = null;
  let statusRequestInFlight = false;
  loadObservation = async function demandDrivenObservation(level, organismId = null) {
    if (statusRequestInFlight) return;
    statusRequestInFlight = true;
    let tick = null;
    try {
      const response = await fetch('/observation/status', { cache: 'no-store' });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      tick = (await response.json()).tick;
    } catch (_) {
      statusRequestInFlight = false;
      return originalLoadObservation(level, organismId);
    }
    statusRequestInFlight = false;
    const currentPayload = observation?.payload || {};
    const currentLevel = observation?.context?.level;
    const currentObject = currentPayload[level];
    const sameTarget = currentLevel === level && (level === 'World' || (currentObject?.id != null && String(currentObject.id) === String(organismId)));
    const viewKey = typeof observationViewKey === 'function' ? observationViewKey() : 'full';
    if (sameTarget && lastObservationTick === tick && lastObservationViewKey === viewKey) return;
    await originalLoadObservation(level, organismId);
    lastObservationTick = tick;
    lastObservationViewKey = viewKey;
  };

  try {
    const response = await fetch('/observation/resources', { cache: 'no-store' });
    if (response.ok) {
      normalizePalette(await response.json());
      renderMaterialLegend();
      render();
    }
  } catch (_) {
    // Neutral rendering fallback is intentional.
  }
  refreshWorkspaceMetrics();
  setInterval(refreshWorkspaceMetrics, 1000);
})();
