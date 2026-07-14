# Factory Test Fail Action Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply per-run factory-test failure policy immediately after each enabled test item and expose partial failure results to the UI.

**Architecture:** Add a reusable single-item evaluator in the threshold module, then make the factory runner build a configuration-aware step list and funnel every exit through one finalization path. Keep the current Tauri event and command surface, extending progress payload usage rather than adding a parallel API.

**Tech Stack:** Rust, Tokio, Tauri EventBus, React, TypeScript, Zustand, Vitest

---

### Task 1: Single-item threshold evaluation

**Files:**
- Modify: `src-tauri/src/gh3036/threshold_config.rs`

- [x] Add `evaluate_test_item` using the existing `TestEvaluationResult::evaluate_channels` logic.
- [x] Add unit tests for enabled pass, enabled fail, and disabled results.
- [x] Run `cargo test gh3036::threshold_config` and confirm all cases pass.

### Task 2: Configuration-aware execution policy

**Files:**
- Modify: `src-tauri/src/gh3036/factory_test.rs`

- [x] Add pure helpers that map steps to configured test items and decide whether a failed item stops the run.
- [x] Add unit tests proving disabled tests are skipped and `Stop`/`Continue` differ.
- [x] Build the runtime step list from enabled items, skipping environment switching when `LPLCTR` is disabled.
- [x] Evaluate each collected item immediately and append it to the cumulative evaluation result.

### Task 3: Unified finalization and progress events

**Files:**
- Modify: `src-tauri/src/gh3036/factory_test.rs`

- [x] Publish each returned `FactoryTestStepResult` in `FactoryTestProgressEvent.step_result`.
- [x] Route threshold stops and execution errors through cleanup and final result persistence.
- [x] Generate error codes from partial evaluations, save CSV, store result/evaluation state, and publish `Failed` with the exact reason.
- [x] Keep `Completed` for full runs, including full runs whose overall judgment is `FAIL` under `continue`.

### Task 4: Failed-result frontend synchronization

**Files:**
- Modify: `src/stores/gh3036Store.ts`
- Create: `src/stores/factoryTestState.test.ts`

- [x] Extract a small terminal-status predicate used by the event handler.
- [x] Fetch the final result for both `completed` and `failed` events.
- [x] Test terminal-status handling for completed, failed, running, and stopped states.

### Task 5: Verification

**Files:**
- Modify: only files above if verification finds task-related issues.

- [x] Run focused Rust unit tests.
- [x] Run frontend unit tests and TypeScript type checking.
- [x] Run Rust formatting and inspect the final diff for unrelated changes.
