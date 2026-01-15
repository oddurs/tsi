# Physics Reference

This document explains the physics calculations used in `tsi`.

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| g₀ | 9.80665 m/s² | Standard gravity |

## Tsiolkovsky Rocket Equation

The fundamental equation of rocketry, derived by Konstantin Tsiolkovsky in 1903:

```
Δv = Isp × g₀ × ln(m₀/m₁)
```

Where:
- **Δv** = change in velocity (m/s)
- **Isp** = specific impulse (seconds)
- **g₀** = standard gravity (9.80665 m/s²)
- **m₀** = initial (wet) mass
- **m₁** = final (dry) mass
- **m₀/m₁** = mass ratio

### Inverse Form

To find the required mass ratio for a given delta-v:

```
mass_ratio = e^(Δv / (Isp × g₀))
```

## Specific Impulse (Isp)

Specific impulse measures engine efficiency - how much thrust is produced per unit of propellant consumed per second.

```
Isp = v_e / g₀
```

Where **v_e** is the effective exhaust velocity.

Higher Isp means more delta-v from the same amount of propellant.

### Typical Values

| Propellant | Isp Range (vacuum) |
|------------|-------------------|
| LOX/RP-1 | 300-350s |
| LOX/CH4 | 340-380s |
| LOX/LH2 | 420-460s |
| Solid | 250-290s |

## Mass Ratio

```
mass_ratio = m_wet / m_dry
         = (m_dry + m_propellant) / m_dry
```

### Structural Ratio

The structural ratio relates structural mass to propellant mass:

```
ε = m_structural / m_propellant
```

Typical values: 0.05-0.15 (5-15%)

Lower structural ratios are better but harder to achieve.

## Thrust-to-Weight Ratio (TWR)

```
TWR = F / (m × g)
```

Where:
- **F** = thrust (Newtons)
- **m** = mass (kg)
- **g** = local gravity (m/s²)

### Requirements

- **TWR > 1.0**: Required to lift off from Earth
- **TWR ≈ 1.2-1.5**: Typical for first stages (balance of thrust and efficiency)
- **TWR < 1.0**: Acceptable for upper stages (already moving)

## Burn Time

Time to consume all propellant:

```
t = m_propellant / ṁ
```

Where mass flow rate is:

```
ṁ = F / (Isp × g₀)
```

Combined:

```
t = (m_propellant × Isp × g₀) / F
```

## Stage Performance Calculations

### Single Stage

Given an engine and propellant mass:

1. Calculate structural mass: `m_struct = m_prop × ε`
2. Calculate engine mass: `m_engine = m_engine_dry × n_engines`
3. Calculate dry mass: `m_dry = m_struct + m_engine`
4. Calculate wet mass: `m_wet = m_dry + m_prop`
5. Calculate mass ratio: `R = m_wet / m_dry`
6. Calculate delta-v: `Δv = Isp × g₀ × ln(R)`

### With Payload

When carrying payload on top of the stage:

```
m_wet_total = m_wet_stage + m_payload
m_dry_total = m_dry_stage + m_payload
R = m_wet_total / m_dry_total
Δv = Isp × g₀ × ln(R)
```

The payload "eats into" the mass ratio, reducing delta-v.

## Multi-Stage Rockets

Total delta-v is the sum of each stage's delta-v:

```
Δv_total = Δv₁ + Δv₂ + Δv₃ + ...
```

Each stage is calculated independently, with the upper stages treated as payload for lower stages.

### Why Stage?

Single-stage-to-orbit (SSTO) is theoretically possible but impractical because:

1. You carry empty tanks the whole way
2. Mass ratio requirements are extreme
3. No existing materials can achieve the needed structural ratios

Staging lets you discard empty mass, dramatically improving overall performance.

### Optimal Staging

For stages with the same Isp and structural ratio, optimal performance comes from equal delta-v contribution per stage:

```
Δv_per_stage = Δv_total / n_stages
```

This is the basis for analytical staging optimization.

## Limitations

`tsi` uses ideal rocket equation calculations. Real rockets face additional losses:

| Loss | Typical Magnitude |
|------|------------------|
| Gravity losses | 1,000-1,500 m/s |
| Atmospheric drag | 100-400 m/s |
| Steering losses | 50-150 m/s |

These losses are not currently modeled in `tsi`. The calculated delta-v represents the theoretical maximum; actual orbital insertion requires 15-20% more.

## References

- Sutton, G.P. & Biblarz, O. (2016). *Rocket Propulsion Elements*
- Turner, M.J.L. (2009). *Rocket and Spacecraft Propulsion*
- NASA SP-8012: *Staging Optimization*
