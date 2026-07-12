# Security Dependency Policy

The Rust dependency gate has two required commands:

~~~sh
cargo deny check advisories --config deny.toml
python3 scripts/verify_advisory_exceptions.py
~~~

The first command enforces the Cargo advisory policy. The second command reads
the live `cargo audit --json` result and rejects every advisory that is not in
`security/advisory-exceptions.toml`. It also rejects a stale exception, a
missing deny-policy entry, an expired exception, or a package/category mismatch.

The exception registry is an accountable debt record, not an allowlist. Each
entry names the dependency path, risk, mitigation, review condition, and expiry
date. The current registry expires on 2026-10-01. A dependency update or graph
reduction must remove each entry before that date.

`cargo audit` still reports the registered advisories. Passing the policy gate
does not mean the dependency graph is free of advisories, and it does not
replace code review, input-boundary tests, hardware safety review, or license
review.

## Current dependency boundary

The GUI dependency graph retains `quick-xml` 0.38.4 through Wayland scanner
and accessibility XML dependencies. The GUI does not accept application XML
input, which narrows exposure but does not remove the upstream defect. The
registry records the exact paths and requires a compatible GUI-stack update
when one permits `quick-xml` 0.41.0 or later.

The graph also retains unmaintained `paste` and `ttf-parser` dependencies. The
registry treats them separately from vulnerabilities and requires a future
dependency replacement or removal. Run the two commands above after any lockfile
change so the registry, `deny.toml`, and live audit remain synchronized.
