# Documentation Quality Checklist
- README up-to-date
- Build/run instructions
- Architecture documented
- API documented with examples
- Configuration documented
- Troubleshooting
- Roadmap synced with STATUS
- Warnings treated as errors (build, test, lint)
- Automation scripts pass
- Clean script removes artifacts (target/coverage)

## Latest Verification
- 2026-01-06: `cargo fmt --all -- --check`
- 2026-01-06: `cargo clippy-all`
- 2026-01-06: `cargo test --workspace --locked`
- 2026-01-06: `scripts/doc_validate.sh`
