# Quickstart: Deterministic Testing & Chaos Suite

**Feature**: [`specs/002-test-infrastructure-and-chaos-suite`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/002-test-infrastructure-and-chaos-suite/spec.md)
**Status**: `Ready for Verification`
**Created**: 2026-08-24

---

## 1. Quick Execution of All Testing Tiers

Run the full local CI suite encompassing all 5 quality tiers plus CTS and Chaos tests:

```bash
bash scripts/local-ci.sh
```

---

## 2. Targeted Testing Commands

### Scenario 1: Run `mock-agy` CLI directly
```bash
cargo run -p mock-agy -- --scenario stream_normal
cargo run -p mock-agy -- --scenario stderr_flood
```

### Scenario 2: Run Chaos Suite (10MB Stderr & 100x Abort Race)
```bash
cargo test -p ttagyd --test chaos_suite
```

### Scenario 3: Run Cross-Language Conformance Suite (CTS)
```bash
# Rust Conformance
cargo test -p ttagy-client --test conformance_test

# TypeScript Conformance
cd packages/ttagy-client && node --test src/__tests__/conformance.test.mjs

# Python Conformance
PYTHONPATH=python python3 -m unittest discover -s python/tests
```
