# Process Parameters v1: Intel 10um pMOS Silicon-Gate Technology

Extracted process parameters with citations for the MCS-4/MCS-40 chip families.
Each parameter includes source, confidence level, and uncertainty bounds.

## Gate Oxide

| Parameter | Value | Uncertainty | Confidence | Source |
|-----------|-------|-------------|------------|--------|
| tox | 80 nm | +/-20 nm (50-100 nm range) | MEDIUM | faggin1970sgt, osburn2002tox |
| epsilon_ox | 3.9 * epsilon_0 | exact | HIGH | fundamental constant |
| Cox | ~4.3e-4 F/m2 | derived from tox | MEDIUM | calculated |

## Threshold Voltages

| Parameter | Value | Uncertainty | Confidence | Source |
|-----------|-------|-------------|------------|--------|
| Vth_enh | -3.0 V | +/-1V (-2 to -4V) | MEDIUM | intel4004ds (VOH/VOL), deal1974oxidecharges |
| Vth_dep | +1.0 V | +/-0.5V | LOW | inferred from circuit topology |

## Mobility

| Parameter | Value | Uncertainty | Confidence | Source |
|-----------|-------|-------------|------------|--------|
| mu_0 (holes) | 175 cm2/Vs | +/-50 | MEDIUM | hofstein1965frequency, sze1981semiconductor |
| theta | 0.03 V-1 | estimated | LOW | typical for era |

## Substrate

| Parameter | Value | Uncertainty | Confidence | Source |
|-----------|-------|-------------|------------|--------|
| resistivity | ~10 ohm-cm | +/-5 | MEDIUM | inferred from Intel process docs |
| N_sub | ~4.5e14 cm-3 | derived | MEDIUM | from resistivity |
| orientation | (111) or (100) | unknown | LOW | faggin2015memoirs suggests (111) for early SGT |

## Supply

| Parameter | Value | Notes | Confidence | Source |
|-----------|-------|-------|------------|--------|
| VDD | -15V | standard | HIGH | intel4004ds |
| VDD | -10V | later variant | HIGH | intel4004ds |
| VSS | 0V | ground | HIGH | intel4004ds |

## Geometry

| Parameter | Value | Uncertainty | Confidence | Source |
|-----------|-------|-------------|------------|--------|
| L_min | 10 um | exact | HIGH | multiple primary sources |
| x_j | ~2 um | estimated | LOW | typical for era |

## Notes

- All parameters are best estimates from published literature.
- Parameters feed the Rust process model in mcs4-core/src/process/.
- Confidence levels: HIGH = primary source quote, MEDIUM = derived/cross-referenced,
  LOW = estimated from typical values for the technology node.
- See bibliography.bib for full citation details.
