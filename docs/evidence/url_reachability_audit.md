# URL Reachability Audit

Date: 2026-02-25 (rechecked 2026-02-25)
Method: HTTP HEAD request with curl, 15-second timeout, follow redirects.

## PDF Source URLs (critical)

| # | URL | Status | Notes |
|---|-----|--------|-------|
| 1 | `datasheets.chipdb.org/Intel/MCS-4/datashts/intel-4004.pdf` | 200 | Verified |
| 2 | `datasheets.chipdb.org/Intel/MCS-40/4040.pdf` | 200 | Verified |
| 3 | `bitsavers.org/components/intel/MCS4/MCS4_Data_Sheet_Nov71.pdf` | 200 | Verified |
| 4 | `bitsavers.org/components/intel/MCS4/MCS-4_UsersManual_Feb73.pdf` | 200 | Verified |
| 5 | `bitsavers.org/components/intel/MCS4/MCS-4_Assembly_Language_Programming_Manual_Dec73.pdf` | 200 | Verified |
| 6 | `bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf` | 200 | Verified |
| 7 | `bitsavers.org/components/intel/MCS40/MCS_40_Microcomputer_Set_Advance_Specifications_Sep74.pdf` | 200 | Verified |
| 8 | `bitsavers.org/components/intel/_dataBooks/1975_Intel_Data_Catalog.pdf` | 200 | Verified 2026-02-25 |
| 9 | `www.4004.com/assets/i400x_analyzer_repacked_20221111.zip` | 200 | Verified |

## Dead chipdb.org URLs (removed from manifests)

These URLs were previously in source_manifest.json but returned 404. They have been replaced
with bitsavers equivalents in the current manifests.

| URL | Status | Replacement |
|-----|--------|-------------|
| `chipdb.org/Intel/MCS-4/datashts/intel-4040.pdf` | 404 | `chipdb.org/Intel/MCS-40/4040.pdf` |
| `chipdb.org/Intel/MCS-4/datashts/mcs4.pdf` | 404 | `bitsavers.org/.../MCS4_Data_Sheet_Nov71.pdf` |
| `chipdb.org/Intel/MCS-4/mcs4_users_manual.pdf` | 404 | `bitsavers.org/.../MCS-4_UsersManual_Feb73.pdf` |
| `chipdb.org/Intel/MCS-4/mcs40_users_manual.pdf` | 404 | `bitsavers.org/.../MCS-40_Users_Manual_Nov74.pdf` |
| `chipdb.org/Intel/MCS-4/mcs40_advance_specs.pdf` | 404 | `bitsavers.org/.../MCS_40_Microcomputer_Set_Advance_Specifications_Sep74.pdf` |
| `chipdb.org/Intel/MCS-4/1975_catalog_302.pdf` | 404 | `bitsavers.org/.../_dataBooks/1975_Intel_Data_Catalog.pdf` (full catalog) |
| `chipdb.org/Intel/MCS-4/1975_catalog_232-252.pdf` | 404 | (same full catalog) |
| `chipdb.org/Intel/MCS-4/1975_catalog_276-282.pdf` | 404 | (same full catalog) |

## Reference Site URLs (secondary)

| # | URL | Status | Notes |
|---|-----|--------|-------|
| 1 | `www.computerhistory.org/siliconengine/...` | 200 | OK |
| 2 | `uvicrec.blogspot.com/` | 200 | OK |
| 3 | `www.4004.com/` | 200 | Plain HTTP |
| 4 | `pyntel4004.readthedocs.io/` | 200 | OK |
| 5 | `en.wikipedia.org/wiki/Intel_4004` | 200 | OK |
| 6 | `archive.org/details/bitsavers_intelMCS40ReferenceSchematics_81608214` | 200 | OK |
| 7 | `archive.org/details/bitsavers_intelMCS4IroComputerModulesJan74_4532529` | 200 | OK |
| 8 | `www.intel4004.com/4004_original_schematics.htm` | 200 | Plain HTTP |
| 9 | `www.retrotechnology.com/restore/4040_doc.html` | 200 | OK |
| 10 | `www.4004.com/mcs4-masks-schematics-sim.html` | 200 | OK |
| 11 | `github.com/asicerik/j4004` | 200 | OK |
| 12 | `github.com/asicerik/go4004` | 200 | OK |
| 13 | `opencores.org/projects/mcs-4` | 200 | OK |
| 14 | `e4004.szyc.org/` | 200 | Plain HTTP |

## Degraded / Unreachable

| URL | Status | Notes | Wayback Fallback |
|-----|--------|-------|------------------|
| `hackaday.com/2018/06/25/federico-faggin-the-real-silicon-man/` | 404 | Article removed or slug changed. Rechecked 2026-02-25: still 404. | `web.archive.org/web/2023/https://hackaday.com/2018/06/25/federico-faggin-the-real-silicon-man/` (timed out; try in browser) |
| `en.wikichip.org/wiki/intel/mcs-4/4004` | 520 | Origin server error (Cloudflare/Ezoic). Rechecked 2026-02-25: still 520. | `web.archive.org/web/2024/https://en.wikichip.org/wiki/intel/mcs-4/4004` |
| `en.wikichip.org/wiki/intel/mcs-40/4040` | 520 | Same issue. Rechecked 2026-02-25: still 520. | `web.archive.org/web/2024/https://en.wikichip.org/wiki/intel/mcs-40/4040` |
| `siliconprawn.org/map/intel/4004b/` | 403 | Cloudflare bot protection; accessible in browser. Rechecked 2026-02-25: still 403. | N/A (works in browser with JS) |

## Photomicrograph Source URLs

| # | URL | Status | Notes |
|---|-----|--------|-------|
| 1 | `www.4004.com/mcs4-masks-schematics-sim.html` | 200 | CC BY-NC-SA 3.0 |
| 2 | `alumni.media.mit.edu/~mcnerney/2009-4004/` | 200 | License unconfirmed |
| 3 | `commons.wikimedia.org/.../Ic-photo-Intel--P4040...` | 200 | CC BY-SA |
| 4 | `commons.wikimedia.org/.../Chip_layout_...Intel_4004...` | 200 | CC0 1.0 |
| 5 | `happytrees.org/chips/File:Ic-photo-Intel--P4040...` | 200 | CC BY-SA 4.0 |
| 6 | `www.cpu-collection.de/...Intel...4040` | 200 | License unconfirmed |
| 7 | `siliconprawn.org/map/intel/4004b/` | 403 | Bot-protected; license unconfirmed |

## Summary

- Total URLs tested: 39
- Reachable (200): 31 (bitsavers catalog now verified)
- Degraded (403/520): 3
- Unreachable (404): 1 (Wayback fallback added)
- Dead chipdb.org (corrected): 8

All critical PDF source URLs now point to verified bitsavers.org or chipdb.org paths.
The 8 fabricated chipdb.org paths have been replaced with working bitsavers URLs.
Wayback Machine fallback URLs added for degraded/unreachable secondary sources.
