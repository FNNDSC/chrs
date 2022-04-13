//! Helpers for executing futures concurrently, while limiting the number
//! of simultaneous executions (i.e. caps the size of the connection pool),
//! and with a progress bar.

use crate::constants::NUM_THREADS;
use futures::{pin_mut, stream, StreamExt, TryFuture, TryStream, TryStreamExt};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use itertools::Itertools;
use std::error::Error;
use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;
use tokio::join;
use tokio::sync::mpsc;

/// Executes many futures with a progress bar. If any of the futures
/// produce an error, it is returned. (Ok values produced by futures
/// are ignored.)
///
/// A maximum of [crate::constants::NUM_THREADS] futures will be
/// executed concurrently.
///
/// Gotcha: elements of `tasks` may not have an opaque return type, i.e.
/// you probably can't use anonymous functions, closures, nor async blocks.
/// If necessary, wrap the functionality in a helper function with
/// an explicit function signature.
///
/// # Arguments
///
/// * `tasks` - A stream which produces futures to execute
/// * `len` - Number of elements which will be produced by the given stream
/// * `hidden` - Whether to hide the progress bar
pub async fn do_with_progress<S, T: Debug, E: Error>(
    tasks: S,
    len: u64,
    hidden: bool,
) -> Result<(), E>
where
    S: Sized,
    S: TryStream<Error = E>,
    <S as TryStream>::Ok: TryFuture<Ok = T, Error = E>,
    <<S as TryStream>::Ok as TryFuture>::Ok: Debug,
    <<S as TryStream>::Ok as TryFuture>::Error: Error,
{
    let (tx, mut rx) = mpsc::unbounded_channel();

    let pool = async move {
        let buf = tasks.try_buffer_unordered(NUM_THREADS);
        pin_mut!(buf);
        while let Some(res) = buf.next().await {
            tx.send(res).unwrap();
        }
    };

    let main = async move {
        let bar = if hidden {
            ProgressBar::hidden()
        } else {
            ProgressBar::new(len as u64).with_style(style())
        };
        bar.enable_steady_tick(Duration::from_millis(200));
        for _ in (0..len).progress_with(bar) {
            let _result = rx.recv().await.unwrap()?;
        }
        Ok(())
    };

    let (_, results) = join!(pool, main);
    results
}

/// Calls [do_with_progress] on a sequence of futures which are produced
/// synchronously and without error.
pub async fn collect_then_do_with_progress<F, T: Debug, E: Error>(
    tasks: impl Iterator<Item = F>,
    hidden: bool,
) -> Result<(), E>
where
    F: Future<Output = Result<T, E>>,
{
    let proto_stream = tasks.map(coerce_as_result).collect_vec();
    let len = proto_stream.len() as u64;
    do_with_progress(stream::iter(proto_stream), len, hidden).await
}

/// A workaround to inform the compiler of an error type for items which do not cause errors.
fn coerce_as_result<T: TryFuture<Error = E>, E>(f: T) -> Result<T, E>
where
    <T as TryFuture>::Ok: Debug,
    <T as TryFuture>::Error: Error,
{
    Result::Ok(f)
}

/// Progress bar style.
fn style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {wide_bar} ({pos}/{len} @ {per_sec}, ETA {eta})")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_do_with_progress_ok() -> anyhow::Result<()> {
        let tasks = (0..5).into_iter().map(wrap_ok);
        let execution = collect_then_do_with_progress(tasks, true);
        let e = timeout(Duration::from_secs(1), execution)
            .await
            .context("Test timed out, the function is probably frozen.")?;
        e.context("Error from tasks, even though they're hard-coded to always be Ok")?;
        anyhow::Ok(())
    }

    #[tokio::test]
    async fn test_do_with_progress_err() -> anyhow::Result<()> {
        let tasks = vec![pretend_to_fail(false), pretend_to_fail(true)].into_iter();
        let execution = collect_then_do_with_progress(tasks, true);
        let e = timeout(Duration::from_secs(1), execution)
            .await
            .context("Test timed out, the function is probably frozen.")?;
        assert_eq!(
            e,
            Err(DummyError),
            "Should have returned the error from failing task."
        );
        anyhow::Ok(())
    }

    async fn wrap_ok<T>(t: T) -> Result<T, DummyError> {
        Ok(t)
    }

    async fn pretend_to_fail(fail: bool) -> Result<u8, DummyError> {
        if fail {
            Err(DummyError)
        } else {
            Ok(3)
        }
    }

    #[derive(thiserror::Error, Debug, PartialEq)]
    #[error("DummyError")]
    struct DummyError;
}
