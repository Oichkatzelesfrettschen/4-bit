# Interconnect Parameters: Intel 10um pMOS Process

Aluminum metallization and polysilicon interconnect parameters.

## Aluminum Metallization (Single Layer)

| Parameter | Value | Uncertainty | Source |
|-----------|-------|-------------|--------|
| Sheet resistance (Rs_Al) | 0.03-0.05 ohm/sq | typical | sinha1973aluminum |
| Thickness | ~1 um | estimated | typical for era |
| Width (minimum) | ~10 um | from L_min | process constraint |
| Capacitance to substrate | ~0.03-0.05 fF/um2 | estimated | tummala1989packaging |

## Polysilicon Interconnect

| Parameter | Value | Uncertainty | Source |
|-----------|-------|-------------|--------|
| Sheet resistance (Rs_poly) | 20-50 ohm/sq | estimated | faggin1970sgt |
| Thickness | ~0.4-0.5 um | estimated | typical for SGT |
| Gate capacitance | Cox * W * L | calculated | oxide.rs |

## Contact Resistance

| Parameter | Value | Uncertainty | Source |
|-----------|-------|-------------|--------|
| Al-to-Si (ohmic) | 10-100 ohm | high uncertainty | sinha1973aluminum |
| Al-to-poly | ~5-50 ohm | estimated | typical for era |

## Wire Parasitic Models

For Elmore delay estimation:
- R_wire = Rs * length / width
- C_wire = C_per_area * length * width + 2 * C_fringing * length
- tau_wire = R_wire * C_wire (distributed RC, actual delay ~ 0.38 * R*C)

## Critical Path Implications

- 4004 clock distribution: polysilicon gate fan-out to ~100 gates
- Bus output drivers: aluminum trace from pad to logic block
- ROM word lines: polysilicon, dominant source of access time delay
- RAM bit lines: aluminum, faster than polysilicon word lines
