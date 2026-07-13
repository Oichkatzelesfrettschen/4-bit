# Intellec 4 MOD 40 98-013A Schematic Sheet Index

## Scope

This index covers every PDF page in the retained 98-013A reference-schematic
scan. It records only text visible in review renders. The 2026-07-13 JPEG title
crop pass improves title recognition on the previously low-resolution sheets;
its engine outputs and image-decoder limitation are retained in
`docs/evidence/INTELLEC_MOD40_OCR_STATUS.md`. An unresolved label is not a net
claim and cannot enable historical execution.

Source: `intellec4-mod40-reference-schematics`, SHA-256
`34f82ae6228f6ee0832c9696b4a0e922f8ea4bfccf92131476d013108b703bdb`.
The page-level evidence boundary is maintained in
`docs/evidence/INTELLEC_MOD40_PRIMARY_EVIDENCE.md`.

| PDF page | Drawing number | Title | Area | Visible labels | Confidence |
| --- | --- | --- | --- | --- | --- |
| 1 | 98-013A | INTELLEC 4 MOD 40 REFERENCE SCHEMATIC DRAWINGS | document cover | none | high |
| 2 | 1000276-02 | PRINTED WIRING ASSY CENTRAL PROCESSOR MOD | central processor module layout | A1-A4, A11-A32, P1, P4, and card-edge placement | high |
| 3 | 2000318 | SCHEMATIC CENTRAL PROCESSOR MODULE | central processor module | CPU RESET; RESET; CM-ROM; CM-RAM0-3; MEMORY DATA IN/OUT; TTY | high |
| 4 | 1000013? | PRINTED WIRING ASSEMBLY MOTHER BOARD | motherboard/backplane layout | J1-J18, J22, J42, P38, P37, and P39 placement | medium |
| 5 | 2000077 | SCHEMATIC MOTHER BOARD | motherboard/backplane | J1-J18; CPU RESET; USER RESET OUT; TTY PRINTER; TTY READER CONTROL; remaining text unresolved | medium |
| 6 | unreadable | MEMORY CONTROLLER ASSY | memory controller board layout | A1-A31 controller population; backplane connector/pin labels unresolved | medium |
| 7 | 2000319 | MEMORY CONTROLLER BOARD (INT4MOD40) SCHEMATIC | memory controller board | CM-ROM; CM-RAM0-3; 4002 RESET ENABLE; 4002 RESET; ENABLE MON PROM; PROM SELECT; CPU RESET; STOP PB | high |
| 8 | 1000279 | PRINTED WIRING ASSY RAM MEMORY MODULE | RAM memory module layout | card-edge fingers; no net label transcribed | high |
| 9 | 05-0042-000 | ASSEMBLY DRAWING IN-28 | RAM memory module assembly | module/edge connector; no net label transcribed | high |
| 10 | 01-0176-001? | SCHEMATIC IN-28 | RAM memory module | MODULE SELECT; WRITE; MAD0-MAD11; 32 2102 locations; remaining pin labels unresolved | medium |
| 11 | 1000143 | PRINTED WIRING ASSEMBLY FRONT PANEL LOGIC, SHEET 1 OF 2 | front-panel logic board layout | front-panel control labels; connector names unresolved | high |
| 12 | 1000143 | PRINTED WIRING ASSEMBLY FRONT PANEL LOGIC, SHEET 2 OF 2 | front-panel logic board layout | J1, J2, J3, P39, 8244/9348, 8233, 8266, 3205, 3404, and TTL placement | high |
| 13 | 2000329 | SCHEMATIC (MOD 40) FRONT PANEL LOGIC | front-panel logic board | front-panel control nets; individual labels unresolved | high |
| 14 | 1000034-01 | PRINTED WIRING ASSEMBLY FRONT PANEL DISPLAY BOARD | front-panel display board layout | P40 50-pin connector; LED1-LED46 General Electric SSL-22 placement | high |
| 15 | 2000036 | SCHEMATIC FRONT PANEL DISPLAY (INTELLEC 4) | front-panel display board | display connector/pin labels partly readable | high |
| 16 | 1000081 | PRINTED WIRING ASSEMBLY INPUT/OUTPUT MODULE | input/output module layout | connector/pin labels unresolved | high |
| 17 | 2000083 | SCHEMATIC INPUT/OUTPUT | input/output module | P1; DATA DRIVER; ADDRESS DRIVER | high |
| 18 | 1000090 | PRINTED WIRING ASSEMBLY PROM PROGRAMMER | PROM programmer layout | connector/pin labels unresolved | high |
| 19 | 2000092 | SCHEMATIC PROM PROGRAMMER | PROM programmer | DATA DRIVER; ADDRESS DRIVER; port reference chart | high |
| 20 | 1000063 | PRINTED WIRING ASSEMBLY PROM MEMORY MODULE | PROM memory module layout | connector/pin labels unresolved | high |
| 21 | 2000065 | PROM MEMORY MODULE SCHEMATIC | PROM memory module | PROM DATA OUT; MD1 4; MD1 5; MD1 6; MD1 7; VDD | high |
| 22 | 1000066 | PRINTED WIRING ASSEMBLY INSTRUCTION DATA STORAGE | instruction/data storage module layout | connector/pin labels unresolved | high |
| 23 | 2000068 | SCHEMATIC INSTRUCTION/DATA STORAGE | instruction/data storage module | P1-42 ENABLE MON PROM; P1-41 OUT; ROM | high |
| 24 | 1000119-01 | PRINTED WIRING ASSEMBLY DATA STORAGE MODULE | data storage module layout | connector/pin labels unresolved | high |
| 25 | 2000121 | SCHEMATIC DATA STORAGE/OEM MODULE | data storage/OEM module | 16 x 4002, 3205 selection, CM-RAM0-3, CPU RESET, and 4002 RESET | high |
| 26 | 4000328 | FRONT PANEL ASSEMBLY (MOD 40) | MOD 40 chassis front panel | P36; front-panel harness labels partly readable | high |
| 27 | 2000309 | AC-DC PWR DISTB. | chassis power distribution | J38/P38; J45/P45; J36/P36; J44/P44; FRONT PANEL PROGRAMMING SWITCH | high |
| 28 | Power One | SCHEMATIC DIAGRAM CP110, D5-12 S113 POWER SUPPLIES | chassis power supply | +80 V, +12 V, -10 V, -12 V, +5 V, COM, and sense-output labels | high |
| 29 | 2000325 | SCHEMATIC TTY REF (INT 4/MOD 4) | TTY interface reference | TTY interface labels; connector/net labels require detailed extraction | high |
| 30 | 1000152 | PROM SOCKET ASSY & SCHEMATIC | PROM socket assembly | 24-pin socket; TEXTOOL | high |
| 31 | 2000178 | I/O CONNECTOR BOARD SCHEMATIC | I/O connector board | GND; 37-pin connector | high |
| 32 | 4000168 | INTELLEC SYSTEM FLAT CABLE ASSY | chassis cabling | 50-pin; 40-pin; 26-pin connectors | high |
| 33 | 4000237 | INTELLEC SYSTEM I/O CABLE | I/O cable assembly | AMP connector; PWB INTELLEC 4; PWB INTELLEC 8 | high |
| 34 | 4000242 | INTELLEC 4 CABLE ROUTING (REF) | chassis cable routing | cable routing labels unreadable | high |
| 35 | unreadable | back cover | document back cover | none | low |

## Net-extraction order

1. Extract drawing 2000318 clock, 4289, monitor, 4002, and P4/J4 nets.
2. Extract drawing 2000319 mode selection, RESET, STOP, and monitor/PROM nets.
3. Extract drawing 10 and the motherboard sheets for imm4-72 to imm6-28
   address, byte, write, and card-select routes.
4. Extract drawing 2000329 and display sheets before enabling console actions.
5. Extract drawing 2000325 and the ASR-33 sheets before modeling electrical
   terminal polarity or current-loop behavior.

All former low-confidence sheets now have a title, scope, and bounded initial
record. The board layouts establish placement and connector presence only.
PDF pages 4, 5, 6, 7, 10, 12, 13, 14, 25, and 28 still require targeted
connector, pin, polarity, and cross-sheet path extraction before a complete
board-net claim. Page 35 is the back cover and has no board net content.
