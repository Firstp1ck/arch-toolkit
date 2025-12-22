//! Comprehensive PKGBUILD parsing example for arch-toolkit.
//!
//! This example demonstrates:
//! - Parsing PKGBUILD dependency arrays (depends, makedepends, checkdepends, optdepends)
//! - Parsing conflicts arrays
//! - Handling single-line and multi-line bash array syntax
//! - Handling append syntax (depends+=)
//! - Filtering virtual packages (.so files)
//! - Automatic deduplication
//!
//! Run with:
//!   `cargo run --example pkgbuild_example --features deps`

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example pkgbuild_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)] // Example file - comprehensive demonstration
fn main() {
    use arch_toolkit::deps::{parse_pkgbuild_conflicts, parse_pkgbuild_deps};

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: PKGBUILD Parsing Example                    ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Basic PKGBUILD Parsing
    // ========================================================================
    println!("┌─ Example 1: Basic PKGBUILD Parsing ───────────────────────────┐");
    println!("│ Parse a simple PKGBUILD with dependency arrays                 │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let pkgbuild_content = r"
pkgname=example-package
pkgver=1.2.3
pkgrel=1
depends=('glibc' 'python>=3.10')
makedepends=('make' 'gcc')
checkdepends=('check')
optdepends=('optional: optional-feature')
";

    let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild_content);

    println!("Runtime Dependencies:");
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nMake Dependencies:");
    for dep in &makedepends {
        println!("  - {dep}");
    }
    println!("\nCheck Dependencies:");
    for dep in &checkdepends {
        println!("  - {dep}");
    }
    println!("\nOptional Dependencies:");
    for dep in &optdepends {
        println!("  - {dep}");
    }

    // ========================================================================
    // Example 2: Single-Line Arrays
    // ========================================================================
    println!("\n┌─ Example 2: Single-Line Arrays ────────────────────────────────┐");
    println!("│ Parse single-line bash array syntax                            │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let single_line = r"
depends=('foo' 'bar>=1.2' 'baz')
makedepends=(cmake ninja)
";

    let (depends, makedepends, _, _) = parse_pkgbuild_deps(single_line);
    println!("Dependencies from single-line array:");
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nMake Dependencies from single-line array:");
    for dep in &makedepends {
        println!("  - {dep}");
    }

    // ========================================================================
    // Example 3: Multi-Line Arrays
    // ========================================================================
    println!("\n┌─ Example 3: Multi-Line Arrays ─────────────────────────────────┐");
    println!("│ Parse multi-line bash array syntax                             │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let multiline = r"
depends=(
    'glibc'
    'gtk3>=3.24'
    'python>=3.10'
    'nss'
)
makedepends=(
    'make'
    'gcc'
    'pkg-config'
)
";

    let (depends, makedepends, _, _) = parse_pkgbuild_deps(multiline);
    println!("Dependencies from multi-line array:");
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nMake Dependencies from multi-line array:");
    for dep in &makedepends {
        println!("  - {dep}");
    }

    // ========================================================================
    // Example 4: Append Syntax (depends+=)
    // ========================================================================
    println!("\n┌─ Example 4: Append Syntax (depends+=) ────────────────────────┐");
    println!("│ Parse append syntax used in package() functions               │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let append_syntax = r"
pkgname=test-package
pkgver=1.0.0
depends=('base-dep')

package() {
    depends+=('extra-dep' 'another-dep')
    cd source
    make install
}

build() {
    makedepends+=(cmake ninja)
    cmake -B build
}
";

    let (depends, makedepends, _, _) = parse_pkgbuild_deps(append_syntax);
    println!("Dependencies (including += additions):");
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nMake Dependencies (including += additions):");
    for dep in &makedepends {
        println!("  - {dep}");
    }
    println!("\nNote: Both = and += syntax are supported");

    // ========================================================================
    // Example 5: Unquoted Dependencies
    // ========================================================================
    println!("\n┌─ Example 5: Unquoted Dependencies ─────────────────────────────┐");
    println!("│ Parse unquoted dependencies in arrays                          │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let unquoted = r"
depends=(foo bar libcairo.so libdbus-1.so)
";

    let (depends, _, _, _) = parse_pkgbuild_deps(unquoted);
    println!("Dependencies (unquoted, .so files filtered):");
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nNote: .so virtual packages are automatically filtered out");

    // ========================================================================
    // Example 6: Mixed Quoted and Unquoted
    // ========================================================================
    println!("\n┌─ Example 6: Mixed Quoted and Unquoted ─────────────────────────┐");
    println!("│ Handle arrays with both quoted and unquoted elements           │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let mixed = r"
depends=('quoted-package' unquoted-package 'another-quoted' unquoted2)
";

    let (depends, _, _, _) = parse_pkgbuild_deps(mixed);
    println!("Dependencies (mixed quoted/unquoted):");
    for dep in &depends {
        println!("  - {dep}");
    }

    // ========================================================================
    // Example 7: Parsing Conflicts
    // ========================================================================
    println!("\n┌─ Example 7: Parsing Conflicts ─────────────────────────────────┐");
    println!("│ Extract conflicts from PKGBUILD                                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let conflicts_content = r"
pkgname=jujutsu-git
pkgver=0.1.0
conflicts=('jujutsu')
";

    let conflicts = parse_pkgbuild_conflicts(conflicts_content);
    println!("Conflicts:");
    for conflict in &conflicts {
        println!("  - {conflict}");
    }

    // ========================================================================
    // Example 8: Conflicts with Version Constraints
    // ========================================================================
    println!("\n┌─ Example 8: Conflicts with Version Constraints ────────────────┐");
    println!("│ Version constraints are stripped from conflict names            │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let conflicts_versions = r"
conflicts=('old-pkg<2.0' 'new-pkg>=3.0' 'exact-pkg=1.5.0')
";

    let conflicts = parse_pkgbuild_conflicts(conflicts_versions);
    println!("Conflicts (version constraints removed):");
    for conflict in &conflicts {
        println!("  - {conflict}");
    }
    println!("\nNote: Version constraints are automatically removed");

    // ========================================================================
    // Example 9: Multi-Line Conflicts
    // ========================================================================
    println!("\n┌─ Example 9: Multi-Line Conflicts ──────────────────────────────┐");
    println!("│ Parse multi-line conflicts arrays                              │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let multiline_conflicts = r"
pkgname=pacsea-git
pkgver=0.1.0
conflicts=(
    'pacsea'
    'pacsea-bin'
    'pacsea-stable'
)
";

    let conflicts = parse_pkgbuild_conflicts(multiline_conflicts);
    println!("Conflicts from multi-line array:");
    for conflict in &conflicts {
        println!("  - {conflict}");
    }

    // ========================================================================
    // Example 10: Filtering Virtual Packages (.so files)
    // ========================================================================
    println!("\n┌─ Example 10: Filtering Virtual Packages ───────────────────────┐");
    println!("│ Automatic filtering of .so virtual packages                    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let virtual_pkg = r"
depends=('glibc' 'libedit.so' 'libgit2.so.1' 'libfoo.so=1-64' 'python')
conflicts=('foo' 'libcairo.so' 'bar' 'libdbus-1.so=1-64')
";

    let (depends, _, _, _) = parse_pkgbuild_deps(virtual_pkg);
    let conflicts = parse_pkgbuild_conflicts(virtual_pkg);

    println!("Dependencies after .so filtering:");
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nConflicts after .so filtering:");
    for conflict in &conflicts {
        println!("  - {conflict}");
    }
    println!("\nNote: All .so virtual packages are automatically filtered out");

    // ========================================================================
    // Example 11: Deduplication
    // ========================================================================
    println!("\n┌─ Example 11: Automatic Deduplication ───────────────────────────┐");
    println!("│ Duplicate dependencies are automatically removed                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let duplicates = r"
depends=('glibc' 'gtk3' 'glibc' 'nss' 'gtk3')
";

    let (depends, _, _, _) = parse_pkgbuild_deps(duplicates);
    println!("Dependencies after deduplication:");
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nTotal unique dependencies: {}", depends.len());
    println!("Note: Duplicates are automatically removed");

    // ========================================================================
    // Example 12: Comments and Blank Lines
    // ========================================================================
    println!("\n┌─ Example 12: Comments and Blank Lines ──────────────────────────┐");
    println!("│ Comments and blank lines are automatically ignored              │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let commented = r"
# This is a comment
pkgname=test-package

depends=('foo' 'bar')
# Another comment
makedepends=('make')
";

    let (depends, makedepends, _, _) = parse_pkgbuild_deps(commented);
    println!("Parsed dependencies from PKGBUILD with comments:");
    println!("  Dependencies: {depends:?}");
    println!("  Make Dependencies: {makedepends:?}");
    println!("\nNote: All comments (lines starting with #) are ignored");
    println!("Note: Blank lines are also ignored");

    // ========================================================================
    // Example 13: Ignoring Other PKGBUILD Fields
    // ========================================================================
    println!("\n┌─ Example 13: Ignoring Other PKGBUILD Fields ────────────────────┐");
    println!("│ Only dependency fields are parsed, others are ignored          │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let other_fields = r"
pkgname=test-package
pkgver=1.0.0
pkgdesc=Test package description
url=https://example.com
license=(MIT)
arch=(x86_64)
source=(package-1.0.0.tar.gz)
depends=('foo' 'bar')
makedepends=('make')
";

    let (depends, makedepends, _, _) = parse_pkgbuild_deps(other_fields);
    println!("Only dependency fields are parsed:");
    println!("  Dependencies: {depends:?}");
    println!("  Make Dependencies: {makedepends:?}");
    println!("\nNote: Fields like arch, pkgdesc, url, license, source are ignored");

    // ========================================================================
    // Example 14: Filtering Invalid Package Names
    // ========================================================================
    println!("\n┌─ Example 14: Filtering Invalid Package Names ───────────────────┐");
    println!("│ Invalid package names are automatically filtered               │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let invalid_names = r"
depends=('valid-package' 'invalid)' '=invalid' 'a' 'valid>=1.0')
";

    let (depends, _, _, _) = parse_pkgbuild_deps(invalid_names);
    println!("Valid dependencies (invalid names filtered):");
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nNote: Names ending with ), starting with =, or too short are filtered");

    // ========================================================================
    // Example 15: Real-World Package Example
    // ========================================================================
    println!("\n┌─ Example 15: Real-World Package Example ────────────────────────┐");
    println!("│ Parse a realistic PKGBUILD file                                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let real_world = r"
pkgname=firefox
pkgver=121.0
pkgrel=1
pkgdesc=Standalone web browser from mozilla.org
url=https://www.mozilla.org/firefox/
arch=(x86_64)
license=(MPL GPL LGPL)
depends=(
    glibc
    gtk3
    libpulse
    nss
    libxt
    libxss
    libxcomposite
    libx11
    libxcb
)
makedepends=(
    rust
    llvm
    clang
    nodejs
    cbindgen
    nasm
    python
)
optdepends=(
    'pulseaudio: audio support'
    'ffmpeg: H.264 video decoding'
)
";

    let (depends, makedepends, _, optdepends) = parse_pkgbuild_deps(real_world);
    println!("Package: firefox");
    println!("\nRuntime Dependencies ({} items):", depends.len());
    for dep in &depends {
        println!("  - {dep}");
    }
    println!("\nBuild Dependencies ({} items):", makedepends.len());
    for dep in &makedepends {
        println!("  - {dep}");
    }
    println!("\nOptional Dependencies ({} items):", optdepends.len());
    for dep in &optdepends {
        println!("  - {dep}");
    }

    // ========================================================================
    // Example 16: Complex Scenario (jujutsu-git)
    // ========================================================================
    println!("\n┌─ Example 16: Complex Scenario ──────────────────────────────────┐");
    println!("│ Handle complex PKGBUILD with various edge cases                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let complex = r"
pkgname=jujutsu-git
pkgver=0.1.0
pkgdesc=Git-compatible VCS that is both simple and powerful
url=https://github.com/martinvonz/jj
license=(Apache-2.0)
arch=(i686 x86_64 armv6h armv7h)
depends=(
    glibc
    libc.so
    libm.so
)
makedepends=(
    libgit2
    libgit2.so
    libssh2
    libssh2.so)
    openssh
    git)
cargo
checkdepends=()
optdepends=()
conflicts=('jujutsu')
source=(jujutsu-git::git+https://github.com/martinvonz/jj)
";

    let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(complex);
    let conflicts = parse_pkgbuild_conflicts(complex);

    println!(
        "Dependencies ({} items, .so files filtered):",
        depends.len()
    );
    for dep in &depends {
        println!("  - {dep}");
    }
    println!(
        "\nMake Dependencies ({} items, .so files filtered):",
        makedepends.len()
    );
    for dep in &makedepends {
        println!("  - {dep}");
    }
    println!("\nCheck Dependencies: {} items", checkdepends.len());
    println!("\nOptional Dependencies: {} items", optdepends.len());
    println!("\nConflicts:");
    for conflict in &conflicts {
        println!("  - {conflict}");
    }
    println!("\nNote: .so files are filtered, invalid parsing artifacts are ignored");

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete                            ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
}
