//! Cancellable background index-refresh handle.

use std::future::Future;

use crate::error::{ArchToolkitError, Result};
use crate::types::index::OfficialIndex;

/// What: Represent one caller-supplied background official-index refresh.
///
/// Inputs:
/// - Created by [`spawn_index_refresh`] from a future that produces an
///   [`OfficialIndex`] or a structured toolkit error.
///
/// Output:
/// - A handle that can cancel pending async work and explicitly await its
///   result/error delivery.
///
/// Details:
/// - This handle owns no global index and performs no system mutation.
/// - Cancellation aborts the async task; cooperative futures such as caller
///   HTTP requests are dropped promptly, while caller code still owns any
///   external resource semantics.
#[derive(Debug)]
pub struct IndexRefreshHandle {
    /// Tokio task delivering the caller-supplied refresh result.
    task: tokio::task::JoinHandle<Result<OfficialIndex>>,
}

/// What: Start a caller-supplied async index refresh in the background.
///
/// Inputs:
/// - `refresh`: Sendable `'static` future returning a refreshed index or a
///   structured error.
///
/// Output:
/// - [`IndexRefreshHandle`] for cancellation and explicit result delivery.
///
/// Details:
/// - The API intentionally accepts a future rather than hiding `pacman` or
///   network policy. Callers can use a local fetch, a caller-client HTTP
///   fetcher, or an in-memory fixture with the same cancellation contract.
/// - Dropping the handle detaches the task; call [`IndexRefreshHandle::cancel`]
///   when a caller no longer wants the refresh to continue.
pub fn spawn_index_refresh<F>(refresh: F) -> IndexRefreshHandle
where
    F: Future<Output = Result<OfficialIndex>> + Send + 'static,
{
    IndexRefreshHandle {
        task: tokio::spawn(refresh),
    }
}

impl IndexRefreshHandle {
    /// What: Request cancellation of a pending background refresh.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - Requests task abortion; [`Self::wait`] reports a structured cancelled
    ///   error if the task did not complete first.
    ///
    /// Details:
    /// - Cancellation is idempotent and does not block the calling thread.
    /// - It is meaningful for async futures; callers should not wrap a
    ///   non-cancellable `spawn_blocking` operation in this API and expect it
    ///   to stop after it has started.
    pub fn cancel(&self) {
        self.task.abort();
    }

    /// What: Check whether the refresh task has completed.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - `true` after successful, failed, panicked, or cancelled completion.
    ///
    /// Details:
    /// - This is an observation only; use [`Self::wait`] for explicit result
    ///   or error delivery.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    /// What: Await the background refresh result and deliver task failures explicitly.
    ///
    /// Inputs:
    /// - `self`: Consumes the handle and awaits the owned task.
    ///
    /// Output:
    /// - Refreshed [`OfficialIndex`] or the refresh/task/cancellation error.
    ///
    /// Details:
    /// - A cancelled task maps to an actionable parse error instead of silently
    ///   returning an empty index. Panics and runtime failures also remain
    ///   visible to the caller.
    ///
    /// # Errors
    ///
    /// Returns the caller future's error or a descriptive task failure mapped to
    /// [`ArchToolkitError::Parse`].
    pub async fn wait(self) -> Result<OfficialIndex> {
        self.task
            .await
            .map_err(|error| map_refresh_join_error(&error))?
    }
}

/// What: Translate a Tokio task join error into a public toolkit error.
///
/// Inputs:
/// - `error`: Join failure observed while awaiting a refresh task.
///
/// Output:
/// - Actionable cancellation or task-failure error.
///
/// Details:
/// - Keeps cancellation distinguishable from a caller fetch error while
///   avoiding exposure of Tokio error types in the public return contract.
fn map_refresh_join_error(error: &tokio::task::JoinError) -> ArchToolkitError {
    if error.is_cancelled() {
        return ArchToolkitError::Parse("background index refresh was cancelled".to_string());
    }
    ArchToolkitError::Parse(format!("background index refresh task failed: {error}"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::error::Result;
    use crate::types::index::OfficialIndex;

    use super::spawn_index_refresh;

    #[tokio::test]
    /// What: Verify a background refresh delivers its successful index explicitly.
    ///
    /// Inputs:
    /// - A fixture-only async future returning an empty official index.
    ///
    /// Output:
    /// - The same completed index from [`IndexRefreshHandle::wait`].
    ///
    /// Details:
    /// - Demonstrates that the API returns a handle rather than hidden global
    ///   state or an unobservable detached task.
    async fn refresh_delivers_successful_result() {
        let handle =
            spawn_index_refresh(async { Result::<OfficialIndex>::Ok(OfficialIndex::default()) });
        let index = handle.wait().await.expect("refresh result");
        assert!(index.pkgs.is_empty());
    }

    #[tokio::test]
    /// What: Verify cancellation produces an explicit error on wait.
    ///
    /// Inputs:
    /// - A pending fixture-only async refresh future.
    ///
    /// Output:
    /// - A cancellation error instead of a false successful empty index.
    ///
    /// Details:
    /// - Uses Tokio time only; no system command or remote endpoint is touched.
    async fn refresh_cancellation_is_explicit() {
        let handle = spawn_index_refresh(async {
            tokio::time::sleep(Duration::from_mins(1)).await;
            Result::<OfficialIndex>::Ok(OfficialIndex::default())
        });
        handle.cancel();
        let error = handle.wait().await.expect_err("cancelled refresh error");
        assert!(error.to_string().contains("cancelled"));
    }
}
