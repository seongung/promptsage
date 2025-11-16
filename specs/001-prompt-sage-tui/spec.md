# Feature Specification: Prompt Sage TUI – Prompt History Review

**Feature Branch**: `001-prompt-sage-tui`  
**Created**: 2025-11-15  
**Status**: Draft  
**Input**: User description: "Define the product “Prompt Sage TUI” as a user-facing terminal application that: - indexes prompt history from Claude Code (~/.claude/projects/**.jsonl) and Codex (~/.codex/sessions/**.jsonl), - groups by project and date, shows original prompts, and on demand generates an improved prompt + explanation, - uses Claude (`claude -p --output-format json`) or Codex (`codex exec --json`) with a “Prompt Review Coach” instruction, - enforces read-only/safe modes, - supports copy/export of improvements, - and fits in a single-screen Ratatui layout. List target users, primary jobs, OOS items, and measurable acceptance criteria (e.g., parse ≥95% of logs in a test corpus; review turnaround <3s median on M2)."

## Target Users & Primary Jobs

### Target Users

- Developers who use Claude Code in their editor and want to refine prompts based on past sessions.
- Developers who use OpenAI Codex or similar coding assistants and have JSONL session logs stored on disk.
- Tech leads or prompt engineers who want to audit and improve their own or their team's AI-assisted coding prompts.

### Primary Jobs To Be Done

- Quickly browse historical prompts grouped by project and date to find examples worth improving.
- Request an improved version of a selected prompt plus a short explanation of how and why it was improved.
- Copy or export improved prompts so they can be reused in editors, documentation, or prompt libraries.
- Review prompt history in a safe, read-only way that never executes code or modifies project files.

## Clarifications

### Session 2025-11-15

- Q: What content is sent to the AI for review? → A: Only the selected prompt text (no surrounding transcript, no file paths or project identifiers).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Review a prompt for a project (Priority: P1)

As a developer using Claude Code or Codex, I want to select a project and date, browse prompts from that session, and request an improved version plus a short explanation, so I can reuse a better prompt in my editor.

**Why this priority**: This is the core value of Prompt Sage TUI and delivers a complete, independently useful workflow: understand, improve, and reuse a single prompt.

**Independent Test**: Start with existing JSONL logs for one project and provider; verify that a user can navigate to a prompt, request a review, see the improved prompt and explanation, and copy the improved prompt without any file modifications or code execution.

**Acceptance Scenarios**:

1. **Given** valid prompt logs exist for a project and provider, **When** the user launches the TUI, selects the provider, project, and date, **Then** a list of prompts for that date is shown in a single-screen terminal layout.
2. **Given** a prompt is selected in the list, **When** the user triggers "Review prompt", **Then** the system shows the original prompt, an improved prompt, and a short explanation side by side without executing any project code or changing any files.

---

### User Story 2 - Scan history for improvement opportunities (Priority: P2)

As a developer or prompt engineer, I want to filter and scan prompts across dates and sessions for a project so that I can quickly identify prompts that are confusing, repetitive, or poorly performing and send them for review.

**Why this priority**: This enables users to move beyond one-off reviews and systematically improve prompt quality over time, increasing value for active users.

**Independent Test**: With multiple days of logs for a project, verify that the user can filter by provider and date range, scroll through prompts, and mark or select prompts for review without depending on any export or bulk operations.

**Acceptance Scenarios**:

1. **Given** multiple dates of logs exist for a project, **When** the user filters by provider and date range, **Then** only prompts from matching sessions are listed.
2. **Given** a list of prompts is visible, **When** the user moves the selection through prompts using the keyboard, **Then** the detail pane updates to show the full original prompt and any existing review results without leaving the single-screen layout.

---

### User Story 3 - Export improved prompts for reuse (Priority: P3)

As a developer or tech lead, I want to export improved prompts and explanations so I can share them with my team or reuse them in other tools without re-running the review.

**Why this priority**: Sharing and reuse extends the value of prompt reviews beyond the individual user and reduces repeated work.

**Independent Test**: Starting from a project with existing improved prompts, verify that a user can select one or more prompts and export the improved versions plus explanations to a text-based format, without needing to run new reviews or modify existing logs.

**Acceptance Scenarios**:

1. **Given** improved prompts exist for one or more prompts, **When** the user selects an export option, **Then** a text-based artifact is produced containing the improved prompt and explanation, suitable for use in documentation or other tools.
2. **Given** an improved prompt is visible, **When** the user chooses to copy it, **Then** the improved prompt text is made available for pasting into another application without any changes to project source files.

---

### Edge Cases

- No logs found for a provider or project; the TUI shows a clear "no history found" state instead of failing.
- Log files contain malformed or partially written JSONL lines; invalid lines are skipped with a visible warning while valid prompts remain usable.
- Only one provider's logs are present (Claude Code only or Codex only); the TUI still loads and operates for the available provider.
- The underlying review backends (Claude/Codex) are unavailable or slow; the user sees clear error or timeout messages and the TUI remains responsive.
- Very large log sets (many projects or long histories) are present; indexing and navigation remain responsive enough that the TUI is usable.

## Requirements *(mandatory)*

### UI Layout Reference

