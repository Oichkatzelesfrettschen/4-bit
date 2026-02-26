# Process Parameter Audit

Systematic validation of every process parameter used in the Rust model
(`mcs4-core/src/process/`) against primary and secondary sources.

Date: 2026-02-25

## Methodology

For each parameter:
1. Record the value used in code (from ProcessParams::default())
2. Identify primary source(s) with page/table references
3. Identify corroborating secondary sources
4. Assess confidence: HIGH / MEDIUM / LOW
5. Record uncertainty bounds
6. Note any discrepancies between sources

## Gate Oxide Thickness (tox)

**Code value:** 80 nm (80e-9 m)
**Primary sources:**
- faggin1970sgt: "Silicon Gate Technology", SSE 1970 -- describes gate oxidation
  for the self-aligned process. States thermally grown SiO2; specific thickness
  is behind paywall, but the process description implies thin gate oxide for
  SGT performance advantage over metal-gate.
- intel4004ds: Datasheet does not explicitly state tox, but the operating
  voltage range (VDD=-15V) and VOH/VOL specs constrain the oxide field to
  ~1.9 MV/cm if tox=80nm, well below breakdown (~10 MV/cm).

**Secondary sources:**
- osburn2002tox: States early 1970s tox was "50-100 nm" for MOS processes.
  80nm falls in the center of this range.
- hu2010devices: States 300nm for 10um technology, but this likely refers to
  metal-gate processes; silicon-gate processes used thinner oxide for performance.
- faggin2015memoirs: Faggin discusses the SGT process advantages including
  thinner gate oxide enabled by self-aligned processing.

**Assessment:** MEDIUM confidence. The 80nm value is consistent with the
osburn2002tox range and the performance characteristics of the 4004. However,
no primary source explicitly states "80nm" for the 4004 process. The true
value could be anywhere in the 60-100nm range.

**Uncertainty:** +/-20nm (60-100nm)

## Threshold Voltages

### Enhancement Mode (Vth_enh)

**Code value:** -3.0 V
**Primary sources:**
- intel4004ds: VOH >= -12V (at VDD=-15V, IOH=-100uA). VOL <= -1V (at VDD=-15V,
  IOL=100uA). These output levels are consistent with enhancement-mode pMOS
  switches with |Vth| in the 2-4V range.
- intelmcs4ds1971: System-level timing specs confirm logic threshold compatibility.

**Secondary sources:**
- deal1974oxidecharges: Fixed oxide charge (Qf ~ 1e11 cm-2 for early thermal
  SiO2) shifts pMOS Vth more negative. This is consistent with |Vth|>2V.
- uvicrec4004: Confirms depletion-mode load architecture, implying enhancement
  Vth is significantly negative.

**Assessment:** MEDIUM confidence. The -3V center estimate is consistent with
datasheet output levels and the known oxide charge effects of the era. The
actual value could range from -2V to -4V depending on oxide charge density
and substrate doping.

**Uncertainty:** +/-1V (-2V to -4V)

### Depletion Mode (Vth_dep)

**Code value:** +1.0 V
**Primary sources:**
- No primary source explicitly states the depletion-mode threshold.
- intel4004ds: The existence of depletion loads is confirmed by the circuit
  architecture (verified by analyzer).

**Secondary sources:**
- macpherson1971implant: Ion implantation mechanism for creating depletion-mode
  devices. Typical Vth_dep for pMOS depletion loads: +0.5V to +2V.

**Assessment:** LOW confidence. The +1V value is a reasonable center estimate
but lacks primary-source confirmation. Could range from +0.5V to +2V.

**Uncertainty:** +/-0.5V (+0.5V to +1.5V)

## Hole Mobility (mu_0)

**Code value:** 175 cm2/Vs
**Primary sources:**
- No primary Intel source states the mobility for their process.

**Secondary sources:**
- hofstein1965frequency: Early MOS mobility measurements. Surface hole mobility
  for pMOS on Si is typically 150-250 cm2/Vs depending on orientation and
  surface quality.
