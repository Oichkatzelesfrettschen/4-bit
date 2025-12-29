# Intel 4-bit Microprocessor Documentation Archive

Research archive for Intel's pioneering 4-bit microprocessors: the 4004, 4040, and their associated MCS-4/MCS-40 chip families.

## Overview

### Intel 4004 (1971)
The world's first commercially available single-chip microprocessor, designed by Federico Faggin, Ted Hoff, and Stanley Mazor. Key specifications:
- 4-bit CPU with 2,300 transistors
- 10 micron process (silicon gate PMOS)
- 740 kHz clock speed
- 46 instructions
- 4KB program memory addressing
- 640 bytes data memory addressing

### Intel 4040 (1974)
Enhanced successor to the 4004 with expanded capabilities:
- 4-bit CPU with 3,000 transistors
- 10 micron process (silicon gate enhancement-load PMOS)
- 5.185 MHz system clock (10.8 microsecond cycle time)
- 60 instructions (14 more than 4004)
- Interrupt capability
- Additional registers (24 index registers vs 16)
- ~62,000 instructions per second

### MCS-4 System (4004 Family)
Complete microcomputer system comprising:
- **4001**: 256x8-bit ROM with 4-bit I/O port
- **4002**: 320-bit RAM with 4-bit output port
- **4003**: 10-bit serial-in, parallel-out shift register
- **4004**: 4-bit CPU

### MCS-40 System (4040 Family)
Enhanced microcomputer system with:
- **4040**: Enhanced 4-bit CPU
- **4289**: Standard memory interface
- **4308**: 1Kx8-bit ROM
- **4207/4209/4211**: Clock generators
- **4101**: 256x4-bit static RAM
- **4201**: Clock generator

## Repository Structure

```
docs/
  4004/                    # Intel 4004 documentation
    schematics/            # Circuit schematics (GIF)
    masks/                 # Silicon mask artwork (JPG)
    4004_schematic.pdf     # Original schematic scans
    intel-4004-datasheet.pdf
    4004_applelogic_datasheet.pdf

  4040/                    # Intel 4040 documentation
    4040-datasheet.pdf

  MCS-4/                   # MCS-4 system documentation
    MCS-4_Assembly_Language_Programming_Manual_Dec73.pdf
    MCS-4_UsersManual_Feb73.pdf
    i4001-schematic.gif    # 4001 ROM schematic
    i4002-schematic.gif    # 4002 RAM schematic
    i4003-schematic.gif    # 4003 shift register schematic

  MCS-40/                  # MCS-40 system documentation
    MCS-40_Users_Manual_Nov74.pdf
    MCS-40_Advance_Specifications_Sep74.pdf

  emulators/               # Simulators and emulators
    i400x_analyzer_20210324.zip  # Lajos Kintli's gate-level simulator
```

## Emulators and Simulators

### Cycle-Accurate / Gate-Level Simulators

| Project | Type | Language | Accuracy | Link |
|---------|------|----------|----------|------|
| **Lajos Kintli's MCS-4 Analyzer** | Gate-level | Win64 exe | Transistor-level | [4004.com](https://www.4004.com/mcs4-masks-schematics-sim.html) |
| **j4004** | Cycle-accurate | Kotlin | Near cycle-accurate | [GitHub](https://github.com/asicerik/j4004) |
| **go4004** | Cycle-accurate | Go/SDL | Near cycle-accurate | [GitHub](https://github.com/asicerik/go4004) |
| **OpenCores MCS-4** | FPGA | Verilog | Cycle-accurate | [OpenCores](https://opencores.org/projects/mcs-4) |
| **e4004** | Emulator | JavaScript | Instruction-level | [e4004.szyc.org](http://e4004.szyc.org/) |

### Lajos Kintli's Simulator (Included)
The `i400x_analyzer_20210324.zip` in `docs/emulators/` contains:
- Windows 64-bit executable
- Verified 4004 mask artwork
- Transistor netlist
- Complete MCS-4 masks and schematics
- Animated gate-level simulation

## External Resources

### Primary Documentation Sources
- [4004.com - 50th Anniversary Project](https://www.4004.com/) - Definitive resource with Intel-licensed documentation
- [Bitsavers Intel Archive](http://www.bitsavers.org/components/intel/) - Comprehensive PDF collection
- [ChipDB Datasheets](https://datasheets.chipdb.org/Intel/) - Clean datasheet scans

### Additional Manuals (Not Downloaded)
- Intel Intellec 4 MOD 40 Reference Schematics - [Internet Archive](https://archive.org/details/bitsavers_intelMCS40ReferenceSchematics_81608214)
- Intel Intellec 4 and Micro Computer Modules Jan74 - [Internet Archive](https://archive.org/details/bitsavers_intelMCS4IroComputerModulesJan74_4532529)
- Pro-Log Microprocessor User's Guide (Sep 1975) - Covers 4004/4040/8008/8080

### Technical References
- [intel4004.com - Original Schematics](http://www.intel4004.com/4004_original_schematics.htm)
- [WikiChip 4040](https://en.wikichip.org/wiki/intel/mcs-40/4040)
- [Retrotechnology MCS 4/40](https://www.retrotechnology.com/restore/4040_doc.html)

## Historical Significance

The 4004 was originally designed for Busicom calculators but Intel negotiated rights to sell it for non-calculator applications. This decision launched the microprocessor revolution.

Key dates:
- **1969**: Busicom approaches Intel
- **1970**: Federico Faggin joins Intel, begins 4004 design
- **November 15, 1971**: Intel 4004 announced publicly
- **1974**: Intel 4040 released
- **November 15, 2006**: Intel releases 4004 schematics
- **November 15, 2009**: Complete MCS-4 artwork and simulator released

## Recent Developments

In November 2025, Klaus Scheffler and Lajos Kintli completed a discrete-transistor implementation of the complete MCS-4 system, achieving 2x the original clock speed.

## License

Original Intel documentation is provided under Intel's non-commercial use license as granted for the 4004.com project. Third-party emulators have their own licenses - see individual project repositories.

## Contributing

Pull requests welcome for:
- Additional documentation scans
- Improved emulators
- Educational materials
- Historical corrections