The core Prompt Sage TUI screen MUST present a single-screen, multi-pane layout conceptually similar to
the reference below (adapted as needed for terminal size), with:

- A header showing the application name and active provider.
- A projects area.
- A prompts list with date, session id, snippet, length, and review status.
- Side-by-side panes for the original prompt and the improved "Sage" prompt plus explanation.
- A footer showing available keybindings for navigation and review actions.

```text
+----------------------------------------------------------------------------------+
| PromptSage  [Claude]*   Projects: my-service | lib-auth | misc                   |
+----------------------------------------------------------------------------------+
| Prompts  (↑/↓ select · Enter preview · r Sage review · / search · f filter)      |
| Date        Session   Snippet                              Len   Sage             |
| 2025-11-15  #a9c1..   build script cross-platform...       145   ✓                |
| 2025-11-14  #e21b..   how to package binary...              88   R                |
| 2025-11-12  #9fa2..   add ci cache steps...                  73   ×                |
| ...                                                                              |
+----------------------------------------------+-----------------------------------+
| [Original] (scrollable)                      | [Sage]  Score 84/100              |
| Generate a build script that...              | Create a cross-platform build...   |
| ...                                          | - Breaks into steps               |
|                                              | - OS/arch explicit                |
|                                              | - Output contract                 |
+----------------------------------------------+-----------------------------------+
| [Tab] switch pane  [r] review(Sage)  [c] copy  [s] save  [d] DIFF  [q] quit      |
+----------------------------------------------------------------------------------+
```

### Functional Requirements

- **FR-001**: System MUST index prompt history from standard Claude Code and Codex JSONL log locations (for example, `~/.claude/projects/**.jsonl` and `~/.codex/sessions/**.jsonl`) and organize prompts by provider, project, and date.
- **FR-002**: System MUST present a single-screen terminal interface that simultaneously exposes: a header with application name and active provider, a project/session list, a prompt list (including key metadata such as date, session id, snippet, and review status), the original prompt, and any improved prompt plus explanation, without requiring multi-screen navigation, conceptually aligned with the UI layout reference.
- **FR-003**: Users MUST be able to select provider, project, and date (or date range) and navigate through prompts using keyboard-only controls.
- **FR-004**: On explicit user request, System MUST send only the selected prompt text (no surrounding transcript or file paths or project identifiers) to the underlying AI assistant and generate an improved version of that prompt plus a short explanation using a "Prompt Review Coach" instruction, focusing on clearer, more specific prompts with explicit output contracts.
- **FR-005**: System MUST operate in a read-only mode by default: it MUST NOT execute project code, run arbitrary shell commands, or modify existing project files or log files.
- **FR-006**: Users MUST be able to copy an improved prompt (with or without its explanation) for reuse and export one or more improved prompts plus explanations to a text-based artifact (for example, a file or stdout) without altering any existing logs or source code.
- **FR-007**: System MUST handle missing or malformed log entries gracefully by skipping invalid records, surfacing clear warnings, and continuing to index and display valid prompts.
- **FR-008**: System MUST support operating when only one provider's logs are present (Claude Code only or Codex only) without requiring configuration changes.
- **FR-009**: System MUST provide visible status feedback during prompt review requests (for example, "reviewing", "completed", "failed") and surface errors or timeouts without freezing the TUI.
- **FR-010**: System MUST keep all analysis and review operations read-only and MUST provide configuration options to disable or restrict any export behavior that users consider sensitive.
- **FR-011**: System MUST ignore AI-generated or internal agent prompts (such as hidden system messages, tool calls, or scaffolding prompts not written directly by users) when listing, reviewing, or exporting prompts, and MUST only treat user-authored prompts from the logs as candidates for review and export.

### Key Entities *(include if feature involves data)*

- **Prompt Source**: Represents a physical log source (for example, Claude Code or Codex JSONL files) and its configuration (paths, provider name, enabled/disabled).
- **Project Session**: Represents a grouping of prompts by provider, project identifier, and date or time window; used for navigation and filtering.
- **Prompt Record**: Represents a single prompt extracted from history, including metadata (provider, project, timestamp, source file reference if available) and the original prompt text.
- **Prompt Review**: Represents the outcome of a review for a given prompt, including the improved prompt text, an explanation of changes, timestamps, and status (pending, completed, failed).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a representative test corpus of Claude Code and Codex JSONL logs, at least 95% of log files and prompt entries are successfully parsed into prompt records without errors that block usage; any unparsed entries are logged and do not crash the application.
- **SC-002**: For typical prompts (for example, up to a few paragraphs of text), the median time from triggering "Review prompt" to seeing the improved prompt and explanation is less than 3 seconds on a modern laptop comparable to an Apple M2 with stable connectivity, with at least 90% of reviews completing within 5 seconds.
- **SC-003**: In usability tests, at least 80% of target users can complete User Story 1 (review and copy an improved prompt for a known project) within 5 minutes without consulting external documentation.
- **SC-004**: In guided safety tests, 100% of review flows complete without executing any project code or modifying project files or existing log files.
- **SC-005**: In qualitative feedback from early adopters, at least 80% of respondents rate the improved prompts as "clearly better" or "much better" than the originals for their primary coding workflows.
