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
- docs/photomicrographs/4004/4004-composite-thumb.jpg
- docs/photomicrographs/4004/4004-layout-cc0.jpg
- docs/4004/annotated/i4004-schematic-bus-labels.png
- docs/4004/annotated/i4004-poly-diffusion-transistors.png

Notes
- The emulator layers are used by the i400x analyzer to extract netlists and compare to schematics.
- External die-shot tiles exist for Intel 4004B at siliconprawn.org (license unverified; not imported).
- The CC0 4004 layout poster photo is not a die shot; it is a photographed layout drawing.
## Intel 4001 (ROM + I/O)
Local artifacts
- docs/MCS-4/i4001-schematic.gif
- docs/emulators/i4001-metal.bmp
- docs/emulators/i4001-vias.bmp
- docs/emulators/i4001-poly.bmp
- docs/emulators/i4001-diffusion.bmp
- docs/emulators/i4001-schematic.bmp
- docs/emulators/i4001-signals.txt
- docs/photomicrographs/4001/4001-composite-thumb.jpg
- docs/4001/annotated/i4001-schematic-bus-labels.png
- docs/4001/annotated/i4001-poly-diffusion-transistors.png

## Intel 4002 (RAM + output)
Local artifacts
- docs/MCS-4/i4002-schematic.gif
- docs/emulators/i4002-metal.bmp
- docs/emulators/i4002-vias.bmp
- docs/emulators/i4002-poly.bmp
- docs/emulators/i4002-diffusion.bmp
- docs/emulators/i4002-schematic.bmp
- docs/emulators/i4002-signals.txt
- docs/photomicrographs/4002/4002-composite-thumb.jpg
- docs/4002/annotated/i4002-schematic-bus-labels.png
- docs/4002/annotated/i4002-poly-diffusion-transistors.png

## Intel 4003 (shift register)
Local artifacts
- docs/MCS-4/i4003-schematic.gif
- docs/emulators/i4003-metal.bmp
- docs/emulators/i4003-vias.bmp
- docs/emulators/i4003-poly.bmp
- docs/emulators/i4003-diffusion.bmp
- docs/emulators/i4003-schematic.bmp
- docs/emulators/i4003-signals.txt
- docs/photomicrographs/4003/4003-composite-thumb.jpg
- docs/4003/annotated/i4003-schematic-bus-labels.png
- docs/4003/annotated/i4003-poly-diffusion-transistors.png

## Intel 4040 (CPU)
Local artifacts
- docs/4040/4040-datasheet.pdf
- docs/MCS-40/4040_Datasheet.pdf

External photo references
- https://happytrees.org/chips/Intel_-_P4040 (package photo, CC BY-SA 4.0)
- https://commons.wikimedia.org/wiki/File:Ic-photo-Intel--P4040--(4040-CPU).jpg (package photo, CC BY-SA)
- https://www.cpu-collection.de/?l0=co&l1=Intel&l2=4040&l3=Intel_4040 (package photos; license not stated, not imported)
## System-level wiring and schematics
- docs/MCS-40/Intel_Intellec_4_MOD_40_Reference_Schematics.pdf (board-level wiring)
- docs/MCS-4/MCS-4_UsersManual_Feb73.pdf (system diagrams)
- docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf (system diagrams)
- docs/MCS-40/4004_schematic.pdf (duplicate of 4004 schematic)
- docs/MCS-40/1975_Intel_Data_Catalog.pdf (component catalog)

## Emulator analyzer reference
- docs/emulators/readme.txt (analyzer workflow and layer semantics)
- docs/emulators/license.txt (Intel license for source materials)
- docs/NETLIST_WORKFLOW.md (local workflow summary)

## Layer analysis notes (from analyzer readme)
- Netlists are extracted from both mask layers and schematic bitmaps, then matched.
- Differences noted in the 4004 CPU between layout and schematic include gate input order and TEST pin revision.
- Bootstrap loader elements appear as resistor symbols in schematics but are RC + transistor networks in layout.
- Analyzer highlights transistors and wires by signal state and can simulate CPU/ROM/RAM together.

## Known gaps
- No confirmed die-shot photos in repo for 4040 or MCS-40 support chips.
- Only package photos found so far for 4040; no die photomicrographs located yet.
- Only thumbnail composite photomicrographs for 4001-4004 are in repo; no full-size composites yet.
- External 4004B die-shot tiles exist (siliconprawn.org) but license is unverified.
- No mask-layer sets for 4040 or MCS-40 support chips in repo.
- No transistor-level layer bitmaps for 4040 in repo.
- Provenance/licensing for external photomicrograph links is recorded in docs/evidence/photomicrograph_permissions.md.
- Full-size composite photomicrographs from alumni.media.mit.edu are not imported; license not confirmed.
- Wikimedia Commons die-shot galleries list an Intel 4004 layout image but no 4040 die shots.

## External sources to review next
- https://www.4004.com/ (schematics, masks, and historical material)
- https://www.4004.com/mcs4-masks-schematics-sim.html (mask artwork and composite photomicrographs)
- http://alumni.media.mit.edu/~mcnerney/2009-4004/4001-masks-composite.jpg
- http://alumni.media.mit.edu/~mcnerney/2009-4004/4002-masks-composite.jpg
- http://alumni.media.mit.edu/~mcnerney/2009-4004/4003-masks-composite.jpg
- http://alumni.media.mit.edu/~mcnerney/2009-4004/4004-masks-composite.jpg
- http://alumni.media.mit.edu/~mcnerney/2009-4004/4001-composite-photo.jpg
- http://alumni.media.mit.edu/~mcnerney/2009-4004/4004-composite-photo.jpg
- http://www.intel4004.com/ (schematics and history)
- https://bitsavers.org/components/intel/ (manuals and schematics)
- https://commons.wikimedia.org/wiki/File:Ic-photo-Intel--P4040--(4040-CPU).jpg (4040 package photo, CC BY-SA)
- https://happytrees.org/chips/Intel_-_4040_family (CPU Grave Yard, CC BY-SA 4.0)
- https://happytrees.org/chips/Intel_-_P4040 (CPU Grave Yard P4040 package photo)
- https://commons.wikimedia.org/wiki/Gallery:Die_shots_of_microprocessors (contains CC0 4004 layout image; no 4040 die shots)
- https://commons.wikimedia.org/wiki/Category:Intel_microprocessor_dies
- http://siliconprawn.org/map/intel/ (die-shot map listing; no 4040 entry currently)
- http://siliconprawn.org/map/intel/4004b/ (4004B die-shot tiles and images; license unverified)

## Provenance status (external)
- 4004.com: CC BY-NC-SA 3.0 statement recorded in docs/evidence/photomicrograph_permissions.md.
- alumni.media.mit.edu photomicrographs: license not recorded in repo; verify terms.
- Wikimedia 4040 package photo: CC BY-SA per file page; attribution required.
- CPU Grave Yard (happytrees.org): CC BY-SA 4.0 per site footer and file page.
- siliconprawn.org die-shot tiles: license not recorded in repo; verify terms.
- intel4004.com: license not recorded in repo; verify terms.
