<!-- Thank you for contributing to arch-toolkit! 

**Important references:**
- [CONTRIBUTING.md](../CONTRIBUTING.md) — Full contribution guidelines and PR process
- [Development Wiki](https://github.com/Firstp1ck/arch-toolkit/wiki) — Development tools and documentation

Please ensure you've reviewed these before submitting your PR.
-->

## Summary
Briefly describe the problem and how your change solves it. (show as list if possible)

## Type of change
- [ ] feat (new feature)
- [ ] fix (bug fix)
- [ ] docs (documentation only)
- [ ] refactor (no functional change)
- [ ] perf (performance)
- [ ] test (add/update tests)
- [ ] chore (build/infra/CI)
- [ ] ui (visual/interaction changes)
- [ ] breaking change (incompatible behavior)

## Related issues
Closes #

## How to test
List exact steps and commands to verify the change. Include flags like `--dry-run` when appropriate.

```bash
# examples
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test -- --test-threads=1
cargo run --example with_caching
```

## Screenshots / recordings (if UI changes)
Include before/after images or a short GIF. Update files in `Images/` if relevant.

## Checklist

**Code Quality:**
- [ ] Code compiles locally (`cargo check`)
- [ ] `cargo fmt --all` ran without changes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean
- [ ] `cargo test -- --test-threads=1` passes
- [ ] Complexity checks pass for new code (`cargo test complexity -- --nocapture`)
- [ ] All new functions/methods have rustdoc comments (What, Inputs, Output, Details)
- [ ] No `unwrap()` or `expect()` in non-test code

**Testing:**
- [ ] Added or updated tests where it makes sense
- [ ] For bug fixes: created failing tests first, then fixed the issue
- [ ] Tests are meaningful and cover the functionality

**Documentation:**
- [ ] Updated README if API or behavior changed
- [ ] Updated rustdoc comments for public API changes
- [ ] Updated examples if usage patterns changed
- [ ] Added or updated integration tests for new features

**Compatibility:**
- [ ] No breaking changes to public API (or clearly documented if intentional)
- [ ] Backward compatible with existing code using the library
- [ ] Feature flags work correctly (optional features don't break default builds)

## Notes for reviewers
Call out tricky areas, assumptions, edge cases, or follow-ups.

## Breaking changes
Describe any breaking changes and migration steps (e.g., config key renames).

## Additional context
Logs, links to discussions, design notes, or prior art.


