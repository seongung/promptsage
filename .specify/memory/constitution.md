<!--
Sync Impact Report
Version change: N/A → 1.0.0
Modified principles:
- [PRINCIPLE_1_NAME] → Vision
- [PRINCIPLE_2_NAME] → Non-negotiables
- [PRINCIPLE_3_NAME] → Engineering Guardrails
- [PRINCIPLE_4_NAME] → UX Guardrails
- [PRINCIPLE_5_NAME] → Operational Policies
Added sections:
- Vision (under Core Principles)
- Non-negotiables
- Engineering Guardrails
- UX Guardrails
- Operational Policies
Removed sections:
- None (template placeholders replaced)
Templates requiring updates:
- ✅ .specify/templates/plan-template.md (Constitution Check gates aligned)
- ⚠ .specify/templates/spec-template.md (generic; no direct constitution gates to sync)
- ⚠ .specify/templates/tasks-template.md (generic; no direct constitution gates to sync)
- ⚠ .specify/templates/commands/*.md (directory not present; no checks performed)
Follow-up TODOs:
- TODO(COMMAND_TEMPLATES_DIR): When command templates are added, ensure they reference the current constitution principles.
-->

# Prompt Sage TUI Constitution

## Core Principles

### Vision

Prompt Sage TUI exists to provide a reliable, safe, and portable terminal interface for working with AI coding agents (including Claude Code and OpenAI Codex) by turning raw transcripts into auditable prompts, plans, and commands without surprising side effects.

All feature and design decisions MUST prioritize, in order: (1) correctness of transcript parsing, (2) safety and controlled execution, (3) high-quality prompt instruction, (4) portability, (5) performance, (6) privacy, and (7) observability.

### Non-negotiables

1. Transcript parsing correctness:
   - Parsers for Claude Code and OpenAI Codex transcripts MUST preserve message order, speaker roles, and code block boundaries exactly.
   - Any change to transcript parsing MUST include regression tests using real-world example transcripts for both agents.
   - Parsed representations MUST be deterministic given the same input transcript.

2. Safety and execution control:
   - The TUI MUST NOT execute code or shell commands automatically.
   - All generated commands and code transformations MUST be read-only by default (for example, using dry-run flags or diff views where applicable).
   - Any feature that can modify files, run commands, or invoke tools with side effects MUST require explicit, reversible user consent and display a clear summary of the impact before confirmation.

3. Privacy baseline:
   - The system MUST avoid sending secrets or sensitive paths inadvertently in model calls; redaction MUST be offered before any outbound call that includes user data.
   - Logs and telemetry MUST NOT contain secrets or raw user content unless the user explicitly opts in.

### Engineering Guardrails

1. Transcript and prompt handling:
   - All parsers and prompt builders MUST be implemented as pure, side-effect-free functions given their inputs.
   - Cross-agent behavior (Claude Code versus OpenAI Codex) MUST be encapsulated behind well-defined interfaces so that agent-specific quirks do not leak into the rest of the codebase.

2. Portability:
   - The supported environments are macOS and Linux as first-class; Windows usage MUST be supported via WSL for Codex workflows.
   - Platform-specific code MUST be isolated behind small adapters with tests that can run on at least one continuous-integration-supported POSIX environment.
   - No feature MAY depend on a graphical user interface or non-portable shell features without a documented fallback for supported platforms.

3. Performance and responsiveness:
   - Long-running operations (network calls, large file I/O, heavy parsing, or model invocations) MUST run in background workers or asynchronous tasks, not on the main TUI input loop.
   - The TUI event loop MUST remain responsive; user input MUST NOT be blocked by background work beyond a brief debounce window.
   - Operations MUST be designed to stream partial results where feasible instead of blocking until full completion.

4. Observability and feature flags:
   - New cross-cutting features (parsers, tool runners, model backends) MUST be guarded by feature flags with safe defaults.
   - Logs MUST be structured (at least level, timestamp, component, and correlation id when applicable) to support debugging without reproducing user data.

### UX Guardrails

1. Prompt instruction quality:
   - Any prompt rewrite produced by the TUI MUST make the user's intent clearer and more specific than the original input.
   - Prompts sent to models MUST include an explicit "Output Contract" section stating required format, constraints, and non-goals when applicable.
   - The TUI MUST expose both the original user input and the final prompt text so that users can audit changes before sending.

2. Safety signaling:
   - Actions that can cause side effects (file writes, command execution, tool calls) MUST be visually and textually distinguished from read-only actions.
   - Confirmation dialogs MUST summarize what will change (paths, commands, or resources) and provide a simple way to cancel.

3. Privacy and redaction UX:
   - Before sending any transcript or prompt that includes user data to a model, the TUI MUST offer an opportunity to review and redact sensitive content.
   - Redaction tooling MUST clearly show what content will leave the local environment and what will remain local.

### Operational Policies

1. Configuration and defaults:
   - Default configuration MUST enforce read-only behavior and conservative logging; users MAY opt into more powerful or verbose modes explicitly.
   - Feature flags controlling safety-sensitive behavior MUST default to the safest option and MUST be able to be toggled without code changes (for example, via configuration files or environment variables).

2. Logging and retention:
   - Logs MUST capture enough structured context to debug (component, operation, outcome, correlation id) but MUST exclude secrets and high-sensitivity payloads by default.
   - Any optional logging of prompts, transcripts, or model responses MUST be off by default and clearly marked as potentially sensitive.

3. Rollouts and regressions:
   - Changes to transcript parsing, execution flows, or privacy behavior MUST be rolled out behind flags or configuration switches that allow rapid rollback.
   - Each release MUST document any change that affects safety, privacy, or portability, including mitigation or rollback instructions.

## Additional Constraints

The implementation and evolution of Prompt Sage TUI MUST remain consistent with this constitution. Where conflicts arise between convenience and the non-negotiable principles above, correctness, safety, and privacy MUST win.

This constitution supersedes ad hoc conventions or shortcuts taken in individual features. Exceptions MUST be explicit, temporary, and documented with a clear plan to return to compliance.

## Development Workflow

Feature work, refactors, and operational changes MUST include an explicit Constitution Check step in their design or implementation plan. Reviews MUST verify that changes respect:

- Transcript parsing correctness for Claude Code and OpenAI Codex.
- Safety and read-only-by-default execution.
- Prompt instruction quality with explicit output contracts.
- Portability across macOS, Linux, and WSL-based Codex workflows.
- Non-blocking UI behavior with background workers for long-running tasks.
- Privacy expectations, including redaction and safe logging.
- Observability via structured logs and feature flags.

Deviations from these guardrails MUST be documented, justified, time-bounded, and tracked as technical debt.

## Governance

This constitution governs the design, implementation, and operation of Prompt Sage TUI. It applies to all contributors and automated tools acting on this project.

Amendments to this constitution MUST:

- Be proposed in writing, including the rationale and expected impact on correctness, safety, prompt quality, portability, performance, privacy, and observability.
- Specify the intended semantic version bump (MAJOR, MINOR, or PATCH) and justify it.
- Be reviewed and approved through the project's standard review process before adoption.

Versioning policy:

- MAJOR version increases when principles or guardrails are removed, redefined, or relaxed in a way that is not backward compatible with existing governance.
- MINOR version increases when new principles, sections, or materially expanded guidance are added while preserving existing guarantees.
- PATCH version increases when clarifications, wording changes, or non-semantic refinements are made.

Compliance review:

- Each significant change (feature, refactor, or operational change) MUST include an explicit Constitution Check, referencing how the change satisfies or impacts the principles in this document.
- Periodic reviews (at least quarterly while the project is active) SHOULD assess whether the implementation, documentation, and operational practices still comply with this constitution; if such reviews are skipped, maintainers MUST explain why and how compliance risk is mitigated.

**Version**: 1.0.0 | **Ratified**: 2025-11-15 | **Last Amended**: 2025-11-15
