use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;

/// File transfer event.
#[derive(Debug)]
pub enum FileTransferEvent {
    /// Start of a new transfer of size *N*
    Start { id: usize, name: String, size: u64 },
    /// A chunk of *N* bytes was received
    Chunk { id: usize, delta: u64 },
    /// File transfer done
    Done(usize),
}

/// A [MultiProgress] wrapper for showing the upload or download progress of multiple files.
pub struct MultiFileTransferProgress {
    multi_progress: MultiProgress,
    overall_bar: ProgressBar,
    bars: HashMap<usize, ProgressBar>,
    size_threshold: u64,
    total_size: u64
}

impl MultiFileTransferProgress {
    /// Create a new multi-progress bar.
    pub fn new(total_files: u64, size_threshold: u64) -> Self {
        let multi_progress = MultiProgress::new();
        let overall_bar =
            multi_progress.add(ProgressBar::new(total_files).with_style(overall_style()));
        Self {
            multi_progress,
            overall_bar,
            bars: Default::default(),
            size_threshold,
            total_size: 0,
        }
    }

    /// Update this with an event.
    pub fn update(&mut self, event: FileTransferEvent) {
        match event {
            FileTransferEvent::Start { id, name, size } => self.add_file(id, name, size),
            FileTransferEvent::Chunk { id, delta } => self.on_chunk(id, delta),
            FileTransferEvent::Done(id) => self.finish_one(id),
            // FileTransferEvent::Println(msg) => self.println(msg)
        }
    }

    fn add_file(&mut self, id: usize, name: String, size: u64) {
        self.total_size += size;
        if size >= self.size_threshold {
            let bar = ProgressBar::new(size)
                .with_style(file_style())
                .with_prefix(name);
            self.bars.insert(id, self.multi_progress.add(bar));
        }
    }

    fn on_chunk(&self, id: usize, delta: u64) {
        if let Some(bar) = self.bars.get(&id) {
            bar.inc(delta)
        }
    }

    fn finish_one(&mut self, id: usize) {
        if let Some(bar) = self.bars.remove(&id) {
            self.multi_progress.remove(&bar);
        }
        self.overall_bar.inc(1);
    }

    /// Get the total size of all (attempted) transfers.
    pub fn total_size(&self) -> u64 {
        self.total_size
    }
}

fn overall_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {wide_bar} {human_pos}/{human_len} Files, ETA {eta}")
        .unwrap()
}

fn file_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{prefix} {wide_bar} {bytes}/{total_bytes} @ {bytes_per_sec}")
        .unwrap()
}
