# Development Guide

## Tools
- cargo, clippy, rustfmt.
- Facebook Infer: infer run -o mcs4-emu/infer-out.
- TLA+: place specs in mcs4-emu/spec and verify with tla2tools.
- CMake: for optional native tooling, keep separate.

## Automation
- scripts/build_time.sh, scripts/bench.sh for baseline.
- Keep STATUS.md and docs/ROADMAP.md synchronized.
