//! Headless shell state for the desktop foundation.
//!
//! This model owns the worker protocol decisions: which request is current,
//! when a completed snapshot becomes visible, and whether a message is stale.
//! It contains no egui types, so the acceptance tests exercise the same logic
//! the window uses without opening a display.

use crate::preview::PreviewSnapshot;
use crate::worker::{RequestId, WorkerMessage};

/// Current activity shown in the status area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellActivity {
    /// No request has been started.
    Idle,
    /// The worker is processing the active request.
    Running {
        /// Active request.
        request_id: RequestId,
        /// Rows reported complete.
        completed_rows: u32,
        /// Rows requested.
        total_rows: u32,
    },
    /// The active request produced and committed a snapshot.
    Succeeded {
        /// Request whose snapshot is now displayed.
        request_id: RequestId,
    },
    /// The active request failed. The last accepted snapshot remains visible.
    Failed {
        /// Request that failed.
        request_id: RequestId,
        /// Human-readable failure reason.
        message: String,
    },
}

impl ShellActivity {
    /// Returns whether the status area should show an indeterminate spinner.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }
}

/// Result of feeding one worker message into the shell model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// The message applied to the active request.
    Applied,
    /// The message belongs to an older or unknown request and was ignored.
    IgnoredStale,
}

/// Headless state of the desktop shell.
#[derive(Debug)]
pub struct ShellModel {
    active_request: Option<RequestId>,
    last_accepted_request: Option<RequestId>,
    snapshot: Option<PreviewSnapshot>,
    activity: ShellActivity,
}

