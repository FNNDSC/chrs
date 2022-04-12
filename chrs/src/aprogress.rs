use futures::future::try_join_all;
use std::fmt::Debug;
use std::future::Future;
use tokio::join;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
/// Executes many futures with a progress bar. If any of the futures
/// produce an error, it is returned. (Ok values produced by futures
/// are ignored.)
///
/// Gotcha: elements of `tasks` may not have an opaque return type, i.e.
/// you probably can't use anonymous functions, closures, nor async blocks.
/// If necessary, wrap the functionality in a helper function with
/// an explicit function signature.
pub async fn do_with_progress<T: Debug, E: std::error::Error>(
    tasks: Vec<impl Future<Output = Result<T, E>>>,
) -> Result<(), E> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut handles = Vec::with_capacity(tasks.len());
    for task in tasks {
        let tx = tx.clone();
        handles.push(call_and_send(tx.clone(), task));
    }
    let count = handles.len();
    let main = async move {

        let bar = ProgressBar::new(count as u64).with_style(style());
        for _ in (0..count).progress_with(bar) {
            let _result = rx.recv().await.unwrap()?;
        }
        Ok(())
    };
    let (_, results) = join!(try_join_all(handles), main);
    results
}

/// A helper function to await the given future and send it
/// through the channel. This is necessary because the compiler
/// cannot infer the type of `E`.
async fn call_and_send<T: Debug, E: std::error::Error>(
    tx: UnboundedSender<Result<T, E>>,
    task: impl Future<Output = Result<T, E>>,
) -> Result<(), E> {
    tx.send(task.await).unwrap();
    Ok(())
}

fn style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {wide_bar} ({pos}/{len} @ {per_sec}, ETA {eta})").unwrap()
}
