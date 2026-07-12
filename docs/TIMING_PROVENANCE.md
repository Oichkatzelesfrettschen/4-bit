# Timing Parameter Provenance

`docs/evidence/timing_parameters.json` is the machine-checkable timing
parameter ledger. It binds every active MCS-4 logical-clock parameter to an
OCR locator, a value interval in picoseconds, one or more code use sites, a
limitation, and a falsifier.

Run the ledger gate with:

~~~sh
python3 scripts/verify_timing_parameters.py
~~~

The ledger uses the Intel 4004 datasheet OCR table at lines 752 through 757.
OCR is retained evidence, not a replacement for the original scan. The code
uses the documented clock-period and phase-window bounds for deterministic
logical timing. It does not model device-level propagation delay, package
loading, clock skew, or a target FPGA clock implementation.

The selected `TCY_TYP` value is 1.35 us, the lower bound of the retained 1.35
to 2.0 us range. This selection makes the emulator deterministic; it does not
claim that every physical part operates at that period. A timing claim becomes
stronger only when a measured or calibrated model records its workload, source
conditions, and comparison result.
