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
  // The workspace CSS relies on this flex container class. Without it the
  // viewport can collapse to zero height after the canvas is reparented.
  main?.classList.add('ev-main');
  const canvas = document.getElementById('world');
  const worldColumn = document.createElement('section');
  worldColumn.className = 'ev-world-column';
  const worldToolbar = document.createElement('div');
  worldToolbar.className = 'ev-world-toolbar';
  worldToolbar.innerHTML = '<span>WORLD VIEW</span><span class="ev-view-hint">Drag to pan · Scroll to zoom · Select an organism to inspect</span>';
  const cameraControls = document.createElement('span');
  cameraControls.className = 'ev-camera-controls';
  cameraControls.innerHTML = '<button type="button" data-zoom="out" aria-label="Zoom out" title="Zoom out">−</button><button type="button" data-zoom="in" aria-label="Zoom in" title="Zoom in">+</button><button type="button" data-zoom="fit" aria-label="Fit world" title="Fit current view">Fit</button>';
  worldToolbar.append(cameraControls);
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
  overviewContent.innerHTML = '<div class="ev-section-label">LIVE WORLD</div><div class="ev-stat-grid"><div><small>Current tick</small><strong id="ev-side-tick">—</strong></div><div><small>Population</small><strong id="ev-side-population">—</strong></div><div><small>Simulation rate</small><strong id="ev-side-rate">—</strong></div><div><small>History</small><strong id="ev-side-history">—</strong></div></div><div class="ev-section-label">CHEMISTRY ACTIVITY</div><div class="ev-stat-grid"><div><small>COMBINE committed</small><strong id="ev-combine-count">0</strong></div><div><small>BREAK committed</small><strong id="ev-break-count">0</strong></div></div><div class="ev-note" id="ev-last-reaction">Waiting for chemistry events…</div><div class="ev-note">Select an organism to inspect its physical structure.</div>';
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
    const heading = document.createElement('div');
    heading.className = 'ev-note';
    heading.textContent = 'Element color is calculated from its natural resonant frequency. Aura strength follows the oscillator response amplitude.';
    const scale = document.createElement('div');
    scale.style.height = '14px';
    scale.style.borderRadius = '999px';
    scale.style.margin = '12px 0 6px';
    scale.style.background = 'linear-gradient(90deg, hsl(200 90% 62%), hsl(240 90% 62%), hsl(280 90% 62%), hsl(320 90% 62%))';
    const labels = document.createElement('div');
    labels.className = 'ev-material-row';
    labels.innerHTML = '<span>Lower natural frequency</span><span>Higher natural frequency</span>';
    list.append(heading, scale, labels);
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

  function resonanceAppearance(part) {
    const frequency = Number.isFinite(part.resonance_frequency_cycles_per_day)
      ? Math.max(1, part.resonance_frequency_cycles_per_day) : 440;
    const amplitude = Number.isFinite(part.resonance_amplitude)
      ? Math.max(0, Math.min(1, part.resonance_amplitude)) : 0;
    // Hue represents the material's natural resonant frequency; amplitude
    // controls brightness and aura strength. Phase contributes a small hue shift.
    const normalizedFrequency = Math.max(0, Math.min(1,
      (Math.log(frequency) - Math.log(100)) / (Math.log(1000) - Math.log(100))));
    const hue = 200 + normalizedFrequency * 120;
    const lightness = 42 + amplitude * 24;
    return {
      fill: `hsl(${hue} 88% ${lightness}%)`,
      outline: `hsl(${hue} 95% ${Math.min(88, lightness + 18)}%)`,
      amplitude,
      hue
    };
  }

  function drawPhysicalFieldPart(part, viewport) {
    const appearance = resonanceAppearance(part);
    const displayPosition = window.__evosimGetDisplayPosition?.(part) || { x: part.x, y: part.y };
    const point = camera.worldToScreen(displayPosition.x, displayPosition.y, viewport.width, viewport.height);
    const fluid = part.form?.Fluid;
    const localBounds = formBounds(part.form, 0, 0, part.rotation_radians);
    const radius = localBounds
      ? Math.max(Math.abs(localBounds.minX), Math.abs(localBounds.maxX), Math.abs(localBounds.minY), Math.abs(localBounds.maxY))
      : (fluid && Number.isFinite(fluid.nominal_area) ? Math.sqrt(fluid.nominal_area / Math.PI) : 0.5);
    const screenRadius = Math.max(1, radius * camera.scale);
    const auraRadius = Math.max(7, Math.min(30, screenRadius * 5));
    if (point.x < -auraRadius || point.y < -auraRadius ||
        point.x > viewport.width + auraRadius || point.y > viewport.height + auraRadius) return;

    // The aura is a read-only visualization of the oscillator response.
    // It is deliberately drawn before the physical form and never feeds back
    // into simulation state, energy, or movement.
    if (appearance.amplitude > 0) {
      const alpha = Math.min(0.34, 0.10 + appearance.amplitude * 0.24);
      ctx.save();
      const gradient = ctx.createRadialGradient(point.x, point.y, 0, point.x, point.y, auraRadius);
      gradient.addColorStop(0, `hsla(${appearance.hue} 92% 68% / ${alpha})`);
      gradient.addColorStop(0.42, `hsla(${appearance.hue} 92% 62% / ${alpha * 0.52})`);
      gradient.addColorStop(1, `hsla(${appearance.hue} 92% 58% / 0)`);
      ctx.fillStyle = gradient;
      ctx.beginPath();
      ctx.arc(point.x, point.y, auraRadius, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    ctx.save();
    ctx.translate(point.x, point.y);
    if (radius * camera.scale < 2.25) {
      ctx.fillStyle = appearance.fill;
      ctx.globalAlpha = 0.78 + appearance.amplitude * 0.22;
      ctx.beginPath();
      ctx.arc(0, 0, 1.25, 0, Math.PI * 2);
      ctx.fill();
    } else {
      ctx.scale(camera.scale, camera.scale);
      ctx.fillStyle = appearance.fill;
      ctx.strokeStyle = appearance.outline;
      ctx.globalAlpha = 0.78 + appearance.amplitude * 0.22;
      if (fluid) {
        ctx.beginPath();
        ctx.arc(0, 0, Math.sqrt(Math.max(fluid.nominal_area, 0) / Math.PI), 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
      } else {
        drawForm(part.form, 0, 0, part.rotation_radians, appearance);
      }
    }
    ctx.restore();
  }

  function drawField(world, viewport) {
    if (!world) return;
    for (const cell of world.field || []) {
      const physicalParts = cell.physical_parts || [];
      for (const part of physicalParts) drawPhysicalFieldPart(part, viewport);

      // Unstructured inventory has no constituent geometry or world-space
      // placement. Show it as a small cell marker, never as fake square atoms.
      if (!physicalParts.length && cell.materials?.length) {
        const name = dominantResource(cell);
        if (!name) continue;
        const appearance = appearanceFor(name);
        const point = camera.worldToScreen(cell.x, cell.y, viewport.width, viewport.height);
        if (point.x < -4 || point.y < -4 || point.x > viewport.width + 4 || point.y > viewport.height + 4) continue;
        ctx.save();
        ctx.fillStyle = appearance.fill;
        ctx.globalAlpha = 0.7;
        ctx.beginPath();
        ctx.arc(point.x, point.y, 2, 0, Math.PI * 2);
        ctx.fill();
        ctx.restore();
      }
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


  function updateChemistryReadout(world) {
    const events = Array.isArray(world?.chemistry_events) ? world.chemistry_events : [];
    const committed = events.filter(event => event.outcome === 'committed');
    const combine = committed.filter(event => event.reaction === 'COMBINE').length;
    const breaking = committed.filter(event => event.reaction === 'BREAK').length;
    const combineNode = document.getElementById('ev-combine-count');
    const breakNode = document.getElementById('ev-break-count');
    const lastNode = document.getElementById('ev-last-reaction');
    if (combineNode) combineNode.textContent = String(combine);
    if (breakNode) breakNode.textContent = String(breaking);
    if (lastNode) {
      const latest = committed[committed.length - 1];
      lastNode.textContent = latest
        ? `Last committed reaction: ${latest.reaction} at tick ${latest.tick}`
        : 'No committed environmental reactions recorded yet';
    }
  }

  renderWorld = function resourceAwareWorldRender() {
    const world = worldPayload();
    const viewport = viewportSize();
    if (!world) return;
    updateChemistryReadout(world);
    // Clip every world layer to the exact field boundary.
    const topLeft = camera.worldToScreen(0, 0, viewport.width, viewport.height);
    const bottomRight = camera.worldToScreen(world.width, world.height, viewport.width, viewport.height);
    ctx.save();
    ctx.beginPath();
    ctx.rect(topLeft.x, topLeft.y, bottomRight.x - topLeft.x, bottomRight.y - topLeft.y);
    ctx.clip();
    renderWaterField(world, viewport);
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
    ctx.restore();
    ctx.save();
    ctx.strokeStyle = 'rgba(186, 230, 253, 0.9)';
    ctx.lineWidth = 2;
    ctx.strokeRect(topLeft.x, topLeft.y, bottomRight.x - topLeft.x, bottomRight.y - topLeft.y);
    ctx.restore();
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

  cameraControls.querySelector('[data-zoom="out"]').addEventListener('click', () => {
    const size = viewportSize();
    camera.zoomAt(size.width / 2, size.height / 2, 1 / 1.5, size.width, size.height);
    render();
  });
  cameraControls.querySelector('[data-zoom="in"]').addEventListener('click', () => {
    const size = viewportSize();
    camera.zoomAt(size.width / 2, size.height / 2, 1.5, size.width, size.height);
    render();
  });
  cameraControls.querySelector('[data-zoom="fit"]').addEventListener('click', () => {
    cameraInitialized = false;
    fitCurrentObservation();
    render();
  });

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
