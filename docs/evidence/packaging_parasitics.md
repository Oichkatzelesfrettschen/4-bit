# Packaging Parasitics: DIP Packages for MCS-4/MCS-40

Electrical models for ceramic and plastic DIP packages used in the chip families.

Primary reference: tummala1989packaging

## Package Types

| Chip | Package | Pin Count |
|------|---------|-----------|
| 4004 | 16-pin ceramic DIP (CDIP) | 16 |
| 4040 | 24-pin CDIP | 24 |
| 4001 | 16-pin CDIP | 16 |
| 4002 | 16-pin CDIP | 16 |
| 4003 | 16-pin CDIP | 16 |
| 4101 | 18-pin CDIP | 18 |
| 4201 | 16-pin CDIP | 16 |
| 4289 | 16-pin CDIP | 16 |
| 4308 | 24-pin CDIP | 24 |

## Electrical Parameters (typical ceramic DIP)

| Parameter | 16-pin | 24-pin | Source |
|-----------|--------|--------|--------|
| Bond wire inductance | 1-3 nH | 1-3 nH | tummala1989packaging |
| Lead inductance | 3-8 nH | 5-10 nH | tummala1989packaging |
| Lead capacitance | 1-3 pF | 1-3 pF | tummala1989packaging |
| Pin-to-pin capacitance | 0.5-1 pF | 0.5-1 pF | estimated |
| Bond wire resistance | 0.05-0.1 ohm | 0.05-0.1 ohm | estimated |

## Thermal Parameters

| Parameter | 16-pin CDIP | 24-pin CDIP | Source |
|-----------|-------------|-------------|--------|
| theta_JA (junction to ambient) | 60-80 C/W | 50-70 C/W | tummala1989packaging |
| theta_JC (junction to case) | 20-30 C/W | 15-25 C/W | estimated |
| Max junction temperature | 125 C | 125 C | intel4004ds |
| Max ambient (commercial) | 70 C | 70 C | intel4004ds |

## Power Dissipation Budget

- 4004 at 740 kHz: 60 mW typical (datasheet), 100 mW max
- At theta_JA = 70 C/W and P = 60 mW: delta_T = 4.2 C (negligible)
- At P = 100 mW: delta_T = 7.0 C (still within thermal budget at 70 C ambient)
