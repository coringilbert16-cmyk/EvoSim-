# Persistent Library Checkpointing

EvoSim keeps Bob's Geometry Reference Library and the Chemistry Library as local
durable knowledge stores. The libraries do not depend on Git for correctness.

When a normal EvoSim process is running, a background checkpoint publisher
checks the two persistent data directories every 120 seconds by default:

- `geometry_library/data/`
- `chemistry_library/data/`

When either store changes, the publisher creates a Git commit containing the
current durable library state and pushes the current branch to `origin`.

## Important behavior

- Library persistence succeeds even when Git or the network is unavailable.
- Git failures are retried on later checkpoints.
- The publisher uses a temporary Git index, so it does not stage the user's
  ordinary source-code changes.
- The publisher never pulls, merges, rebases, or force-pushes.
- If the local branch has diverged from the remote, the checkpoint remains
  local until the normal repository state is reconciled.
- A clean shutdown is not required for durability; the library files are
  already persisted independently of Git.

## Commands

Run one checkpoint immediately:

```text
cargo run -- --library-sync-once
```

Run only the foreground checkpoint daemon:

```text
cargo run -- --library-sync
```

Normal EvoSim commands start the background publisher automatically.

## Configuration

Change the polling interval:

```text
EVOSIM_LIBRARY_SYNC_INTERVAL_SECS=300
```

Disable automatic publishing for a process:

```text
EVOSIM_LIBRARY_SYNC_DISABLE=1
```

The interval defaults to 120 seconds so rapid library growth is batched rather
than producing a Git push for every individual discovery.

## Repository size

The library data is intentionally plain, durable repository data so its exact
history is inspectable. As the catalogue grows, its size must be monitored.
GitHub currently recommends keeping repositories around or below 10 GB on disk
and recommends Git LFS for large individual files. If a library data file
approaches those limits, the persistence format should be sharded before the
checkpoint system is changed to a different storage backend.
