//! Desktop-owned background worker adapter.
//!
//! The worker accepts request-identified synthetic jobs, reports progress,
//! and returns either a completed snapshot or a typed error. Only the desktop
//! package owns this thread; core game rules remain independent of threads and
//! wall-clock time.
//!
//! The job in this module deliberately generates placeholder map data. PR4 and
//! later turn-processing work will replace it with an ordered command batch
//! adapter that follows the same request/result discipline.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::preview::{PreviewSnapshot, preview_row};

/// Identifies one request submitted to the desktop worker.
///
/// Request identifiers are allocated by the callers and echoed in every
/// message. A response whose identifier is no longer the active request is
/// ignored so stale work can never overwrite newer state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(u64);

impl RequestId {
    /// Creates a request identifier from a monotonic counter value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the numeric counter value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "request-{}", self.0)
    }
}

/// Typed failure returned by a synthetic worker job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerError {
    /// The synthetic job was asked to fail at a specific row.
    SyntheticFailure { row: u32 },
    /// The worker command channel is no longer available.
    CommandChannelClosed,
    /// The operating system refused to start the worker thread.
    SpawnFailed { message: String },
}

impl fmt::Display for WorkerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SyntheticFailure { row } => {
                write!(formatter, "synthetic scan failed at row {row}")
            }
            Self::CommandChannelClosed => write!(formatter, "worker command channel is closed"),
            Self::SpawnFailed { message } => {
                write!(formatter, "failed to start worker thread: {message}")
            }
        }
    }
}

impl std::error::Error for WorkerError {}

/// A synthetic scan request handled by the desktop worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntheticJob {
    /// Request identifier echoed in every progress and result message.
    pub request_id: RequestId,
    /// Seed for the deterministic placeholder generator.
    pub seed: u64,
    /// Number of rows to inspect.
    pub rows: u32,
    /// Number of columns inspected per row.
    pub columns: u32,
    /// Delay between rows, used to make progress observable in the shell.
    pub step_delay: Duration,
    /// If set, the job reports an error when it reaches this row.
    pub fail_at_row: Option<u32>,
}

impl SyntheticJob {
    /// Creates a job without an injected failure.
    #[must_use]
    pub fn new(
        request_id: RequestId,
        seed: u64,
        rows: u32,
        columns: u32,
        step_delay: Duration,
    ) -> Self {
        Self {
            request_id,
            seed,
            rows,
            columns,
            step_delay,
            fail_at_row: None,
        }
    }

    /// Marks this job to fail when it reaches `row`.
    #[must_use]
    pub fn failing_at(mut self, row: u32) -> Self {
        self.fail_at_row = Some(row);
        self
    }
}

/// A message produced by the desktop worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerMessage {
    /// A job completed some rows.
    Progress {
        /// Request that produced this progress.
        request_id: RequestId,
        /// Rows completed so far.
        completed_rows: u32,
        /// Total rows in the request.
        total_rows: u32,
    },
    /// A job completed and produced an immutable snapshot.
    Completed {
        /// Request that produced the snapshot.
        request_id: RequestId,
        /// Completed synthetic preview.
        snapshot: PreviewSnapshot,
    },
    /// A job failed without producing a snapshot.
    Failed {
        /// Request that failed.
        request_id: RequestId,
        /// Typed failure reason.
        error: WorkerError,
    },
}

impl WorkerMessage {
    /// Returns the request identifier attached to this message.
    #[must_use]
    pub const fn request_id(&self) -> RequestId {
        match self {
            Self::Progress { request_id, .. }
            | Self::Completed { request_id, .. }
            | Self::Failed { request_id, .. } => *request_id,
        }
    }
}

enum WorkerCommand {
    Submit(SyntheticJob),
    Shutdown,
}

/// Handle to the desktop worker thread.
///
/// Dropping the handle requests shutdown and joins the thread, so closing the
/// window does not leave background work running.
pub struct WorkerHandle {
    commands: Option<Sender<WorkerCommand>>,
    cancelled: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl WorkerHandle {
    /// Starts a worker thread and returns its handle and message receiver.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError::SpawnFailed`] if the operating system cannot
    /// create the worker thread.
    pub fn spawn() -> Result<(Self, Receiver<WorkerMessage>), WorkerError> {
        let (command_sender, command_receiver) = mpsc::channel();
        let (message_sender, message_receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);

        let join = thread::Builder::new()
            .name("stars-desktop-worker".to_owned())
            .spawn(move || worker_loop(&command_receiver, &message_sender, &worker_cancelled))
            .map_err(|error| WorkerError::SpawnFailed {
                message: error.to_string(),
            })?;

        Ok((
            Self {
                commands: Some(command_sender),
                cancelled,
                join: Some(join),
            },
            message_receiver,
        ))
    }

    /// Submits one job. Jobs are handled in submission order.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError::CommandChannelClosed`] if the worker has stopped.
    pub fn submit(&self, job: SyntheticJob) -> Result<(), WorkerError> {
        self.commands
            .as_ref()
            .ok_or(WorkerError::CommandChannelClosed)?
            .send(WorkerCommand::Submit(job))
            .map_err(|_| WorkerError::CommandChannelClosed)
    }

    /// Stops the worker and waits for its thread to finish.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(commands) = self.commands.take() {
            let _ = commands.send(WorkerCommand::Shutdown);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn worker_loop(
    commands: &Receiver<WorkerCommand>,
    messages: &Sender<WorkerMessage>,
    cancelled: &AtomicBool,
) {
    while let Ok(command) = commands.recv() {
        match command {
            WorkerCommand::Submit(job) => run_job(&job, messages, cancelled),
            WorkerCommand::Shutdown => break,
        }
    }
}

fn run_job(job: &SyntheticJob, messages: &Sender<WorkerMessage>, cancelled: &AtomicBool) {
    let mut stars = Vec::new();

    for row in 0..job.rows {
        if cancelled.load(Ordering::SeqCst) {
            return;
        }

        if job.fail_at_row == Some(row) {
            let _ = messages.send(WorkerMessage::Failed {
                request_id: job.request_id,
                error: WorkerError::SyntheticFailure { row },
            });
            return;
        }

        stars.extend(preview_row(job.seed, row, job.rows, job.columns));

        let completed_rows = row.saturating_add(1);
        if messages
            .send(WorkerMessage::Progress {
                request_id: job.request_id,
                completed_rows,
                total_rows: job.rows,
            })
            .is_err()
        {
            return;
        }

        if !job.step_delay.is_zero() {
            thread::sleep(job.step_delay);
        }
    }

    if cancelled.load(Ordering::SeqCst) {
        return;
    }

    let snapshot = PreviewSnapshot {
        request_id: job.request_id,
        seed: job.seed,
        rows: job.rows,
        columns: job.columns,
        stars,
    };

    let _ = messages.send(WorkerMessage::Completed {
        request_id: job.request_id,
        snapshot,
    });
}
