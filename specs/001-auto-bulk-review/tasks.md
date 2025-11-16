# Tasks: Auto Bulk Review On Launch

**Input**: Design documents from `/specs/001-auto-bulk-review/`  
**Prerequisites**: spec.md (user stories), existing Prompt Sage TUI codebase (Rust 1.79+).

**Implementation Notes**: Extend the Prompt Sage TUI (Ratuit/Crossterm) with an automatic review batch triggered at startup, backed by the existing Tokio worker queue and provider adapters.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Ensure repo metadata and docs reflect the new automation feature.

- [X] T001 Update feature overview + branch entry in `specs/README.md` (or main docs) describing the auto bulk review capability.  
- [X] T002 Document configuration defaults (batch size, enable flag) in `specs/001-auto-bulk-review/spec.md` assumptions or quickstart section.  

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared plumbing that the auto-review feature and overlay depend on.

- [X] T003 Add auto-review configuration structs + defaults (enabled flag, batch size) in `src/config/mod.rs` and wire into persisted filters/settings.  
- [X] T004 Extend `src/app/state.rs` to track “startup batch” metadata (list of candidate prompt IDs, completion flags).  
- [X] T005 Add worker telemetry events for batch start/completion/cancellation in `src/telemetry/mod.rs`.  

---

## Phase 3: User Story 1 – Auto-launch bulk review (Priority: P1) 🎯 MVP

**Goal**: On startup, identify newest unreviewed prompts (up to N) and queue them once per session.

**Independent Test**: Launch the TUI with ≥10 unreviewed prompts; verify the newest N are queued automatically exactly once, skipping already-reviewed prompts.

### Implementation Tasks

- [X] T006 [US1] Build prompt selection helper that returns newest unreviewed prompts per provider in `src/app/state.rs`.  
- [X] T007 [US1] Trigger auto-batch after indexing completes in `src/tui/app.rs` (or startup flow), guarding against reruns per session.  
- [X] T008 [US1] Persist “auto batch completed” state to avoid reprocessing when reopening without new prompts in `src/app/state.rs`.  
- [X] T009 [US1] Log summary of auto batch (count queued/skipped) to the footer/status line in `src/tui/layout.rs`.  

---

## Phase 4: User Story 2 – Show floating progress overlay (Priority: P1)

**Goal**: Provide a non-blocking overlay summarizing batch progress (queued, running, completed, failures, elapsed time).

**Independent Test**: During auto review, open the overlay and confirm counts/time update live; closing the overlay does not stop reviews.

### Implementation Tasks

- [ ] T010 [US2] Add overlay state struct (visibility, counts) and timer hooks in `src/tui/app.rs`.  
- [ ] T011 [US2] Render modal overlay with progress bars/table in `src/tui/layout.rs` using Ratatui widgets.  
- [ ] T012 [US2] Feed worker events into overlay state updates (queued→running→completed/failure) via `src/worker/review_queue.rs` notifications.  
- [ ] T013 [US2] Provide keybinding/help text for toggling overlay visibility in footer + docs (`src/tui/layout.rs`, `specs/001-auto-bulk-review/spec.md`).  

---

## Phase 5: User Story 3 – Control the auto batch (Priority: P2)

**Goal**: Allow users to cancel/pause the auto batch to conserve credits; ensure cancellation stops queueing remaining prompts.

**Independent Test**: Start auto batch, cancel via overlay control, and confirm remaining prompts are marked “not processed” and not requeued during the same session.

### Implementation Tasks

- [ ] T014 [US3] Add overlay actions (cancel/confirm) with input handling in `src/tui/app.rs`.  
- [ ] T015 [US3] Implement cancellation path in `src/worker/review_queue.rs` to drop queued jobs and stop scheduling new ones.  
- [ ] T016 [US3] Mark prompts skipped by cancellation with a temporary status in `src/app/state.rs` and surface message in `src/tui/layout.rs`.  
- [ ] T017 [US3] Update quickstart/help docs in `specs/001-auto-bulk-review/spec.md` (or new quickstart) describing cancel/pause behavior.  

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Safety, documentation, and regression coverage for the new automation.

- [ ] T018 [P] Add unit tests for auto-batch selection logic in `tests/app_state_auto_batch.rs`.  
- [ ] T019 [P] Write integration test simulating worker events to ensure overlay counts stay consistent in `tests/worker_auto_batch.rs`.  
- [ ] T020 Ensure auto batch respects read-only/safe mode toggles, updating `specs/001-auto-bulk-review/spec.md` assumptions and `src/config/mod.rs`.  
- [ ] T021 Update `SUCCESS.md` or release notes summarizing the auto-review feature and verification steps.  

---

## Dependencies & Execution Order

1. **Setup** → align docs/feature notes.  
2. **Foundational** → adds config/state support required by every story.  
3. **US1** → relies on Foundational; delivers MVP auto batch.  
4. **US2** → depends on Foundational + US1 worker events to show progress.  
5. **US3** → depends on US1 queueing and US2 overlay to control the batch.  
6. **Polish** → after stories are functional.  

**Graph**: Foundational → US1 → {US2 → US3}. Polish waits on preceding stories.

---

## Parallel Execution Examples

- T003 (config) can run in parallel with T004 (state) once Setup completes; T005 (telemetry) depends on both.  
- Within US1, T006 (selector) and T007 (trigger) can run concurrently; T008 depends on T006; T009 depends on T007.  
- US2 overlay rendering (T011) can proceed in parallel with state updates (T010); wiring worker events (T012) depends on T010; keybindings (T013) depend on overlay existing.  
- Polish tests (T018, T019) can run concurrently after main stories are implemented.  

---

## Implementation Strategy

1. Deliver US1 (auto batch) as the MVP; confirm batch completes and logs summary automatically.  
2. Layer the overlay (US2) for transparency; ensure it’s non-blocking.  
3. Add cancellation controls (US3) to let power users manage consumption.  
4. Finish with targeted tests/docs (Phase 6) to guard against regressions.  
