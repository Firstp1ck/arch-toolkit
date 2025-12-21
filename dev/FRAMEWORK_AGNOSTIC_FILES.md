# Framework-Agnostic Source Files

This document lists source files and directories in `src/` that are independent of the `ratatui`/`crossterm` frameworks and could be reused with other UI frameworks (e.g., GTK, Qt, web frontends).

## Framework-Dependent Files (Excluded)

The following directories/files are tightly coupled to `ratatui`/`crossterm` and would need significant refactoring:

- `src/ui/` - All UI rendering code uses `ratatui` widgets and layout
- `src/events/` - Event handling uses `crossterm` for keyboard/mouse events
- `src/app/terminal.rs` - Terminal setup/restore using `crossterm`
- `src/app/runtime/` - Event loop and terminal management (uses `ratatui::Terminal` and `crossterm` events)
  - `src/app/runtime/event_loop.rs` - Main event loop with `ratatui::Terminal`
  - `src/app/runtime/background.rs` - Background worker spawning
  - `src/app/runtime/channels.rs` - Channel definitions using `crossterm::event::Event`
  - `src/app/runtime/cleanup.rs` - Cleanup operations
  - `src/app/runtime/tick_handler.rs` - Tick handler for UI updates
  - `src/app/runtime/workers/auxiliary.rs` - Uses `crossterm::event::Event`
- `src/main.rs` - Entry point that initializes the TUI framework
- `src/state/app_state/mod.rs` - Uses `ratatui::widgets::ListState`
- `src/state/app_state/defaults.rs` - Uses `ratatui::widgets::ListState`
- `src/state/app_state/methods.rs` - Uses `ratatui::widgets::ListState`
- `src/state/app_state/constants.rs` - Framework-agnostic constants
- `src/state/app_state/defaults_cache.rs` - Framework-agnostic caching
- `src/theme/types.rs` - Uses `crossterm::event::KeyCode` and `ratatui::style::Color`
- `src/theme/parsing.rs` - Uses `crossterm::event::KeyCode` and `ratatui::style::Color`

## Framework-Agnostic Files (Reusable)

### Core Business Logic

#### `src/logic/`
All core business logic modules:
- `src/logic/deps/` - Dependency resolution and analysis
  - `aur.rs` - AUR dependency parsing
  - `parse.rs` - Dependency string parsing
  - `query.rs` - Dependency queries
  - `resolve.rs` - Dependency resolution
  - `reverse.rs` - Reverse dependency analysis
  - `source.rs` - Dependency source handling
  - `srcinfo.rs` - SRCINFO parsing
  - `status.rs` - Dependency status checking
  - `utils.rs` - Dependency utilities
- `src/logic/deps.rs` - Dependency module
- `src/logic/distro.rs` - Distribution detection
- `src/logic/faillock.rs` - Faillock detection
- `src/logic/files/` - File operations
  - `backup.rs` - Backup operations
  - `db_sync.rs` - Database synchronization
  - `lists.rs` - File list operations
  - `mod.rs` - File module
  - `pkgbuild_cache.rs` - PKGBUILD caching
  - `pkgbuild_fetch.rs` - PKGBUILD fetching
  - `pkgbuild_parse.rs` - PKGBUILD parsing
  - `resolution.rs` - File resolution
  - `tests.rs` - File tests
- `src/logic/filter.rs` - Package filtering
- `src/logic/gating.rs` - Gating logic
- `src/logic/lists.rs` - Package list operations
- `src/logic/mod.rs` - Logic module
- `src/logic/password.rs` - Password handling
- `src/logic/prefetch.rs` - Prefetching logic
- `src/logic/preflight/` - Preflight analysis
  - `batch.rs` - Batch preflight
  - `command.rs` - Command preflight
  - `metadata.rs` - Metadata preflight
  - `mod.rs` - Preflight module
  - `tests.rs` - Preflight tests
  - `version.rs` - Version preflight
- `src/logic/query.rs` - Package queries
- `src/logic/sandbox/` - Sandbox analysis
  - `analyze.rs` - Sandbox analysis
  - `fetch.rs` - Sandbox fetching
  - `mod.rs` - Sandbox module
  - `parse.rs` - Sandbox parsing
  - `tests.rs` - Sandbox tests
  - `types.rs` - Sandbox types
