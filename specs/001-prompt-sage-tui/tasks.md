# Tasks: Prompt Sage TUI – Prompt History Review

**Input**: Design documents from `/specs/001-prompt-sage-tui/`  
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Implementation Notes**: Rust 1.79+ single binary using Ratatui + Crossterm for TUI, Tokio worker queue for provider CLI calls, local JSONL indexing for Claude/Codex logs.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish project skeleton, dependencies, and developer ergonomics.

- [X] T001 Align crate metadata and dependency table in `Cargo.toml` (tokio, ratatui, crossterm, serde, glob, clap, anyhow, dirs).  
- [X] T002 Scaffold binary entry and module declarations in `src/main.rs` (cli, indexing, model, providers, tui, worker).  
- [X] T003 Document developer prerequisites and quickstart in `specs/001-prompt-sage-tui/quickstart.md` (Rust toolchain, Claude/Codex CLIs).  

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data contracts, configuration, indexing, and worker plumbing that every story depends on.

- [X] T004 Implement configuration loader with source discovery and filter persistence in `src/config/mod.rs`.  
- [X] T005 Define shared data models (PromptSource, ProjectSession, PromptRecord, PromptReview, ReviewJob) in `src/model/mod.rs`.  
- [X] T006 Build transcript indexers plus summary reporting in `src/indexing/{mod.rs,claude.rs,codex.rs}`.  
- [X] T007 Implement async review queue with job state machine and persistence hooks in `src/worker/review_queue.rs`.  
- [X] T008 Create provider adapters for Claude and Codex CLIs plus JSON contract parser in `src/providers/{claude.rs,codex.rs,mod.rs}` and `specs/001-prompt-sage-tui/contracts/review.json`.  
- [X] T009 Establish telemetry/logging helpers for indexing and worker events in `src/telemetry/mod.rs`.  

---

## Phase 3: User Story 1 – Review a prompt for a project (Priority: P1) 🎯 MVP

**Goal**: Let users browse prompts by project/date, request a review, and view improved prompt + explanation without modifying files.

**Independent Test**: Launch TUI with valid Claude/Codex logs, select a prompt, trigger review, and verify original vs improved prompt/explanation panes populate while logs remain read-only.

### Implementation Tasks

- [X] T010 [US1] Implement prompt list + detail layout in `src/tui/layout.rs` (projects row, prompt table, dual-pane detail).  
- [X] T011 [US1] Build application state + selection handling (projects, prompts, reviews) in `src/app/state.rs`.  
- [X] T012 [US1] Wire keyboard navigation, prompt selection, and review triggers in `src/tui/app.rs`.  
- [X] T013 [US1] Render original/improved prompt panes and explanation formatting in `src/tui/detail.rs`.  
- [X] T014 [US1] Implement review enqueue action + job status integration in `src/tui/actions.rs` and `src/worker/review_queue.rs`.  
- [X] T015 [US1] Add clipboard copy of improved prompt from selection in `src/tui/export.rs`.  

---

## Phase 4: User Story 2 – Scan history for improvement opportunities (Priority: P2)

**Goal**: Provide filtering/search, scrolling, and inspection tools to identify candidates rapidly.

**Independent Test**: Load multiple days of prompts, filter by provider/date/text, move through results with keyboard, and see prompt details update instantly.

### Implementation Tasks

- [X] T016 [US2] Implement filter state (provider, date range, status, text) and persistence wiring in `src/app/filter.rs` and `src/config/mod.rs`.  
- [X] T017 [US2] Render filter bar UI with search/filter modes plus keybindings in `src/tui/filter_bar.rs`.  
- [X] T018 [US2] Add provider/date/text filter controls + hide-reviewed toggle in `src/tui/app.rs`.  
- [X] T019 [US2] Ensure prompt list paging + selection updates refresh detail panes in `src/tui/layout.rs`.  
- [X] T020 [US2] Surface worker/job counters and selection count in header/footer status bars in `src/tui/layout.rs`.  

---

## Phase 5: User Story 3 – Export improved prompts for reuse (Priority: P3)

**Goal**: Allow users to select prompts and export improved prompt + explanation to shareable artifacts without rerunning reviews.

**Independent Test**: With reviewed prompts available, select one or more, invoke export, and verify a text/Markdown file is saved plus clipboard copy works without hitting providers again.

### Implementation Tasks

- [X] T021 [US3] Implement export payload builder (Markdown/text with improved prompt + explanation) in `src/export/mod.rs`.  
- [X] T022 [US3] Add selection toggling + bulk selection helpers in `src/app/state.rs`.  
- [X] T023 [US3] Provide save-to-file workflow with status messaging in `src/tui/app.rs` and `src/tui/export.rs`.  
- [X] T024 [US3] Include tags/metadata in export diff popup and selection preview in `src/tui/detail.rs` and `src/tui/export.rs`.  

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Quality, safety, and documentation refinements across stories.

- [ ] T025 [P] Add structured logging + telemetry emitters for indexing, filtering, and review flows in `src/telemetry/mod.rs`.  
- [ ] T026 [P] Harden error handling and user-facing status messages in `src/tui/app.rs` and `src/worker/review_queue.rs`.  
- [ ] T027 Update documentation (`README.md`, `specs/001-prompt-sage-tui/quickstart.md`) with filtering/export instructions.  
- [ ] T028 Run `cargo fmt`, `cargo clippy`, and `cargo test` plus document results in `SUCCESS.md`.  

---

## Dependencies & Execution Order

1. **Setup (Phase 1)** → establishes workspace + docs.  
2. **Foundational (Phase 2)** → requires Setup; blocks all user stories.  
3. **User Story 1 (Phase 3)** → depends on Foundational; delivers MVP.  
4. **User Story 2 (Phase 4)** → depends on Foundational; can run in parallel with US1 once shared filters wired.  
5. **User Story 3 (Phase 5)** → depends on Foundational and US1 review data structures.  
6. **Polish (Phase 6)** → after desired stories complete.  

**Story Dependency Graph**: US1 → {US2, US3}; US2 and US3 independent after US1 data paths exist.

---

## Parallel Execution Examples

- **Setup**: T001 and T003 can run in parallel once repository exists; T002 depends on T001.  
- **Foundational**: T004 (config) can run in parallel with T005 (models); T006 (indexers) waits on T005; T007/T008 depend on T005 as well but can run concurrently with T006.  
- **User Story 1**: T010 (layout) and T011 (state) can run in parallel; T012 depends on both; T013 depends on T010; T014 depends on worker from Phase 2; T015 depends on T012.  
- **User Story 2**: T016 and T017 parallel; T018 depends on both; T019/T020 depend on layout from T010 but can run concurrently after T018.  
- **User Story 3**: T021 independent; T022 relies on state updates (T011); T023 depends on T021+T022; T024 depends on T021.  

---

## Implementation Strategy

1. **MVP (US1)**: Complete Setup → Foundational → Phase 3 tasks; verify review workflow end-to-end before proceeding.  
2. **Incremental Enhancements**: Layer US2 filters/search to improve discovery; deliver US3 export after users can browse+review reliably.  
3. **Polish**: Once all priority stories are complete, focus on telemetry, error handling, documentation, and CI hygiene.  
4. **Testing cadence**: Run `cargo test` + manual TUI checks after each phase; confirm overlay/worker status remains responsive.  

Total tasks: 28 (Setup:3, Foundational:6, US1:6, US2:5, US3:4, Polish:4).  
Suggested MVP scope: Complete through Phase 3 (User Story 1).  
