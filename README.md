# tsi

A CLI tool for optimizing rocket staging configurations.

Given payload mass, target delta-v, and available engines, `tsi` finds the optimal staging solution that maximizes payload fraction or minimizes total mass.

## Installation

```bash
cargo install tsi
```

## Usage

```bash
# Calculate single-stage performance
tsi calculate --isp 311 --mass-ratio 3.5

# Optimize multi-stage rocket
tsi optimize --payload 5000 --target-dv 9400 --engines merlin-1d,raptor-2

# List available engines
tsi engines
```

## License

MIT
