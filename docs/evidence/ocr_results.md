# OCR Evidence Results (2026-01-07T03:58:36Z)

Clock period evidence (OCR noise preserved)
- Intel 4004 datasheet sidecar: docs/evidence/ocr/4004-datasheet.txt
```text
   744	T 2 =
   745	8 ~
   746	20
   747	0 20 40 60 80
   748	AMBIENT TEMPERATURE {°C}
   749	A.C. Characteristics
   750	Ta=0°C to 70°C, Vgg-Vpp = 15V 5%
   751	Symbol Parameter Min. TI;:)I Max. Unit Test Conditions
   752	toy Clock Period 1.35 2.0 usec
   753	tyR Clock Rise Time 50 ns
   754	toF Clock Fall Times 50 ns
   755	topw Clock Width 380 480 ns
   756	teD1 Clock Delay ¢q to ¢o’ 400 550 ns
   757	tyD2 Clock Delay ¢5 to ¢4 150 ns
   758	tw Data-In, CM, SYNC Write Time 350 100 ns
   759	tyi1.3] Data-In, CM, SYNC Hold Time 40 20 ns
   760	t(3] Data Bus Hold Time During My-X4 and 150 ns
```
- Intel 4040 datasheet sidecar: docs/evidence/ocr/4040-datasheet.txt
```text
   560	go 36 |
   561	:é 30 ;s'wsv
   562	4.2,
   563	25
   564	%, -20 0 20 40 60 80 85
   565	T °C)
   566	A.C. Characteristics 7,=0°Cto 70°C, Vss -Vpp = 15V 5%
   567	Limit
   568	Symbol Parameter Min. Typ. Max. | Unit Conditions
   569	tey Clock Period 1.35 2.0 | usec
   570	toR Clock Rise Time 50 ns
   571	tog Clock Fall Times 50 ns
   572	tdpw Clock Width . 380 480 | ns
   573	t$p1 Clock Delay ¢4 to ¢2 400 550 | ns
   574	t9p2 Clock Delay ¢ to ¢4 150 ns
   575	tw Data-In, CM, SYNC Write Time 350 100 ns
```

Transistor count evidence (4004.com analyzer)
- Source: /tmp/i400x_analyzer/unpacked/readme.txt
```text
   150	Finally both sources contained the same result apart from the bootstrap loaders,
   151	which are indicated on the schematic as a resistor, however those are combinations of
   152	a resistor, a capacitor and a transistor. The matched data contains the following
   153	number of components:
   155		component type		layout	    schematic			
   156		---------------------------------------------
   157		transistor:		  1807	1807-66= 1741
   158		resistor:		   427	    427
   159		capacitor:		    66	  66-66=    0
   160		input (gate) protector:	     8	    8
   162		sum:			  2308		 2176
```
Transistor count not found (rg returned no matches)
- `rg -n "transistor count|count.*transistor|transistor.*count" docs/evidence/ocr/mcs4_users_manual.txt`
- `rg -n "transistor count|count.*transistor|transistor.*count" docs/evidence/ocr/mcs40_users_manual.txt`
- `rg -n "transistor count|count.*transistor|transistor.*count" docs/evidence/ocr/mcs40_advance_specs.txt`
- `rg -n "transistor" docs/evidence/ocr/4004-datasheet.txt docs/evidence/ocr/4040-datasheet.txt`

Context where "transistor" appears without counts
- MCS-4 Users Manual: docs/evidence/ocr/mcs4_users_manual.txt
```text
  4029	The MCS-4 system is designed to interface with all types
  4030	of terminal devices. Interface with teletype is a typical
  4031	example. The interface consists of three simple transistor
  4032	circuits which is shown in Fig. 15 One transistor is used for
  4033	receiving serial data from the teletype, one for transmitting
```
- MCS-40 Users Manual: docs/evidence/ocr/mcs40_users_manual.txt
```text
  6093	Figure 3-2. Flow Chart for Teletype Interfaces.
  6094	example. The interface consists of three simple transistor
  6095	circuits. (See figure below.) One transistor is used for receiv-
  6096	ing serial data from the teletype, one for transmitting data
```
