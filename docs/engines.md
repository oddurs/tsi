# Engine Database

`tsi` includes a database of real rocket engines with accurate performance data. The database is embedded in the binary, so no external files are needed.

## Available Engines

| Engine | Propellant | Thrust (vac) | Isp (vac) | Mass | Notes |
|--------|------------|--------------|-----------|------|-------|
| Merlin-1D | LOX/RP-1 | 914 kN | 311s | 470 kg | SpaceX Falcon 9 first stage |
| Merlin-Vacuum | LOX/RP-1 | 981 kN | 348s | 470 kg | SpaceX Falcon 9 second stage |
| Raptor-2 | LOX/CH4 | 2,450 kN | 350s | 1,600 kg | SpaceX Starship |
| Raptor-Vacuum | LOX/CH4 | 2,550 kN | 380s | 1,600 kg | SpaceX Starship upper stage |
| RS-25 | LOX/LH2 | 2,279 kN | 452s | 3,527 kg | Space Shuttle / SLS |
| RL-10C | LOX/LH2 | 106 kN | 453s | 190 kg | Centaur upper stage |
| F-1 | LOX/RP-1 | 7,770 kN | 304s | 8,400 kg | Saturn V first stage |
| J-2 | LOX/LH2 | 1,033 kN | 421s | 1,788 kg | Saturn V upper stages |
| RD-180 | LOX/RP-1 | 4,152 kN | 338s | 5,480 kg | Atlas V |
| BE-4 | LOX/CH4 | 2,600 kN | 340s | 2,000 kg | Blue Origin New Glenn |
| Rutherford | LOX/RP-1 | 26 kN | 343s | 35 kg | Rocket Lab Electron |

## Propellant Types

### LOX/RP-1 (Kerosene)

- Dense, storable at room temperature
- Lower Isp than hydrogen (~300-340s)
- Used in first stages due to high density

**Engines:** Merlin-1D, Merlin-Vacuum, F-1, RD-180, Rutherford

### LOX/LH2 (Liquid Hydrogen)

- Highest Isp (~420-460s)
- Very low density, requires large tanks
- Difficult to store (cryogenic)
- Best for upper stages

**Engines:** RS-25, RL-10C, J-2

### LOX/CH4 (Methane)

- Balance of density and Isp (~340-380s)
- Easier to handle than hydrogen
- Can be produced on Mars (ISRU)

**Engines:** Raptor-2, Raptor-Vacuum, BE-4

## Sea-Level vs Vacuum Performance

Rocket engines perform differently at sea level vs vacuum:

- **Sea-level:** Atmospheric pressure reduces effective exhaust velocity
- **Vacuum:** Full expansion of exhaust gases, higher Isp

Upper-stage engines (like RL-10C, Merlin-Vacuum, Raptor-Vacuum) are optimized for vacuum and have no sea-level values (shown as `-` in verbose output).

View both values with:

```bash
tsi engines --verbose
```

## Engine Selection Guidelines

### First Stage

- High thrust-to-weight ratio (TWR > 1.2)
- Sea-level capable
- Dense propellant preferred (smaller tanks)

**Good choices:** Merlin-1D, Raptor-2, RD-180, F-1

### Upper Stage

- High Isp is critical
- TWR can be lower (starting from velocity)
- Vacuum-optimized nozzle

**Good choices:** RL-10C, Merlin-Vacuum, Raptor-Vacuum, RS-25

## Adding Custom Engines

Currently, `tsi` uses the embedded engine database. Custom engine support via `--engines-file` is planned for a future release.

For now, you can use manual parameters:

```bash
tsi calculate --isp 350 --thrust 2500000 --propellant-mass 100000
```

## Data Sources

Engine data is compiled from:
- Official manufacturer specifications
- NASA technical documents
- Public launch vehicle user guides

Values represent typical or nominal performance. Actual performance may vary based on throttle setting, altitude, and specific vehicle integration.
