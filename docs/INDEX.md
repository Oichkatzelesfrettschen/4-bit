# Documentation Program (2026-01-07T06:33:37Z)

Objective: Production-grade, registry-driven documentation synchronized with code, covering SIMD
cluster execution, dual UI (CLI/TUI/GUI), and emulator architecture. No toy models; complete,
elegant docs.

MUST DOCUMENT
- Build/run (README)
- Architecture and decisions (ARCHITECTURE.md)
- API interfaces and usage (docs/API.md)
- Configuration and env variables (docs/DEPLOYMENT.md)
- Deploy and operate (docs/DEPLOYMENT.md)
- Breaking changes and migration (docs/CHANGELOG.md)
- Dependencies and requirements (README)
- Installation requirements (mcs4-emu/INSTALLATION.md)

SHOULD DOCUMENT
- Troubleshooting (docs/TROUBLESHOOTING.md)
- Performance tuning and optimization (ARCHITECTURE.md)
- Contributing (docs/CONTRIBUTING.md)
- Limitations and workarounds (ARCHITECTURE.md)
- Testing strategies and coverage (STATUS.md/README)
- Security considerations (docs/DEPLOYMENT.md)
- Audit log and validation status (docs/AUDIT.md)

NICE TO DOCUMENT
- History and rationale (ARCHITECTURE.md)
- Related projects and resources (ARCHITECTURE.md)
- Future plans and roadmap (docs/ROADMAP.md)
- Contributors and acknowledgments (README)
- Benchmarks and performance metrics (docs/CHANGELOG.md)

Information Architecture
- Modular docs; central index (docs/INDEX.md); metadata registry (docs/meta/registry.yaml)
- Cross-links across README, ARCHITECTURE, ROADMAP, STATUS; ToC for long docs
- Automation: link_check.sh, md_lint.sh; CI builds docs, validates links

# Senior Documentation Architect Guidance (2026-01-07T06:33:37Z)

MUST DOCUMENT
- How to build/run (README)
- Architecture and decisions (ARCHITECTURE.md)
- API interfaces and usage (docs/API.md)
- Configuration options and environment variables (docs/DEPLOYMENT.md)
- Deployment and operations (docs/DEPLOYMENT.md)
- Breaking changes and migration (docs/CHANGELOG.md)
- Dependencies and requirements (README)

SHOULD DOCUMENT
- Common issues and troubleshooting (docs/TROUBLESHOOTING.md)
- Performance tuning and optimization (ARCHITECTURE.md)
- Contributing guidelines (docs/CONTRIBUTING.md)
- System limitations and workarounds (ARCHITECTURE.md)
- Testing strategies and coverage (STATUS.md/README)
- Security considerations (docs/DEPLOYMENT.md)

NICE TO DOCUMENT
- Historical context and rationale (ARCHITECTURE.md)
- Related projects and resources (ARCHITECTURE.md)
- Future plans and roadmap (docs/ROADMAP.md)
- Contributors and acknowledgments (README)
- Benchmarks and performance metrics (docs/CHANGELOG.md)

Information Architecture Pattern
- Modular docs with imports; central index; metadata registry (docs/meta/registry.yaml)
- Cross-links and discovery aids; ToC for long documents; indices and search-friendly headings

Metadata Registry (YAML)
- Schema with version and last_updated; programmatic discovery for index generation and validation

Quality Checklist
- Completeness, Accuracy, Discoverability, Maintainability
- Automated link checks, markdown lint, registry validation in CI

Metrics & Anti-Patterns
- Coverage, Freshness (90-day alerts), Link validity, Readability
- Avoid outdated, duplicated, scattered info; centralize and automate

## Registry Snapshot
<!-- DOCS_REGISTRY_START -->
- "README.md"
- "requirements.md"
- "gemini.md"
- "claude.md"
- "ARCHITECTURE.md"
- "mcs4-emu/STATUS.md"
- "mcs4-emu/INSTALLATION.md"
- "mcs4-emu/requirements.md"
- "docs/API.md"
- "docs/DEPLOYMENT.md"
- "docs/CONTRIBUTING.md"
- "docs/TROUBLESHOOTING.md"
- "docs/CHANGELOG.md"
- "docs/ROADMAP.md"
- "docs/TODO.md"
- "docs/INDEX.md"
- "docs/DEVELOPMENT.md"
- "docs/QUALITY-CHECKLIST.md"
- "docs/AUDIT.md"
- "docs/TOOLING_AUDIT.md"
- "docs/CHIP_ARTIFACTS.md"
- "docs/NETLIST_WORKFLOW.md"
- "docs/LAYER_ANNOTATIONS.md"
- "docs/photomicrographs/README.md"
- "docs/evidence/README.md"
- "docs/evidence/DIAGRAM_EXTRACTION.md"
- "docs/evidence/audit_claims_backlog.md"
- "docs/evidence/PRIMARY_SOURCES_BACKLOG.md"
- "docs/evidence/OCR_SIGNAL_LABELS.md"
- "docs/evidence/ANCHOR_COVERAGE_V0.md"
- "docs/evidence/POWER_RAIL_EVIDENCE.md"
- "docs/evidence/photomicrograph_permissions.md"
- "docs/evidence/PROVENANCE_CHECKLIST.md"
- "docs/evidence/ocr_manifest.yaml"
- "docs/evidence/source_manifest.json"
- "docs/evidence/bibliography.bib"
- "docs/evidence/CITATION_GUIDE.md"
- "docs/evidence/url_reachability_audit.md"
- "docs/evidence/ocr_results.md"
<!-- DOCS_REGISTRY_END -->
