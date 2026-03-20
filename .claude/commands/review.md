---
model: opus
---

Review all uncommitted changes (staged + unstaged) in this project and check them against the criteria below. Report issues found, or confirm the changes look good.

## Steps

1. Run `git diff` (unstaged) and `git diff --cached` (staged) to collect all changes.
2. If there are no changes, say so and stop.
3. Read any modified files in full where needed for context.
4. Evaluate the changes against every check below.
5. Output a short verdict: either "Looks good" with a one-line summary, or a numbered list of issues.

## Checks

### Rust conventions
- Uses `Result<T, anyhow::Error>` for QBE compilation errors; `QbeError::new(msg, span)` for user-facing errors with location, `QbeError::no_span(msg)` for internal errors.
- Parser errors use `ParseError` with `Span`; parse methods return `Result<_, ParseError>`.
- `ResType` enum (`Number`, `Bool`, `Void`) is used for compile-time type tracking — no ad-hoc strings.
- QBE IR labels use `@label_N` naming; function params use `%p_<name>`.
- New public items follow the patterns already in the codebase (naming, module structure, visibility).

### Correctness
- No `unwrap()` or `expect()` on user-controlled data — propagate errors instead.
- No panics in parser or compiler paths.
- Type-checking logic is consistent with existing `ResType` handling.
- If new AST variants are added, they are handled in the parser, compiler, **and** tests.

### Tests
- New functionality has corresponding tests.
- Existing tests are not removed or weakened without justification.
- Tests actually assert meaningful behavior, not just "doesn't crash".

### Code quality
- No dead code, unused imports, or leftover debug prints.
- No TODO/FIXME without explanation.
- Changes are minimal — no unrelated refactors, formatting churn, or scope creep.
- Follows the existing coding style (indentation, naming, module layout).

### QBE IR quality (qbe.rs changes)
If the diff touches `qbe.rs`, read the full file and also inspect the QBE IR output for any changed/added functionality. Check for:
- **Redundant temporaries**: a value is stored into `%tN` only to be immediately used once — emit it inline instead.
- **Unnecessary variable slots**: `alloc`/`store`/`load` sequences where a temporary (`%tN`) would suffice. Stack slots should only be used for mutable variables that are reassigned.
- **Dead stores**: a value is written to a slot or temporary but never read.
- **Redundant copies**: `%tA =l copy %tB` where `%tB` could be used directly.
- **Extra loads**: the same slot is loaded multiple times with no intervening store — cache the result in a temporary.
- **Suboptimal control flow**: empty blocks, unconditional jumps to the immediately next block, or branches that could be simplified.
- **Type-width mismatches**: using `w` (32-bit) ops on values that should be `l` (64-bit) or `d` (double), or vice versa.

For each issue found, show the specific IR pattern and suggest the tighter version.

### Build & CI
- `cargo build` and `cargo test` should still pass with the changes (flag if you suspect they won't, but do not run them unless asked).
- No new dependencies added without clear justification.
