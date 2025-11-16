# Feature Specification: Auto Bulk Review On Launch

**Feature Branch**: `001-auto-bulk-review`  
**Created**: 2025-11-15  
**Status**: Draft  
**Input**: User description: "When the app starts, it starts reviewing the most recent prompts in bulk. reviewing only applies to the most recent prompts. If the prompts already reviewed, it does not need to be performed. When reviewing, it pops up a float window or pane to show the progress."

## User Scenarios & Testing *(mandatory)*

<!--
  IMPORTANT: User stories should be PRIORITIZED as user journeys ordered by importance.
  Each user story/journey must be INDEPENDENTLY TESTABLE - meaning if you implement just ONE of them,
  you should still have a viable MVP (Minimum Viable Product) that delivers value.
  
  Assign priorities (P1, P2, P3, etc.) to each story, where P1 is the most critical.
  Think of each story as a standalone slice of functionality that can be:
  - Developed independently
  - Tested independently
  - Deployed independently
  - Demonstrated to users independently
-->

### User Story 1 - Auto-launch bulk review (Priority: P1)

On launch, a prompt reviewer wants the latest unreviewed prompts to be queued and processed automatically so they immediately see improved prompts without manual triage.

**Why this priority**: Provides instant value every time the app opens and ensures recent history is kept fresh.

**Independent Test**: Launch the TUI with at least 5 unreviewed prompts newer than the rest; confirm an automatic batch of the newest items is queued and begins reviewing without user input.

**Acceptance Scenarios**:

1. **Given** at least one unreviewed prompt exists, **When** the TUI finishes indexing on launch, **Then** it queues the newest N (default 10) unreviewed prompts once per session.
2. **Given** queued prompts already have a completed review, **When** the auto batch runs, **Then** those prompts are skipped and not reprocessed.

---

### User Story 2 - See review progress overlay (Priority: P1)

As a reviewer, I want a floating window that summarizes auto-review progress so I can gauge time remaining without leaving the main layout.

**Why this priority**: Users need visibility into background processing; otherwise the automatic queue feels opaque.

**Independent Test**: Launch with an auto batch in progress; verify a floating pane shows total prompts, in-progress count, and completion/skip events until the batch ends.

**Acceptance Scenarios**:

1. **Given** an auto batch is running, **When** I view the overlay, **Then** it shows total prompts in the batch, completed count, failures, and elapsed time.
2. **Given** I dismiss the overlay, **When** the batch continues, **Then** reviews keep running and I can reopen the overlay from a status key.

---

### User Story 3 - Control the auto batch (Priority: P2)

As a reviewer, I want to pause or cancel the auto batch so that it doesn’t consume credits when I need to focus on something else.

**Why this priority**: Prevents waste and lets power users stay in control of provider usage.

**Independent Test**: Start the app with auto review enabled, cancel mid-way via the overlay, and confirm remaining prompts are not queued again during that session.

**Acceptance Scenarios**:

1. **Given** auto review is running, **When** I choose “Cancel auto review” in the overlay, **Then** all queued jobs are removed and the overlay indicates the batch was stopped by the user.
2. **Given** I cancelled the batch earlier this session, **When** I keep using the app, **Then** no further auto review batches run until I restart the app or manually trigger reviews.

### Edge Cases

- No unreviewed prompts exist on launch (should skip auto batch and show a brief notice).
- Network or provider CLI unavailable when the auto batch tries to start (should show overlay with failure summary and not retry until next launch).
- Multiple providers enabled with fewer than N unreviewed prompts each (should aggregate newest prompts across providers without duplicating sessions).
- App launched again quickly after a crash while the previous auto batch was running (must detect persisted state and avoid duplicate reviews).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: On every application launch, the system MUST automatically identify the newest unreviewed prompts (sorted by timestamp) across all enabled providers.
- **FR-002**: The system MUST queue only the top N most recent unreviewed prompts (default 10, configurable in settings) and must never queue prompts already reviewed with status `ReviewedOk`.
- **FR-003**: Auto review MUST run exactly once per session after indexing completes and MUST persist completion/skip metadata so the same batch is not repeated if the UI is reopened without new prompts.
- **FR-004**: While the auto batch runs, the system MUST render a floating overlay/pane summarizing total prompts in the batch, in-progress jobs, completed jobs, failures, and time elapsed.
- **FR-005**: Users MUST be able to dismiss or reopen the overlay without interrupting the batch, and MUST be able to cancel the entire auto batch from the overlay.
- **FR-006**: When cancelled, the system MUST stop enqueueing new prompts, mark remaining ones as “not processed,” and avoid re-queueing them automatically during the same session.
- **FR-007**: Auto review MUST gracefully handle provider failures: show error counts in the overlay, skip failed prompts from reprocessing immediately, and log failures in the existing worker status area.
- **FR-008**: Auto review MUST not block manual reviews—users can still trigger manual reviews, copy prompts, or navigate while the overlay displays progress.
- **FR-009**: The overlay MUST close automatically once all prompts in the batch reach a terminal state (completed, failed, or skipped) and show a summary toast.

### Key Entities *(include if feature involves data)*

- **AutoReviewBatch**: Represents the single startup batch (id, provider mix, prompt ids, start/end timestamps, user cancel flag).
- **ReviewProgressOverlay**: UI state capturing counts (queued, running, completed, failed, skipped), visibility, and user interaction flags (dismissed, cancelled).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With 10 prompts available, the auto batch must fully queue within 5 seconds of indexing completion and finish within 90 seconds on an M2 laptop (assuming providers respond normally).
- **SC-002**: In usability tests, 90% of users can describe the auto-review status within 5 seconds of launch using the overlay information.
- **SC-003**: In telemetry, auto batches must not reprocess prompts more than once per session (0 duplicate reviews recorded across 100 consecutive launches).
- **SC-004**: In dogfooding, at least 80% of launches with new prompts result in all fresh prompts being reviewed without manual intervention.

## Assumptions

- “Most recent prompts” refers to the newest unreviewed prompts across all enabled providers, capped at 10 by default; configuration options can adjust this later.
- The review worker is available and configured before auto review begins; failures are surfaced in the overlay if the worker is missing.
- Auto review is disabled while replaying historical sessions or when the user already has a manual batch running; these cases are out of scope for now.
- Configuration defaults live under `[auto_review]` in `~/.config/prompt-sage/config.toml`:
  - `enabled = true` (users can set to false to disable startup batches).
  - `batch_size = 10` (max number of prompts queued automatically).
  - `overlay = "auto"` (show overlay whenever a batch runs; values: `auto`, `always`, `never`).
- When the batch completes or is cancelled, `auto_review.last_run` metadata is persisted so the next launch can decide whether new prompts exist before queueing again.
