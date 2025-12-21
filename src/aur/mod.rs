//! AUR (Arch User Repository) operations.

#[cfg(feature = "aur")]
mod comments;
#[cfg(feature = "aur")]
mod info;
#[cfg(feature = "aur")]
mod pkgbuild;
#[cfg(feature = "aur")]
mod search;
#[cfg(feature = "aur")]
mod utils;

#[cfg(feature = "aur")]
pub use comments::comments;
#[cfg(feature = "aur")]
pub use info::info;
#[cfg(feature = "aur")]
pub use pkgbuild::pkgbuild;
#[cfg(feature = "aur")]
pub use search::search;
