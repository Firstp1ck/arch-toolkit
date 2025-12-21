---
name: Bug Report
about: Report a bug or unexpected behavior in arch-toolkit
title: '[BUG] '
labels: 'bug'
assignees: ''
---

## Bug Description
A clear and concise description of what the bug is.

## To Reproduce
Steps to reproduce the behavior:
1. 
2. 
3. 
4. 

## Expected Behavior
A clear and concise description of what you expected to happen.

## Actual Behavior
What actually happened instead.

## Environment Information

**arch-toolkit Version:**
- Version: `0.1.0` (or commit hash if building from source)
- Installation method: [ ] Cargo/crates.io [ ] Built from source
- Features enabled: [ ] `aur` [ ] `cache-disk`

**System Information:**
- Distribution: [e.g., Arch Linux, EndeavourOS, Manjaro]
- Rust version: [e.g., `1.75.0`]
- Cargo version: [e.g., `1.75.0`]

## Logs

**Important**: Please run with debug logging enabled and include relevant log output.

```bash
# Run with debug logging
RUST_LOG=arch_toolkit=debug cargo run --example with_caching

# Or in your code
RUST_LOG=arch_toolkit=debug your_program
```

**Relevant log output:**
```
[Paste relevant log output here, especially errors and warnings]
```

## Screenshots
If applicable, add screenshots to help explain the problem. Screenshots are especially helpful for:
- UI rendering issues (misaligned text, broken layout, visual glitches)
- Error messages or dialogs
- Unexpected visual behavior

You can drag and drop images directly into this issue.

## Additional Context
- Does this happen consistently or intermittently?
- When did you first notice this issue?
- Did this work in a previous version? If so, which version?
- Any workarounds you've found?
- Related issues or discussions (if any)

## Checklist
- [ ] I have included all required environment information
- [ ] I have provided logs from running with `RUST_LOG=arch_toolkit=debug`
- [ ] I have checked the [Documentation](https://docs.rs/arch-toolkit)
- [ ] I have searched existing issues to ensure this hasn't been reported before
