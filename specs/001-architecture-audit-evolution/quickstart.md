# Quickstart & Verification Guide: TTAgy Architecture Evolution

**Feature**: [`specs/001-architecture-audit-evolution`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/001-architecture-audit-evolution/spec.md)
**Status**: `Ready for Verification`
**Created**: 2026-08-24

---

## 1. Prerequisites

- **Rust**: `cargo 1.80+` (edition 2021)
- **Node.js**: `node 18.0.0+`
- **Python**: `python 3.10+`
- **Antigravity CLI**: `agy` binary in `$PATH` or standard candidate paths (`$HOME/.local/bin/agy`)

---

## 2. All-in-One Automated Quality Gate

Run the local quality gate encompassing contract schema validation, Rust workspace tests, TypeScript client tests, Python SDK tests, and release build checks:

```bash
bash scripts/local-ci.sh
```

---

## 3. Targeted Scenario Verification

### Scenario A: Verify Stderr Async Drainage & Deadlock Elimination
Run the test that simulates a child process streaming >1MB of stderr diagnostic data while sending NDJSON tokens on stdout:
```bash
cargo test -p ttagy-core --test test_core test_stderr_large_buffer_drain
cargo test -p ttagyd --test consumer_compat test_heavy_stderr_non_blocking
```

### Scenario B: Verify Unix Domain Socket (UDS) & TCP Dual-Mode Server
1. Start `ttagyd` with UDS and TCP binding:
   ```bash
   cargo run -p ttagyd -- --socket /tmp/ttagy.sock --port 8970 --concurrency 4
   ```
2. Verify UDS communication via curl/socat or test client:
   ```bash
   curl --unix-socket /tmp/ttagy.sock http://localhost/api/v1/health
   # Expected: {"status":"ok","version":"v1","transport":"dual_uds_tcp",...}
   ```

### Scenario C: Verify Connection Drop & Immediate Process Tree Termination
1. Run integration test verifying semaphore permit release and SIGKILL dispatch upon stream drop:
   ```bash
   cargo test -p ttagyd --test consumer_compat test_client_abort_instant_kill
   ```
2. Run TypeScript AbortSignal test:
   ```bash
   cd packages/ttagy-client && node --test src/__tests__/client.test.mjs
   ```

### Scenario D: Verify Model Passthrough & Elimination of Lossy Matching
Run unit test ensuring `claude-3.7-sonnet` and custom models are passed through without silent alteration:
```bash
cargo test -p ttagy-core --test test_core test_model_resolution_and_passthrough
```

### Scenario E: Verify Python SDK Parity & Full Client Functionality
Run Python SDK test suite:
```bash
python3 -m unittest discover -s python/tests
```
