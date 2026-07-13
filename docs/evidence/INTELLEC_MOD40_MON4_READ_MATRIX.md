# Intellec 4 MOD 40 MON4 Public Read Matrix

## Result

Public material documents at least three physical-read efforts for four
C1702A monitor devices: Kyle Owen in 2013, Herb Johnson in 2013, and Sid
Jones in 2022 through 2023. This falsifies the earlier claim that no
independently documented physical read had been located.

The material does not meet the repository acceptance gate. It does not
provide two repeatable, machine-readable raw 256-byte acquisitions for each
device in one revision-identified, socket-identified set. It therefore does
not authorize historical monitor execution, a source-faithful FPGA wrapper, or
an equivalence trace.

The evidence source is a secondary reconstruction page, retained locally at
`docs/evidence/local_sources/intellec/retrotechnology_mon4_20260713/`.
The retained page and artifacts remain local-only until redistribution terms
are established.

## Acquisition evidence

| Acquisition | Device evidence | Reader and handling | Published byte artifact | Limitation |
| --- | --- | --- | --- | --- |
| Kyle Owen, August 2013 | Photograph and labels `MON 4 V2.1 000`, `100`, `200`, `300` | Prolog 980, serial adapter, Intel HEX output | `krom0.hex` through `krom3.hex` | One published read per label. `000` decodes to 256 `FF` bytes. No repeated raw acquisition is published. |
| Herb Johnson, 2013 | Removed cards and labels reported as ROM 0 through ROM 3 | Prolog Series 90; values manually entered into a Windows 95 laptop | `hrom0.txt` through `hrom3.txt` | Manual transcription is not a preserved reader output. ROM 1 through ROM 3 labels differ from Kyle's V2.1 labels and may contain patches. |
| Sid Jones, 2022-2023 | Four monitor devices reported as `000`, `100`, `200`, `300` | Homemade Mk I and Mk II 1702A readers | None located | The reported reads have no published raw bytes, digest, device photograph, or repeat-read log. |

The source reports that all three efforts observe inverted stored bytes. That
is a strong cross-check for a bitwise complement candidate, not primary proof
of the full electrical data path or socket order.

## Artifact digests

The following values identify exactly the retained public files. `Raw` means
the decoded published file bytes. `Complemented` means each raw byte XOR
`FF`. It is a derived candidate execution image, not accepted firmware.

| Label | Acquisition | Raw CRC32 | Raw SHA-1 | Raw SHA-256 | Complemented CRC32 | Complemented SHA-1 |
| --- | --- | --- | --- | --- | --- |
| 000 | Kyle | `fea8a821` | `c744cac6af7621524fc3a2b0a9a135a32b33c81b` | `3d6876a0146de8576eb2395a858de1213d1b92c65b779df3a331cfd5a4584546` | `0d968558` | `b376885ac8452b6cbf9ced81b1080bfd570d9b91` |
| 100 | Kyle | `95680736` | `a245916e8f0a38b7f1b4460886a738ef388405a1` | `7b8a4793e8294afeb9e5275742d220b4be896b474a0289c620fa4d2d484fdc86` | `66562a4f` | `040749c45e95dfc39b3397d0c31c8b4c11f0a5fc` |
| 200 | Kyle | `0d3db111` | `1270ce2c29070e3ee9a7cc229f8f1ce16c797e15` | `5a1f3e556772f12b365e8d1041df2abf2a5ad45ab18eafaf6a82efac92ccd467` | `fe039c68` | `1801cfcc7514412865c0fdc7d1800fcf583a2d2a` |
| 300 | Kyle | `c41af8d6` | `0a6f272bf6e948c4117a7fffaf56a3117d3816c8` | `4dab5b59d54acaabd8243b558cbd8f7bc0641159919581b2612450adb193aba4` | `3724d5af` | `b764b3bb3541fbda875f7a7655f46aa54b332631` |
| 000 | Herb | `f936f544` | `7f005b9933cd0ce4c9e8580e124a46e6164d45e6` | `7fee9358b3f81a384c143cb19b678ccf69fef192ce6472a03627d0c70c1fe860` | `0a08d83d` | `e7044f3b54a16d925aaffc85b0c6001f740ea4c5` |
| 100 | Herb | `3ea1c1af` | `0b48cbd235ca85cd06f924b8c4e28f8525df69b5` | `9368cf48595ea1ceaa558d149f5e6a73e6b886e58507f5a93c8180c4d8415664` | `cd9fecd6` | `9c4fb85118c881687fd4b324e5089df05d1e63d1` |
| 200 | Herb | `f043cc51` | `55e62c72ca6abcd34690f846d73f13db534b84da` | `2eab6a0904dd0b418a481afcd7211c740ee9c28a7f854ae53734227b3049e420` | `037de128` | `3694636e1f4e23688b36ea9ee755a0c5888f4328` |
| 300 | Herb | `0027a000` | `8070da1e44c57a8f60c7785bbf0625731d133ff3` | `a37011fce787bb68564500288bcefc94e836993721291cf793f5cf225ce72e39` | `f3198d79` | `b7903073b69f487b6f78842c08694f12225d85f0` |

The complemented Kyle `100`, `200`, and `300` SHA-1 values match the
long-standing public MAME candidate declarations. That agreement establishes
public-artifact lineage only. It does not establish physical provenance,
socket identity, or a canonical revision.

## Cross-acquisition comparison

| Logical label | Kyle versus Herb raw differences | Interpretation |
| --- | ---: | --- |
| 000 | 252 | Kyle is all `FF`; Herb is not. The artifacts cannot both be accepted as the same valid physical content. |
| 100 | 36 | The labels and revisions differ. The source reports possible modifications, patches, and manual transcription. |
| 200 | 4 | A close but non-identical result. The mismatch remains unresolved without repeated raw reads. |
| 300 | 2 | A close but non-identical result. The mismatch remains unresolved without repeated raw reads. |

No byte is repaired, selected, or normalized by majority vote. A mismatch is
evidence of an unresolved hardware, revision, acquisition, or transcription
conflict.

## Socket order and polarity status

The package labels support logical 256-byte address blocks `000`, `100`,
`200`, and `300`. The source photograph places those labeled packages in
left-to-right order. This is not a socket map. Only the primary imm4-43 sheet
can bind each logical address block to A1 through A4 and establish every
signal inversion from C1702A data pins through the CPU memory path.

The documented complement relation is:

```text
candidate_execution_byte = raw_reader_byte XOR 0xff
```

This relation remains `secondary-documented`. It becomes a source-bound
transform only when the primary schematic trace identifies the data path,
polarity, and socket order, and two accepted raw acquisitions agree for every
device.

## Remaining acceptance work

1. Photograph each physical device in socket, record A1 through A4, and retain
   the original label and board identity.
2. Read each device twice with preserved reader output, voltage settings,
   adapter description, tool version, and error log.
3. Repeat with a separately documented acquisition chain and record raw CRC32,
   SHA-1, and SHA-256 values for every acquisition.
4. Trace the imm4-43 primary schematic from each C1702A output through the
   4289 and associated TTL to establish socket order and active polarity.
5. Compare the two raw sets byte-for-byte before applying the primary-backed
   transform and before deriving a 1024-byte monitor image.

Until all five items close, `monitor_media_verified` remains false and every
historical MOD 40 execution path remains blocked.
