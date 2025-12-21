# Security Policy

## Supported Versions

arch-toolkit provides security updates for the current minor release line.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0   | :x:                |

## Reporting a Vulnerability

If you believe you’ve found a security issue in arch-toolkit, please report it responsibly.

- Preferred: Email firstpick1992@proton.me with the subject "[arch-toolkit Security]".
- Alternative: If email isn’t possible, open a GitHub issue with minimal details and include the word "Security" in the title. We’ll triage and, if appropriate, coordinate privately.

Please include, when possible:
- arch-toolkit version (e.g., 0.1.x) and install method (crates.io, git, source)
- Rust version and environment details (OS, architecture, tokio version if applicable)
- Reproduction steps and expected vs. actual behavior
- Impact assessment and a proof-of-concept if available
- Any relevant logs or error messages

What to expect:
- Acknowledgement within 3 business days
- Status updates at least weekly until resolution
- Coordinated disclosure: we’ll work with you on timing and credit (or anonymity if you prefer)

Out of scope:
- Issues in third-party AUR helpers (e.g., paru, yay) or Arch mirrors
- Issues in archlinux.org infrastructure or AUR RPC endpoints
- Non-security bugs (please use regular GitHub issues)

Thank you for helping keep arch-toolkit and its users safe.
