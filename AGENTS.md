# Repository Guidelines

This repository is intended for building and iterating on prompt- and agent-related tooling. Use this guide to keep structure, style, and workflows consistent as the project grows.

## Project Structure & Module Organization

- Place all application and library code in `src/` (e.g., `src/core`, `src/cli`, `src/agents`).
- Keep tests in a parallel tree under `tests/` mirroring the `src/` layout.
- Store example configs, prompts, or datasets in `examples/` or `assets/` as appropriate.
- Prefer small, focused modules over large files; group by domain (e.g., `agents/`, `parsing/`, `storage/`).

## Build, Test, and Development Commands

- `npm install` or `pnpm install` – install dependencies.
- `npm run dev` – run the main development entry point (CLI or app) with live reload when configured.
- `npm test` – run the automated test suite.
- `npm run lint` / `npm run format` – run linters and formatters before pushing changes.

## Coding Style & Naming Conventions

- Use TypeScript where possible; default to 2‑space indentation and single quotes.
- Name files using `kebab-case` for scripts (`prompt-runner.ts`) and `PascalCase` for React components if added.
- Export primary modules as named exports; avoid default exports for shared utilities.
- Run the project formatter (`npm run format`) before committing to keep diffs clean.

## Testing Guidelines

- Prefer fast, deterministic unit tests colocated under `tests/` using the same relative path as `src/`.
- Name test files as `<name>.test.ts` (e.g., `src/core/engine.ts` → `tests/core/engine.test.ts`).
- Ensure new features include tests; bug fixes should add a regression test where practical.

## Commit & Pull Request Guidelines

- Write clear, descriptive commit messages in the form `area: short summary` (e.g., `core: improve agent selection`).
- Keep pull requests focused and small; describe the problem, the approach, and any alternatives considered.
- Link related issues or tasks, and include screenshots or CLI output when UX or behavior changes.
- Ensure lint, tests, and formatting pass locally before opening or updating a PR.

## Agent-Specific Instructions

- When adding agent behavior or prompts, keep them modular (one concern per file) and document assumptions at the top of the file.
- Prefer configuration-driven behavior (JSON/YAML/TS config) over hard‑coded values to simplify experimentation.
