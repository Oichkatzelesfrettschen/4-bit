# Chip Artifacts Catalog

Scope
- Catalog of local schematics, masks, layer bitmaps, and manuals for MCS-4/MCS-40 chips.
- Focus on wiring diagrams, circuit schematics, and transistor-layer artifacts.

Legend (layers)
- metal: top interconnect layer
- vias/contacts: inter-layer connections
- poly: polysilicon gate layer
- diffusion: active silicon regions
- schematic: drawn logic/circuit diagram

## Intel 4004 (CPU)
Local artifacts
- docs/4004/4004_schematic.pdf
- docs/4004/schematics/i4004-schematic.gif
- docs/4004/masks/4004-masks-composite.jpg
- docs/4004/intel-4004-datasheet.pdf
- docs/4004/4004_applelogic_datasheet.pdf
- docs/emulators/i4004-metal.bmp
- docs/emulators/i4004-vias.bmp
- docs/emulators/i4004-poly.bmp
- docs/emulators/i4004-contacts.bmp
- docs/emulators/i4004-diffusion.bmp
- docs/emulators/i4004-schematic.bmp
- docs/emulators/i4004-signals.txt

Notes
- The emulator layers are used by the i400x analyzer to extract netlists and compare to schematics.
## Intel 4001 (ROM + I/O)
Local artifacts
- docs/MCS-4/i4001-schematic.gif
- docs/emulators/i4001-metal.bmp
- docs/emulators/i4001-vias.bmp
- docs/emulators/i4001-poly.bmp
- docs/emulators/i4001-diffusion.bmp
- docs/emulators/i4001-schematic.bmp
- docs/emulators/i4001-signals.txt

## Intel 4002 (RAM + output)
Local artifacts
- docs/MCS-4/i4002-schematic.gif
- docs/emulators/i4002-metal.bmp
- docs/emulators/i4002-vias.bmp
- docs/emulators/i4002-poly.bmp
- docs/emulators/i4002-diffusion.bmp
- docs/emulators/i4002-schematic.bmp
- docs/emulators/i4002-signals.txt

## Intel 4003 (shift register)
Local artifacts
- docs/MCS-4/i4003-schematic.gif
- docs/emulators/i4003-metal.bmp
- docs/emulators/i4003-vias.bmp
- docs/emulators/i4003-poly.bmp
- docs/emulators/i4003-diffusion.bmp
- docs/emulators/i4003-schematic.bmp
- docs/emulators/i4003-signals.txt

## Intel 4040 (CPU)
Local artifacts
- docs/4040/4040-datasheet.pdf
- docs/MCS-40/4040_Datasheet.pdf
## System-level wiring and schematics
- docs/MCS-40/Intel_Intellec_4_MOD_40_Reference_Schematics.pdf (board-level wiring)
- docs/MCS-4/MCS-4_UsersManual_Feb73.pdf (system diagrams)
- docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf (system diagrams)
- docs/MCS-40/4004_schematic.pdf (duplicate of 4004 schematic)
- docs/MCS-40/1975_Intel_Data_Catalog.pdf (component catalog)

## Emulator analyzer reference
- docs/emulators/readme.txt (analyzer workflow and layer semantics)
- docs/emulators/license.txt (Intel license for source materials)

## Layer analysis notes (from analyzer readme)
- Netlists are extracted from both mask layers and schematic bitmaps, then matched.
- Differences noted in the 4004 CPU between layout and schematic include gate input order and TEST pin revision.
- Bootstrap loader elements appear as resistor symbols in schematics but are RC + transistor networks in layout.
- Analyzer highlights transistors and wires by signal state and can simulate CPU/ROM/RAM together.

## Known gaps
- No confirmed die-shot photos in repo for 4001/4002/4003/4004/4040.
- No mask-layer sets for 4040 or MCS-40 support chips in repo.
- No transistor-level layer bitmaps for 4040 in repo.

## External sources to review next
- https://www.4004.com/ (schematics, masks, and historical material)
- http://www.intel4004.com/ (schematics and history)
- https://bitsavers.org/components/intel/ (manuals and schematics)
