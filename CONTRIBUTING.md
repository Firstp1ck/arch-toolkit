# Contributing to arch-toolkit

Thanks for your interest in contributing! arch-toolkit is a Rust library for Arch Linux package management, providing AUR operations, dependency resolution, and package index queries.

By participating, you agree to follow our [Code of Conduct](CODE_OF_CONDUCT.md).

For newcomers looking to contribute, we recommend starting with issues labeled "Good First Issue" in our [issue tracker](https://github.com/Firstp1ck/arch-toolkit/issues).

## Ways to contribute
- Bug reports and fixes
- Feature requests and implementations
- Documentation and examples
- Performance improvements
- Test coverage improvements

## Before you start
- **Target platform**: arch-toolkit is a Rust library that works on any platform where Rust runs. It provides functionality for Arch Linux package management operations.
- **Security**: If your report involves a security issue, use our [Security Policy](SECURITY.md).

## Development setup

### Prerequisites
1. Install Rust (stable):
   ```bash
   rustup default stable
   ```
2. Clone the repository:
   ```bash
   git clone https://github.com/Firstp1ck/arch-toolkit
   cd arch-toolkit
   ```

### Running tests
Tests must be run single-threaded to avoid race conditions:
```bash
cargo test -- --test-threads=1
# or
RUST_TEST_THREADS=1 cargo test
```

### Building the library
```bash
# Build the library
cargo build

# Build with all features
cargo build --all-features

# Build documentation
cargo doc --open
```

## Code quality requirements

### Pre-commit checklist
Before committing, ensure all of the following pass:

1. **Format code:**
   ```bash
   cargo fmt --all
   ```

2. **Lint with Clippy:**
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```
   
   The project uses strict Clippy settings configured in `Cargo.toml`:
   ```toml
   [lints.clippy]
   cognitive_complexity = "warn"
   pedantic = { level = "deny", priority = -1 }
   nursery = { level = "deny", priority = -1 }
   unwrap_used = "deny"
   ```
   
   Additional settings in `clippy.toml`:
   - `cognitive-complexity-threshold = 25`
   - `too-many-lines-threshold = 150`

3. **Check compilation:**
   ```bash
   cargo check
   ```

4. **Run tests:**
   ```bash
   cargo test -- --test-threads=1
   ```

5. **Check complexity (for new code):**
   ```bash
   # Run complexity tests to ensure new functions meet thresholds
   cargo test complexity -- --nocapture
   ```
   
   **Complexity thresholds:**
   - **Cyclomatic complexity**: Should be < 25 for new functions
   - **Data flow complexity**: Should be < 25 for new functions

### Code documentation requirements

**For all new code (functions, methods, structs, enums):**

1. **Rust documentation comments** are required:
   ```rust
   /// What: Brief description of what the function does.
   ///
   /// Inputs:
   /// - `param1`: Description of parameter 1
   /// - `param2`: Description of parameter 2
   ///
   /// Output:
   /// - Description of return value or side effects
   ///
   /// Details:
   /// - Additional context, edge cases, or important notes
   pub fn example_function(param1: Type1, param2: Type2) -> Result<Type3> {
       // implementation
   }
   ```

2. **Documentation should include:**
   - **What**: What the function/method does
   - **Inputs**: All parameters with descriptions
   - **Output**: Return value, side effects, or state changes
   - **Details**: Important implementation details, edge cases, or usage notes

3. **Include examples** in documentation when helpful:
   ```rust
   /// # Examples
   /// ```
   /// use arch_toolkit::aur;
   /// let client = reqwest::Client::new();
   /// let packages = aur::search(&client, "yay").await?;
   /// ```
   ```

### Testing requirements

**For bug fixes:**
1. **Create failing tests first** that reproduce the issue
2. Fix the bug
3. Verify tests pass
4. Add additional tests for edge cases if applicable

**For new features:**
1. Add unit tests for new functions/methods
2. Add integration tests for new workflows
3. Test error cases and edge conditions
4. Ensure tests are meaningful and cover the functionality

**Test guidelines:**
- Tests should be deterministic and not rely on external state
- Use mock HTTP responses (e.g., `wiremock`) for network-dependent tests
- Mark tests that require network access with `#[ignore]` and document why

## Commit and branch guidelines

### Branch naming
- `feat/<short-description>` — New features
- `fix/<short-description>` — Bug fixes
- `docs/<short-description>` — Documentation only
- `refactor/<short-description>` — Code refactoring
- `test/<short-description>` — Test additions/updates
- `chore/<short-description>` — Build/infrastructure changes

### Commit messages
Prefer [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>: <short summary>

<optional longer description>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring (no functional change)
- `perf`: Performance improvements
- `test`: Test additions or updates
- `chore`: Build/infrastructure changes
- `breaking change`: Incompatible behavior changes

**Examples:**
```
feat: add AUR comments scraping functionality

- Implemented HTML parsing for AUR package comments
- Added date parsing and timezone conversion
- Included pinned comment detection
```

```
fix: resolve rate limiting backoff reset issue

Fixes issue where rate limiting backoff was not reset after
successful requests, causing unnecessary delays.
```

**Guidelines:**
- Keep commits focused and reasonably small
- Add rationale in the body if the change is non-obvious
- Reference issue numbers if applicable: `Closes #123` or `Fixes #456`

## Pull Request process

### Before opening a PR

1. **Ensure all quality checks pass:**
   - [ ] `cargo fmt --all` (no changes needed)
   - [ ] `cargo clippy --all-targets --all-features -- -D warnings` (clean)
   - [ ] `cargo check` (compiles successfully)
   - [ ] `cargo test -- --test-threads=1` (all tests pass)
   - [ ] Complexity checks pass for new code

2. **Code requirements:**
   - [ ] All new functions/methods have rustdoc comments (What, Inputs, Output, Details)
   - [ ] Code follows project conventions (see below)
   - [ ] No `unwrap()` or `expect()` in non-test code (use proper error handling)
   - [ ] Complexity thresholds met (cyclomatic < 25, data flow < 25)

3. **Testing:**
   - [ ] Added/updated tests where it makes sense
   - [ ] For bug fixes: created failing tests first, then fixed the issue
   - [ ] Tests are meaningful and cover the functionality

4. **Documentation:**
   - [ ] Updated README if API or behavior changed
   - [ ] Updated rustdoc comments for public API changes
   - [ ] Added examples to documentation where helpful

5. **Compatibility:**
   - [ ] No breaking changes (or clearly documented if intentional)
   - [ ] Backward compatibility maintained when possible

### PR description template

Use the following structure for your PR description:

```markdown
## Summary
Brief description of what this PR does.

**Bug Fixes:**
1. Description of bug fix 1
2. Description of bug fix 2

**New Features:**
1. Description of new feature 1
2. Description of new feature 2

## Type of change
- [ ] feat (new feature)
- [ ] fix (bug fix)
- [ ] docs (documentation only)
- [ ] refactor (no functional change)
- [ ] perf (performance)
- [ ] test (add/update tests)
- [ ] chore (build/infra/CI)
- [ ] breaking change (incompatible behavior)

## Related issues
Closes #123

## How to test
Step-by-step testing instructions or code examples.

## Checklist
- [ ] Code compiles locally
- [ ] `cargo fmt --all` ran without changes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean
- [ ] `cargo test -- --test-threads=1` passes
- [ ] Added or updated tests where it makes sense
- [ ] Updated docs if API or behavior changed
- [ ] No breaking changes (or clearly documented if intentional)

## Notes for reviewers
Any additional context, implementation details, or decisions that reviewers should know.

## Breaking changes
None (or description of breaking changes if applicable)
```

## Project conventions

### Code style
- **Language**: Rust (edition 2024)
- **Naming**: Clear, descriptive names. Favor clarity over brevity.
- **Error handling**: Use `Result` types. Avoid `unwrap()`/`expect()` in non-test code.
- **Early returns**: Prefer early returns over deep nesting.
- **Logging**: Use `tracing` for diagnostics. Avoid noisy logs at info level.

### API design
- **Feature flags**: New functionality should be gated behind feature flags when appropriate
- **Error types**: Use the unified `ArchToolkitError` type for all errors
- **Async-first**: Prefer async APIs for I/O operations
- **Documentation**: All public APIs must have comprehensive rustdoc comments with examples

### Documentation updates
When updating documentation:

1. **README.md**: Keep high-level, provide usage examples
2. **rustdoc**: Ensure all public items have complete documentation
3. **Examples**: Add examples to `examples/` directory for common use cases

## Filing issues

### Bug reports
Include the following information:
- arch-toolkit version (e.g., `0.1.0` or commit hash)
- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce (code example preferred)
- Expected vs. actual behavior
- Error messages or logs (run with `RUST_LOG=arch_toolkit=debug`)

**Example:**
```markdown
**Version**: 0.1.0
**Rust Version**: rustc 1.75.0
**OS**: Arch Linux

**Steps to reproduce:**
```rust
use arch_toolkit::aur;
use reqwest::Client;

let client = Client::new();
let result = aur::search(&client, "yay").await;
```

**Expected**: Returns `Ok(Vec<AurPackage>)` with search results
**Actual**: Returns `Err(ArchToolkitError::Network(...))`

**Logs:**
[Include relevant log output with RUST_LOG=arch_toolkit=debug]
```

### Feature requests
- Describe the problem being solved
- Describe the desired API/behavior
- Include code examples of how you'd like to use the feature
- Consider edge cases and backward compatibility

Open issues in the [issue tracker](https://github.com/Firstp1ck/arch-toolkit/issues).

## Publishing to crates.io

When publishing new versions:
- Ensure all tests pass
- Update `CHANGELOG.md` with release notes
- Update version in `Cargo.toml`
- Run `cargo publish --dry-run` to verify
- Tag the release in git
- Publish with `cargo publish`

## Code of Conduct and Security
- **Code of Conduct**: See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). For conduct issues, contact firstpick1992@proton.me.
- **Security Policy**: See [SECURITY.md](SECURITY.md) for how to report vulnerabilities.

## Getting help
- Check the [README.md](README.md) for documentation
- Review existing issues and PRs
- Ask questions in [Discussions](https://github.com/Firstp1ck/arch-toolkit/discussions)

Thank you for helping improve arch-toolkit!
