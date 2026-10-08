# Bob

Bob is EvoSim's persistent geometry knowledge engine.

- `library.rs` owns durable formations, contact families, canonicalization, and indexed runtime lookup.
- `worker.rs` expands and persists geometry knowledge.
- `server.rs` exposes the read-only geometry viewer.

Bob is knowledge, not live simulation state. Runtime code should query Bob and then perform authoritative physical validation rather than rediscovering geometry.
