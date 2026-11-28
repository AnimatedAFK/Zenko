# Rules for the Rust rewrite of Zenko

These are the user's standing rules for every session working in RUST_REWRITE/.

## The project
- The original Python project (zenko.py and its assets) is the source of truth. Keep its look, animations, features and behaviour exactly.
- Never change or overwrite the original Python project. Never replace its assets with made-up approximations.
- The finished app is written in Rust. All Rust work stays inside RUST_REWRITE/.

## Saving tokens
1. Look at the project structure before changing anything.
2. Read only the files the current task needs.
3. Don't re-read files that haven't changed.
4. Reuse the docs and notes that already exist (PROGRESS.md, docs/MIGRATION_PLAN.md, docs/AUDIT_INVENTORY.md).
5. Don't paste large blocks of code into replies.
6. Don't re-explain things that are already settled.
7. Don't work out architecture decisions again (see PROGRESS.md).
8. When more files are needed, read only the parts that matter.

## Quality (saving tokens never wins over this)
1. Never trade correctness for tokens.
2. Keep everything that already works.
3. Copy the original rendering logic. Don't approximate it.
4. Keep modules maintainable, and make each one testable on its own.
5. Check dependencies and compatibility before adding new ones.
6. Run the relevant tests after every change.
7. Fix real errors. Don't hide them.
8. Don't call a feature complete until it has been tested.

## How to do each task
1. Name the exact feature and what it depends on.
2. Read the source files it touches.
3. Explain the approach in a few words.
4. Build the feature completely.
5. Run the relevant tests.
6. Update the docs and the migration checklist (docs/MIGRATION_PLAN.md).
7. Finish with a short report.

## Keeping track between sessions
At the end of each task, update PROGRESS.md with:
- what was finished
- which files changed
- important architecture decisions
- the tests that were run and their results
- problems still open
- the next recommended task

Don't repeat the whole project history every session.

Saving tokens must never mean less implementation detail, an unfinished migration, a less faithful look, or skipped tests.