- `src/logic/selection.rs` - Package selection
- `src/logic/services/` - Service management
  - `binaries.rs` - Service binary detection
  - `command.rs` - Service commands
  - `mod.rs` - Service module
  - `systemd.rs` - Systemd integration
  - `tests.rs` - Service tests
  - `units.rs` - Service units
- `src/logic/sort.rs` - Package sorting
- `src/logic/summary.rs` - Summary generation

### Package Indexing

#### `src/index/`
Package indexing and caching:
- `src/index/distro.rs` - Distribution-specific indexing
- `src/index/enrich.rs` - Package enrichment
- `src/index/explicit.rs` - Explicit package indexing
- `src/index/fetch.rs` - Package fetching
- `src/index/installed.rs` - Installed package indexing
- `src/index/mirrors.rs` - Mirror management
- `src/index/mod.rs` - Index module
- `src/index/persist.rs` - Index persistence
- `src/index/query.rs` - Index queries
- `src/index/update.rs` - Index updates

### Installation Logic

#### `src/install/`
Package installation and removal:
- `src/install/batch.rs` - Batch installation
- `src/install/command.rs` - Installation commands
- `src/install/direct.rs` - Direct installation
- `src/install/executor.rs` - Command execution
- `src/install/logging.rs` - Installation logging
- `src/install/mod.rs` - Install module
- `src/install/patterns.rs` - Installation patterns
- `src/install/remove/` - Removal logic
  - `tests.rs` - Removal tests
- `src/install/remove.rs` - Package removal
- `src/install/scan/` - Package scanning
  - `common.rs` - Scan common utilities
  - `dir.rs` - Directory scanning
  - `mod.rs` - Scan module
  - `pkg.rs` - Package scanning
  - `spawn.rs` - Scan spawning
  - `summary.rs` - Scan summary
- `src/install/shell.rs` - Shell operations
- `src/install/single.rs` - Single package installation
- `src/install/utils.rs` - Installation utilities

### Data Sources

#### `src/sources/`
External data source integration:
- `src/sources/advisories.rs` - Security advisories
- `src/sources/comments.rs` - AUR comments
- `src/sources/details.rs` - Package details
- `src/sources/feeds/` - News feeds
  - `cache.rs` - Feed caching
  - `helpers.rs` - Feed helpers
  - `mod.rs` - Feed module
  - `news_fetch.rs` - News fetching
  - `rate_limit.rs` - Rate limiting
  - `tests.rs` - Feed tests
  - `updates.rs` - Feed updates
- `src/sources/mod.rs` - Sources module
- `src/sources/news/` - News handling
  - `aur.rs` - AUR news
  - `cache.rs` - News caching
  - `fetch.rs` - News fetching
  - `mod.rs` - News module
  - `parse.rs` - News parsing
  - `tests_aur.rs` - AUR news tests
  - `tests.rs` - News tests
  - `utils.rs` - News utilities
- `src/sources/pkgbuild.rs` - PKGBUILD handling
- `src/sources/search.rs` - Package search
- `src/sources/status/` - Status checking
  - `api.rs` - Status API
  - `html.rs` - HTML parsing
  - `mod.rs` - Status module
  - `translate.rs` - Status translation
  - `utils.rs` - Status utilities

### Internationalization

#### `src/i18n/`
Internationalization support:
- `src/i18n/detection.rs` - Locale detection
- `src/i18n/loader.rs` - Translation loading
- `src/i18n/mod.rs` - I18n module
- `src/i18n/resolver.rs` - Translation resolution
- `src/i18n/translations.rs` - Translation management

### Command-Line Arguments

#### `src/args/`
CLI argument parsing and processing:
- `src/args/args.rs` - Argument definitions
- `src/args/cache.rs` - Cache operations
- `src/args/definition.rs` - Argument definitions
- `src/args/i18n.rs` - I18n arguments
- `src/args/install.rs` - Install arguments
- `src/args/list.rs` - List arguments
- `src/args/mod.rs` - Args module
- `src/args/news.rs` - News arguments
- `src/args/package.rs` - Package arguments
- `src/args/remove.rs` - Remove arguments
- `src/args/search.rs` - Search arguments
- `src/args/update.rs` - Update arguments
- `src/args/utils.rs` - Argument utilities

