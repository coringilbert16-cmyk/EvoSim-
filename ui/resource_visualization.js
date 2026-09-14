// Read-only observation-layer rendering helpers.
// Resource identity comes from the simulation-side observation endpoint.
(async function initializeResourceVisualization() {
  document.querySelector('header')?.remove();

  const paletteState = { resources: new Map(), fieldCellSize: 25 };

  function normalizePalette(payload) {
    const resources = Array.isArray(payload) ? payload : (payload.resources || []);
    for (const entry of resources) {
      if (!Array.isArray(entry) || entry.length !== 2) continue;
      paletteState.resources.set(entry[0], entry[1]);
    }
    if (!Array.isArray(payload) && Number.isFinite(payload.field_cell_size)) paletteState.fieldCellSize = payload.field_cell_size;
  }

  function appearanceFor(name) {
    return paletteState.resources.get(name) || { fill: '#AAAAAA', outline: '#666666', fill_opacity: 255 };
  }

  function dominantResource(material) {
    const parts = material?.parts || [];
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
    const cells = world.field || [];
    const maxByResource = new Map();
    for (const cell of cells) {
      for (const [name, amount] of cell.materials || []) {
        if (!Number.isFinite(amount) || amount <= 0) continue;
        maxByResource.set(name, Math.max(maxByResource.get(name) || 0, amount));
      }
    }
    for (const cell of cells) {
      const topLeft = camera.worldToScreen(cell.x - size / 2, cell.y - size / 2, viewport.width, viewport.height);
      const screenSize = size * camera.scale;
      if (screenSize <= 0) continue;
      for (const [name, amount] of cell.materials || []) {
        const maximum = maxByResource.get(name) || 0;
        if (!Number.isFinite(amount) || amount <= 0 || !(maximum > 0)) continue;
        const appearance = appearanceFor(name);
        const density = Math.max(0, Math.min(1, amount / maximum));
        const baseOpacity = Math.max(0, Math.min(1, appearance.fill_opacity / 255));
        if (!(baseOpacity > 0)) continue;
        const centerX = topLeft.x + screenSize / 2;
        const centerY = topLeft.y + screenSize / 2;
        const innerRadius = Math.max(0, screenSize * 0.08);
        const outerRadius = Math.max(innerRadius + 1, screenSize * 0.72);
        const peakAlpha = Math.min(0.82, 0.08 + density * 0.74) * baseOpacity;
        const gradient = ctx.createRadialGradient(centerX, centerY, innerRadius, centerX, centerY, outerRadius);
        gradient.addColorStop(0, `${appearance.fill}${Math.round(peakAlpha * 255).toString(16).padStart(2, '0')}`);
        gradient.addColorStop(0.45, `${appearance.fill}${Math.round(peakAlpha * 0.72 * 255).toString(16).padStart(2, '0')}`);
        gradient.addColorStop(1, `${appearance.fill}00`);
        ctx.save(); ctx.fillStyle = gradient;
        ctx.fillRect(centerX - outerRadius, centerY - outerRadius, outerRadius * 2, outerRadius * 2);
        ctx.restore();
      }
    }
  }

  function drawWorldPoints(world, viewport) {
    for (const point of world.vents || []) {
      const p = camera.worldToScreen(point.x, point.y, viewport.width, viewport.height);
      ctx.save(); ctx.strokeStyle = '#CFCFCF'; ctx.lineWidth = 1.5; ctx.beginPath();
      ctx.moveTo(p.x - 5, p.y); ctx.lineTo(p.x + 5, p.y);
      ctx.moveTo(p.x, p.y - 5); ctx.lineTo(p.x, p.y + 5); ctx.stroke(); ctx.restore();
    }
    for (const point of world.decomposing_bodies || []) {
      const p = camera.worldToScreen(point.x, point.y, viewport.width, viewport.height);
      ctx.save(); ctx.strokeStyle = '#8B8B8B'; ctx.setLineDash([3, 3]); ctx.beginPath();
      ctx.arc(p.x, p.y, 6, 0, Math.PI * 2); ctx.stroke(); ctx.restore();
    }
  }

  // World observation exposes authoritative coarse bounds, not exact forms.
  // The rectangle is only a visual projection of the observed extent.
  function drawCoarseOrganisms(world, viewport) {
    for (const organism of world?.organisms || []) {
      const a = camera.worldToScreen(organism.min_x, organism.min_y, viewport.width, viewport.height);
      const b = camera.worldToScreen(organism.max_x, organism.max_y, viewport.width, viewport.height);
      const width = Math.max(1, b.x - a.x);
      const height = Math.max(1, b.y - a.y);
      ctx.save(); ctx.fillStyle = '#777777'; ctx.globalAlpha = 0.35;
      ctx.fillRect(a.x, a.y, width, height);
      ctx.strokeStyle = '#BBBBBB'; ctx.globalAlpha = 0.9;
      ctx.strokeRect(a.x, a.y, width, height); ctx.restore();
    }
  }

  const originalRenderWorld = renderWorld;
  renderWorld = function resourceAwareWorldRender() {
    const world = worldPayload();
    const viewport = viewportSize();
    drawField(world, viewport);
    drawWorldPoints(world, viewport);
    drawCoarseOrganisms(world, viewport);
    originalRenderWorld();
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
      const point = camera.worldToScreen(unit.placement.x, unit.placement.y, viewport.width, viewport.height);
      const appearance = appearanceFor(dominantResource(unit.material));
      ctx.save(); ctx.translate(point.x, point.y); ctx.scale(camera.scale, camera.scale);
      ctx.fillStyle = appearance.fill; ctx.strokeStyle = appearance.outline; ctx.globalAlpha = appearance.fill_opacity / 255;
      drawForm(unit.form, 0, 0, unit.placement.rotation_radians); ctx.restore();
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
    if (sameTarget && lastObservationTick === tick) return;
    await originalLoadObservation(level, organismId);
    lastObservationTick = tick;
  };

  try {
    const response = await fetch('/observation/resources', { cache: 'no-store' });
    if (response.ok) {
      normalizePalette(await response.json());
      render();
    }
  } catch (_) {
    // Neutral rendering fallback is intentional.
  }
})();