- sze1981semiconductor: Bulk hole mobility in Si at 300K is ~450 cm2/Vs.
  Surface mobility is degraded by factor of 2-3x, giving ~150-225 cm2/Vs.
- faggin2015memoirs: Notes (111) substrate orientation for early SGT, which
  has lower surface mobility than (100).

**Assessment:** MEDIUM confidence. 175 cm2/Vs is consistent with surface
hole mobility on (111) Si with moderate interface roughness scattering.
The value directly affects simulated current drive and switching speed.

**Uncertainty:** +/-50 cm2/Vs (125-225 cm2/Vs)

**Open question:** Was the substrate (111) or (100)? Faggin's memoirs suggest
(111) for the earliest SGT work, but Intel may have switched to (100) later.
(100) has ~20% higher surface mobility.

## Substrate Doping (N_sub)

**Code value:** 4.5e14 cm-3 (corresponding to ~10 ohm-cm n-type)
**Primary sources:**
- No primary Intel source states the substrate resistivity for this process.

**Secondary sources:**
- Common practice for pMOS of the era: 5-20 ohm-cm n-type substrate.
- 10 ohm-cm is a standard starting material specification.

**Assessment:** MEDIUM confidence. 10 ohm-cm is a reasonable assumption but
lacks confirmation. Affects body effect (gamma) and junction capacitance.

**Uncertainty:** factor of 2x (5-20 ohm-cm, or 2.3e14 to 9e14 cm-3)

## Channel-Length Modulation (lambda)

**Code value:** 0.02 V-1
**Assessment:** LOW confidence. Estimated from typical long-channel values.
10um gate length should have modest CLM. Range: 0.01-0.05 V-1.

## DIBL Coefficient (eta_dibl)

**Code value:** 0.005 V/V
**Assessment:** HIGH confidence for being negligible. At 10um channel length,
DIBL is effectively zero. The 0.005 value is structurally present for
future shorter-channel models but does not affect simulation accuracy.

## Mobility Degradation (theta)

**Code value:** 0.03 V-1
**Assessment:** LOW confidence. Estimated from typical vertical-field mobility
degradation for thick-oxide pMOS. No primary source available.

## Junction Depth (x_j)

**Code value:** 2 um (2e-6 m)
**Assessment:** LOW confidence. Typical for diffusion-based junctions of the era.
Ion implantation (if used) could give shallower junctions.

## Source/Drain Doping (N_sd)

**Code value:** 1e19 cm-3
**Assessment:** MEDIUM confidence. p+ diffused junctions typically achieve
1e18-1e20 cm-3 surface concentration. Affects contact resistance and
junction capacitance.

## Temperature

**Code value:** 300 K (27 C)
**Assessment:** HIGH confidence as nominal. Datasheet specifies 0-70C commercial
range. 300K is standard nominal.

## Summary of Confidence Levels

| Parameter | Value | Confidence | Primary Source? |
|-----------|-------|------------|-----------------|
| tox | 80nm | MEDIUM | No (derived from range) |
| Vth_enh | -3V | MEDIUM | Indirect (VOH/VOL) |
| Vth_dep | +1V | LOW | No |
| mu_0 | 175 cm2/Vs | MEDIUM | No (secondary ranges) |
| N_sub | 4.5e14/cm3 | MEDIUM | No (standard practice) |
| lambda | 0.02/V | LOW | No |
| eta_dibl | 0.005 V/V | HIGH (negligible) | N/A (physics constraint) |
| theta | 0.03/V | LOW | No |
| x_j | 2um | LOW | No |
| N_sd | 1e19/cm3 | MEDIUM | No (typical range) |
| VDD | -15V | HIGH | intel4004ds |
| VSS | 0V | HIGH | intel4004ds |
| L_min | 10um | HIGH | multiple sources |
| T_nom | 300K | HIGH | standard |

## Priority Actions

1. Obtain faggin1970sgt full text (behind Elsevier paywall) to confirm tox
2. Search for Intel process characterization papers from 1970-1972 era
3. Determine substrate orientation ((111) vs (100)) definitively
4. Cross-reference Vth against circuit simulation of known-good test vectors
