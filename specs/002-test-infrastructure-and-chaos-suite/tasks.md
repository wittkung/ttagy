# Tasks: Deterministic Testing Infrastructure & Chaos Suite (CTS)

**Feature**: [`specs/002-test-infrastructure-and-chaos-suite`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/002-test-infrastructure-and-chaos-suite/spec.md)
**Plan**: [`plan.md`](./plan.md)
**Status**: `Completed`

---

## Phase 1: Mock CLI Binary (`crates/mock-agy`)

- [X] T001 Add `crates/mock-agy` to workspace root `Cargo.toml`
- [X] T002 [P] Create `crates/mock-agy/Cargo.toml`
- [X] T003 Implement scenario dispatcher (`stream_normal`, `stderr_flood`, `malformed_ndjson`, `abort_hang`, `quota_error`, `empty_output`) in `crates/mock-agy/src/main.rs`

---

## Phase 2: Chaos & Boundary Stress Suite

- [X] T004 Implement 10MB Stderr flood non-blocking test in `crates/ttagyd/tests/chaos_suite.rs`
- [X] T005 [US1] Implement 100x rapid abort race test asserting 0 orphan processes and 100% permit recovery in `crates/ttagyd/tests/chaos_suite.rs`
- [X] T006 [US1] Implement sandbox zero-leakage assertion in `crates/ttagyd/tests/chaos_suite.rs`

---

## Phase 3: Cross-Language Conformance Suite (CTS)

- [X] T007 [P] Create shared JSON conformance fixtures under `tests/conformance/fixtures/`
- [X] T008 [US2] Implement Rust CTS runner in `crates/ttagy-client/tests/conformance_test.rs`
- [X] T009 [P] [US2] Implement TypeScript CTS runner in `packages/ttagy-client/src/__tests__/conformance.test.mjs`
- [X] T010 [P] [US2] Implement Python CTS runner in `python/tests/test_conformance.py`

---

## Phase 4: CI/CD Quality Gate & Verification

- [X] T011 Update `scripts/local-ci.sh` to include `mock-agy` build, Chaos suite, and CTS execution
- [X] T012 Run `bash scripts/local-ci.sh` and verify all tests pass in $\le 5\text{s}$ with 0 cloud tokens consumed
