# Reliability Models: Intel 10um pMOS

Degradation and reliability models applicable to the MCS-4/MCS-40 process.

## NBTI (Negative Bias Temperature Instability)

Primary reference: jeppson1977nbti

NBTI is the dominant reliability mechanism for pMOS under negative gate bias.
At VDD=-15V, the gate oxide field is approximately 1.9 MV/cm (for 80nm tox).

- Mechanism: interface trap generation under negative Vgs stress
- Impact: threshold voltage shift (Vth becomes more negative over time)
- Activation energy: ~0.2-0.3 eV (accelerated at elevated temperature)
- Time dependence: power-law (delta_Vth proportional to t^n, n ~ 0.25)

## TDDB (Time-Dependent Dielectric Breakdown)

Primary reference: crook1979tddb

Oxide reliability at 1.9 MV/cm field stress:
- Well below the intrinsic breakdown field (~10 MV/cm for SiO2)
- TDDB lifetime at 1.9 MV/cm and 70C: effectively infinite for normal operation
- Concern only at elevated temperature with contamination-induced weak spots

## HCI (Hot Carrier Injection)

Primary reference: hu1985hotelectron

- Less prominent in pMOS than nMOS (holes have lower impact ionization rate)
- At VDD=-15V, drain field can cause some hot-hole injection
- Primarily relevant for output driver transistors under switching stress

## Electromigration

Primary reference: dheurle1971electrotransport

- Single-layer aluminum metallization without diffusion barriers
- Current density limits: ~1e5 A/cm2 for reliable operation at 70C
- Critical paths: clock driver metal, bus output driver metal, VDD/VSS rails
- Mitigation: wider metal traces on high-current paths (visible in die photos)

## Oxide Charge Drift

Primary reference: deal1974oxidecharges

- Mobile ionic contamination (Na+, K+) in gate oxide causes Vth drift
- Intel's clean processing reduced but did not eliminate this mechanism
- Burn-in screens designed to accelerate and stabilize mobile charge drift