impl Default for ShellModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellModel {
    /// Creates an idle shell with no accepted snapshot.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active_request: None,
            last_accepted_request: None,
            snapshot: None,
            activity: ShellActivity::Idle,
        }
    }

    /// Marks a newly submitted request as active.
    ///
    /// Returns `false` without changing state when the identifier is not newer
    /// than the active or last accepted request. This keeps request ordering
    /// explicit rather than relying on message arrival order.
    pub fn begin_request(&mut self, request_id: RequestId, total_rows: u32) -> bool {
        if self
            .active_request
            .is_some_and(|active| request_id <= active)
        {
            return false;
        }
        if self
            .last_accepted_request
            .is_some_and(|accepted| request_id <= accepted)
        {
            return false;
        }

        self.active_request = Some(request_id);
        self.activity = ShellActivity::Running {
            request_id,
            completed_rows: 0,
            total_rows,
        };
        true
    }

    /// Applies one worker message if it belongs to the active request.
    ///
    /// Completion commits the snapshot atomically and only once. A failure
    /// clears the active request but retains the previous accepted snapshot.
    pub fn apply(&mut self, message: WorkerMessage) -> ApplyOutcome {
        match &message {
            WorkerMessage::Progress {
                request_id,
                completed_rows,
                total_rows,
            } => {
                if self.active_request != Some(*request_id) {
                    return ApplyOutcome::IgnoredStale;
                }

                self.activity = ShellActivity::Running {
                    request_id: *request_id,
                    completed_rows: (*completed_rows).min(*total_rows),
                    total_rows: *total_rows,
                };
                ApplyOutcome::Applied
            }
            WorkerMessage::Completed {
                request_id,
                snapshot,
            } => {
                if self.active_request != Some(*request_id) || snapshot.request_id != *request_id {
                    return ApplyOutcome::IgnoredStale;
                }

                self.active_request = None;
                self.last_accepted_request = Some(*request_id);
                self.snapshot = Some(snapshot.clone());
                self.activity = ShellActivity::Succeeded {
                    request_id: *request_id,
                };
                ApplyOutcome::Applied
            }
            WorkerMessage::Failed { request_id, error } => {
                if self.active_request != Some(*request_id) {
                    return ApplyOutcome::IgnoredStale;
                }

                self.active_request = None;
                self.activity = ShellActivity::Failed {
                    request_id: *request_id,
                    message: error.to_string(),
                };
                ApplyOutcome::Applied
            }
        }
    }

    /// Currently active request, if any.
    #[must_use]
    pub const fn active_request(&self) -> Option<RequestId> {
        self.active_request
    }

    /// Request whose snapshot was most recently committed.
    #[must_use]
    pub const fn last_accepted_request(&self) -> Option<RequestId> {
        self.last_accepted_request
    }

    /// Last accepted preview, retained across failures and newer pending requests.
    #[must_use]
    pub const fn snapshot(&self) -> Option<&PreviewSnapshot> {
        self.snapshot.as_ref()
    }

    /// Current status activity.
    #[must_use]
    pub const fn activity(&self) -> &ShellActivity {
        &self.activity
    }

    /// Progress fraction for the status area, if the active request reports rows.
    #[must_use]
    pub fn progress_fraction(&self) -> Option<f32> {
        match self.activity {
            ShellActivity::Running {
                completed_rows,
                total_rows,
                ..
            } if total_rows > 0 => Some(completed_rows as f32 / total_rows as f32),
            _ => None,
        }
    }

    /// Status text for the status area.
    #[must_use]
    pub fn status_text(&self) -> String {
        match &self.activity {
            ShellActivity::Idle => "Idle. Run a synthetic scan to exercise the worker.".to_owned(),
            ShellActivity::Running {
                request_id,
                completed_rows,
                total_rows,
            } => format!("Scanning {request_id}: {completed_rows}/{total_rows} rows complete."),
            ShellActivity::Succeeded { request_id } => {
                format!("{request_id} complete. Snapshot committed.")
            }
            ShellActivity::Failed {
                request_id,
                message,
            } => {
                format!("{request_id} failed: {message}. The last accepted snapshot is retained.")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyOutcome, ShellActivity, ShellModel};
    use crate::preview::{DEFAULT_PREVIEW_COLUMNS, preview_snapshot};
    use crate::worker::{RequestId, WorkerError, WorkerMessage};

    fn snapshot(request_id: RequestId) -> crate::preview::PreviewSnapshot {
        preview_snapshot(request_id, 42, 4, DEFAULT_PREVIEW_COLUMNS)
    }

    #[test]
    fn completion_is_atomic_and_applied_once() {
        let mut shell = ShellModel::new();
        let request = RequestId::new(1);
        assert!(shell.begin_request(request, 4));

        assert_eq!(
            shell.apply(WorkerMessage::Progress {
                request_id: request,
                completed_rows: 2,
                total_rows: 4,
            }),
            ApplyOutcome::Applied
        );
        assert!(shell.snapshot().is_none());

        assert_eq!(
            shell.apply(WorkerMessage::Completed {
                request_id: request,
                snapshot: snapshot(request),
            }),
            ApplyOutcome::Applied
        );
        assert_eq!(shell.last_accepted_request(), Some(request));
        assert_eq!(
            shell.snapshot().map(|value| value.request_id),
            Some(request)
        );

        assert_eq!(
            shell.apply(WorkerMessage::Completed {
                request_id: request,
                snapshot: snapshot(request),
            }),
            ApplyOutcome::IgnoredStale
        );
        assert_eq!(
            shell.activity(),
            &ShellActivity::Succeeded {
                request_id: request
            }
        );
    }

    #[test]
    fn failure_retains_the_last_accepted_snapshot() {
        let mut shell = ShellModel::new();
        let first = RequestId::new(1);
        let second = RequestId::new(2);

        shell.begin_request(first, 2);
        shell.apply(WorkerMessage::Completed {
            request_id: first,
            snapshot: snapshot(first),
        });
        let accepted = shell.snapshot().cloned().expect("first snapshot");

        assert!(shell.begin_request(second, 4));
        assert_eq!(
            shell.apply(WorkerMessage::Failed {
                request_id: second,
                error: WorkerError::SyntheticFailure { row: 1 },
            }),
            ApplyOutcome::Applied
        );

        assert_eq!(shell.active_request(), None);
        assert_eq!(shell.last_accepted_request(), Some(first));
        assert_eq!(shell.snapshot(), Some(&accepted));
        assert!(matches!(
            shell.activity(),
            ShellActivity::Failed {
                request_id,
                ..
            } if *request_id == second
        ));
    }

    #[test]
    fn stale_progress_and_results_are_ignored() {
        let mut shell = ShellModel::new();
        let first = RequestId::new(1);
        let second = RequestId::new(2);

        shell.begin_request(first, 4);
        shell.begin_request(second, 4);

        assert_eq!(
            shell.apply(WorkerMessage::Progress {
                request_id: first,
                completed_rows: 3,
                total_rows: 4,
            }),
            ApplyOutcome::IgnoredStale
        );
        assert_eq!(
            shell.apply(WorkerMessage::Completed {
                request_id: first,
                snapshot: snapshot(first),
            }),
            ApplyOutcome::IgnoredStale
        );
        assert!(shell.snapshot().is_none());
        assert_eq!(shell.active_request(), Some(second));

        assert_eq!(
            shell.apply(WorkerMessage::Completed {
                request_id: second,
                snapshot: snapshot(second),
            }),
            ApplyOutcome::Applied
        );
        assert_eq!(shell.last_accepted_request(), Some(second));
    }

    #[test]
    fn request_identifiers_must_increase() {
        let mut shell = ShellModel::new();
        let request = RequestId::new(3);

        assert!(shell.begin_request(request, 4));
        assert!(!shell.begin_request(request, 4));
        assert!(!shell.begin_request(RequestId::new(2), 4));

        shell.apply(WorkerMessage::Completed {
            request_id: request,
            snapshot: snapshot(request),
        });
        assert!(!shell.begin_request(RequestId::new(3), 4));
    }

    #[test]
    fn mismatched_snapshot_identifier_is_not_committed() {
        let mut shell = ShellModel::new();
        let request = RequestId::new(5);
        shell.begin_request(request, 4);

        assert_eq!(
            shell.apply(WorkerMessage::Completed {
                request_id: request,
                snapshot: snapshot(RequestId::new(4)),
            }),
            ApplyOutcome::IgnoredStale
        );
        assert!(shell.snapshot().is_none());
    }
}
