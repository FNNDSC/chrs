use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

/// Create a progress bar with bytes units.
pub fn progress_bar_bytes(len: u64) -> ProgressBar {
    let stderr = ProgressDrawTarget::stderr_with_hz(2);
    ProgressBar::with_draw_target(Some(len), stderr).with_style(bytes_style())
}

/// Progress bar style.
fn bytes_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template(
            "[{elapsed_precise}] {wide_bar} ({bytes}/{total_bytes} @ {bytes_per_sec}, ETA {eta})",
        )
        .unwrap()
}
