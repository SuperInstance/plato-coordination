# plato-coordination

Cross-room fleet coordination for PLATO nervous system.

## What It Does

Handles coordination across the fleet of PLATO rooms. When a situation spans multiple rooms or requires delegation, this crate manages cross-room communication, conflict resolution, and state aggregation.

## Ecosystem

- **[plato-state](https://github.com/SuperInstance/plato-state)** ← Depends on (room state vectors)
- **[plato-tiles](https://github.com/SuperInstance/plato-tiles)** ← Depends on (tile types for communication)
- **[plato-nervous](https://github.com/SuperInstance/plato-nervous)** — L3 coordination layer in the signal chain
- **[plato-signal-chain](https://github.com/SuperInstance/plato-signal-chain)** — Pipeline includes L3 fleet coordination
- **[plato-dashboard](https://github.com/SuperInstance/plato-dashboard)** → Renders fleet status

See [DEPENDENCIES.md](./DEPENDENCIES.md) for the full dependency map.
