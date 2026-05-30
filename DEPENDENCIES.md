# DEPENDENCIES — plato-coordination

## Signal Chain Layer

**L3 (Fleet)** — Cross-room fleet coordination.

Coordinates rooms across the fleet. Handles delegation, conflict resolution, and cross-room state aggregation for the PLATO nervous system.

## Ecosystem Dependencies

| Repo | Relationship | Description |
|------|-------------|-------------|
| [plato-state](https://github.com/SuperInstance/plato-state) | **Depends on** | Room state vectors for cross-room decision making |
| [plato-tiles](https://github.com/SuperInstance/plato-tiles) | **Depends on** | Tile types for inter-room communication |
| [plato-nervous](https://github.com/SuperInstance/plato-nervous) | **Related** | L3 fleet coordination is part of the signal chain |
| [plato-signal-chain](https://github.com/SuperInstance/plato-signal-chain) | **Related** | Signal chain includes the L3 coordination layer |
| [plato-dashboard](https://github.com/SuperInstance/plato-dashboard) | **Depended on by** | Dashboard renders fleet coordination status |

## Data Flow

```
IN:
  - Room state vectors (from plato-state)
  - Tiles requiring cross-room resolution (from plato-tiles)
  - Fleet topology and room capabilities

OUT:
  - Coordination decisions (delegation assignments)
  - Cross-room state aggregation
  - Fleet-level summary tiles
  - Conflict resolution outcomes
```

## Dependency Graph Position

```
plato-tiles → plato-rooms → plato-state
                              ↓
                    plato-coordination ← (this crate)
```
