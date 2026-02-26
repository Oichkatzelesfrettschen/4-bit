# Noise Parameters: pMOS Flicker Noise

Noise characterization parameters for pMOS transistors of the era.

Primary reference: klaassen1971noise

## 1/f (Flicker) Noise Model

S_id(f) = Kf * Id^Af / (f * Cox * L^2)

| Parameter | Value | Uncertainty | Source |
|-----------|-------|-------------|--------|
| Kf (flicker noise coefficient) | ~1e-24 V2*F | order of magnitude | klaassen1971noise |
| Af (current exponent) | ~1.0 | typical | klaassen1971noise |

## Thermal Noise

S_id = 4*k*T*gm*(2/3) for long-channel MOSFET in saturation

At 300K with gm ~ 1e-4 A/V (typical for 10um/10um pMOS):
- S_id_thermal ~ 1.1e-24 A2/Hz

## Noise Margin Analysis

For the 4004 at 740 kHz:
- 1/f noise dominates below ~100 kHz
- At clock frequency, both 1/f and thermal contribute
- Logic noise margins (VOH-VIH, VIL-VOL) are large (~2-5V) for -15V supply
- Noise is not a practical concern for digital operation at these margins

## Relevance to Emulator

- Noise parameters are included for completeness and materials science accuracy
- The behavioral/gate-level emulator does not model noise
- Noise becomes relevant only for transistor-level analog simulation
  (e.g., sense amplifier margins in RAM cells, clock jitter analysis)
