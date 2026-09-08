// Browser-only camera transform. This module owns view state, never simulation state.

class Camera {
  constructor() {
    this.x = 0;
    this.y = 0;
    this.scale = 1;
  }

  set(x, y, scale) {
    if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(scale) || scale <= 0) {
      return;
    }
    this.x = x;
    this.y = y;
    this.scale = scale;
  }

  fitWorld(worldWidth, worldHeight, viewportWidth, viewportHeight, padding = 0.92) {
    if (![worldWidth, worldHeight, viewportWidth, viewportHeight, padding].every(Number.isFinite)) {
      return;
    }
    if (worldWidth <= 0 || worldHeight <= 0 || viewportWidth <= 0 || viewportHeight <= 0 || padding <= 0) {
      return;
    }

    this.x = worldWidth / 2;
    this.y = worldHeight / 2;
    this.scale = Math.min(viewportWidth / worldWidth, viewportHeight / worldHeight) * padding;
  }

  worldToScreen(x, y, viewportWidth, viewportHeight) {
    return {
      x: (x - this.x) * this.scale + viewportWidth / 2,
      y: (y - this.y) * this.scale + viewportHeight / 2,
    };
  }

  screenToWorld(x, y, viewportWidth, viewportHeight) {
    return {
      x: (x - viewportWidth / 2) / this.scale + this.x,
      y: (y - viewportHeight / 2) / this.scale + this.y,
    };
  }

  pan(screenDeltaX, screenDeltaY) {
    if (!Number.isFinite(screenDeltaX) || !Number.isFinite(screenDeltaY)) {
      return;
    }
    this.x -= screenDeltaX / this.scale;
    this.y -= screenDeltaY / this.scale;
  }

  zoomAt(screenX, screenY, factor, viewportWidth, viewportHeight) {
    if (!Number.isFinite(factor) || factor <= 0 || !Number.isFinite(screenX) || !Number.isFinite(screenY)) {
      return;
    }

    const before = this.screenToWorld(screenX, screenY, viewportWidth, viewportHeight);
    this.scale *= factor;
    const after = this.screenToWorld(screenX, screenY, viewportWidth, viewportHeight);
    this.x += before.x - after.x;
    this.y += before.y - after.y;
  }
}

window.EvoSimCamera = Camera;