### State Management

#### `src/state/`
Application state (framework-agnostic types):
- `src/state/mod.rs` - State module
- `src/state/types.rs` - Core state types (PackageItem, PackageDetails, Source, AppMode, NewsItem, etc.)
- `src/state/modal.rs` - Modal state types (PreflightAction, Modal enum, etc.)
- `src/state/app_state/constants.rs` - Application state constants (framework-agnostic)
- `src/state/app_state/defaults_cache.rs` - Defaults caching (framework-agnostic)

### Theme and Configuration

#### `src/theme/`
Theme and configuration (mostly framework-agnostic):
- `src/theme/config/` - Theme configuration
  - `settings_ensure.rs` - Settings validation
  - `settings_save.rs` - Settings persistence
  - `skeletons.rs` - Config skeletons
  - `tests.rs` - Config tests
  - `theme_loader.rs` - Theme loading
- `src/theme/config.rs` - Theme config module
- `src/theme/mod.rs` - Theme module
- `src/theme/paths.rs` - Config path resolution
- `src/theme/store.rs` - Theme storage
- `src/theme/settings/` - Settings parsing
  - `mod.rs` - Settings module
  - `normalize.rs` - Settings normalization
  - `parse_settings.rs` - Settings parsing
  - `parse_keybinds.rs` - Keybind parsing (uses crossterm types only in tests)

**Note:** `src/theme/types.rs` and `src/theme/parsing.rs` use framework types (`crossterm::event::KeyCode`, `ratatui::style::Color`) but could be abstracted with trait-based design.

### Utilities

#### `src/util/`
Utility functions:
- `src/util/config.rs` - Configuration utilities
- `src/util/curl.rs` - cURL command building
- `src/util/mod.rs` - Utility module (mostly framework-agnostic, except `ensure_mouse_capture()`)
- `src/util/pacman.rs` - Pacman command utilities
- `src/util/srcinfo.rs` - SRCINFO utilities

**Note:** `src/util/mod.rs` contains `ensure_mouse_capture()` which uses `crossterm`, but this could be made optional or abstracted.

### Application State and Caching

#### `src/app/`
Application-level modules (excluding runtime and terminal):
- `src/app/deps_cache.rs` - Dependency caching
- `src/app/files_cache.rs` - File caching
- `src/app/mod.rs` - App module
- `src/app/persist.rs` - State persistence
- `src/app/recent.rs` - Recent queries
- `src/app/sandbox_cache.rs` - Sandbox caching
- `src/app/services_cache.rs` - Services caching

**Note:** `src/app/runtime/handlers/` contains framework-agnostic business logic handlers, but they operate on `AppState` which includes `ratatui::widgets::ListState`. The handler logic itself (dependency resolution, file operations, etc.) is framework-agnostic and could be adapted.

### Other Files

- `src/announcements.rs` - Announcement handling
- `src/lib.rs` - Library entry point
- `src/test_utils.rs` - Test utilities

## Summary

The majority of Pacsea's core functionality is framework-agnostic:

- **Business Logic**: All dependency resolution, package management, installation, and analysis logic
- **Data Sources**: AUR integration, news feeds, package indexing
- **State Management**: Core data structures and state types
- **Configuration**: Settings and theme loading (with minor framework dependencies)
- **Utilities**: Most utility functions for package operations

To adapt Pacsea for a different UI framework (GTK, Qt, web, etc.), you would primarily need to:

1. Replace `src/ui/` with framework-specific rendering code
2. Replace `src/events/` with framework-specific event handling
3. Abstract the framework-specific types in `src/theme/types.rs` and `src/theme/parsing.rs`
4. Replace `src/app/runtime/` with framework-specific event loop (the handlers in `src/app/runtime/handlers/` contain reusable business logic)
5. Replace `src/app/terminal.rs` with framework-specific initialization
6. Abstract `ratatui::widgets::ListState` usage in `src/state/app_state/mod.rs`, `src/state/app_state/defaults.rs`, and `src/state/app_state/methods.rs`
7. Make `ensure_mouse_capture()` in `src/util/mod.rs` optional or framework-specific

The core business logic, data fetching, state management, and utilities would remain largely unchanged. The handlers in `src/app/runtime/handlers/` contain framework-agnostic business logic that could be reused with minimal changes.

