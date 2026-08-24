# Tasks: Omni-Language SDK Ecosystem

**Feature**: [`specs/003-omni-language-sdk-ecosystem`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/003-omni-language-sdk-ecosystem/spec.md)
**Plan**: [`plan.md`](./plan.md)
**Status**: `Completed & Verified`

---

## Phase 1: C-ABI Native FFI Library (`crates/ttagy-ffi`)

- [x] T001 Add `crates/ttagy-ffi` to workspace root `Cargo.toml`
- [x] T002 [P] Create `crates/ttagy-ffi/Cargo.toml` with `cdylib` and `rlib` crate-types
- [x] T003 Implement C-ABI export symbols (`ttagy_client_create`, `ttagy_client_free`, `ttagy_client_chat`, `ttagy_string_free`, `ttagy_last_error_message`) in `crates/ttagy-ffi/src/lib.rs`
- [x] T004 [P] Create C header `crates/ttagy-ffi/include/ttagy.h` and `crates/ttagy-ffi/include/module.modulemap`
- [x] T005 Add C FFI integration test in `crates/ttagy-ffi/tests/test_c_ffi.rs`

---

## Phase 2: Go Native SDK (`golang/ttagy`)

- [x] T006 [P] Create Go module `golang/ttagy/go.mod`
- [x] T007 [P] Implement `golang/ttagy/types.go`
- [x] T008 [P] Implement `golang/ttagy/parser.go`
- [x] T009 Implement `golang/ttagy/client.go` supporting UDS, TCP SSE, and local fallback
- [x] T010 Add Go unit and CTS tests in `golang/ttagy/client_test.go`

---

## Phase 3: Dart / Flutter SDK (`dart/ttagy`)

- [x] T011 [P] Create Dart package `dart/ttagy/pubspec.yaml`
- [x] T012 [P] Implement `dart/ttagy/lib/src/types.dart`
- [x] T013 [P] Implement `dart/ttagy/lib/src/parser.dart`
- [x] T014 Implement `dart/ttagy/lib/src/client.dart` and `dart/ttagy/lib/ttagy.dart`
- [x] T015 Add Dart unit and CTS tests in `dart/ttagy/test/conformance_test.dart`

---

## Phase 4: JVM & .NET SDKs

- [x] T016 [P] Implement Java 17+ / Kotlin SDK in `jvm/ttagy/`
- [x] T017 [P] Implement C# / .NET 8+ SDK in `dotnet/ttagy/`

---

## Phase 5: CI/CD Quality Gate & Verification

- [x] T018 Update `scripts/local-ci.sh` to include C-FFI and Go CTS tests
- [x] T019 Run `bash scripts/local-ci.sh` and ensure 100% PASS with 0 cloud quota consumed
