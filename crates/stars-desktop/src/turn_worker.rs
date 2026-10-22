//! Desktop-owned adapter for core turn processing.
//!
//! The worker thread owns the authoritative [`GameSession`]. The UI submits
//! request-identified order batches and receives ordered phase progress,
//! exactly one completed [`TurnOutcome`], or a typed failure. A rejected batch
//! never mutates the session, so the worker continues from the last accepted
//! state. Stale request identifiers are rejected before execution and stale
//! messages are ignored by [`TurnSessionModel`].
//!
//! No egui types are used here, so the same protocol is exercised headlessly.

use std::fmt;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use stars_core::model::GameState;
use stars_core::orders::{OrderBatch, OrderError};
use stars_core::turn::{GameSession, Phase, PhaseProgress, TurnOutcome};
use stars_core::view::PlayerView;

use crate::worker::RequestId;

/// Typed failure from a turn request or the worker itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnWorkerError {
    /// Core rejected the order batch without changing authoritative state.
    Order(OrderError),
    /// The initial state could not form a valid session.
    SessionRejected {
        /// Validation detail.
        message: String,
    },
    /// The request identifier was not newer than the last accepted request.
    StaleRequest {
        /// Rejected request.
        request: RequestId,
        /// Last request seen by the worker, if any.
        last_request: Option<RequestId>,
    },
    /// The worker command channel is no longer available.
    CommandChannelClosed,
    /// The operating system refused to start the worker thread.
    SpawnFailed {
        /// Operating-system detail.
        message: String,
    },
}

impl fmt::Display for TurnWorkerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Order(error) => write!(formatter, "order batch rejected: {error}"),
            Self::SessionRejected { message } => {
                write!(formatter, "initial game state rejected: {message}")
            }
            Self::StaleRequest {
                request,
                last_request,
            } => match last_request {
                Some(last) => write!(
                    formatter,
                    "{request} is not newer than worker request {last}"
                ),
                None => write!(formatter, "{request} is not a fresh request"),
            },
            Self::CommandChannelClosed => {
                write!(formatter, "turn worker command channel is closed")
            }
            Self::SpawnFailed { message } => {
                write!(formatter, "failed to start turn worker thread: {message}")
            }
        }
    }
}

impl std::error::Error for TurnWorkerError {}

/// Message produced by the turn worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnWorkerMessage {
    /// One phase is about to run.
    Progress {
        /// Request being processed.
        request_id: RequestId,
        /// Ordered phase progress.
        progress: PhaseProgress,
    },
    /// The batch committed atomically and produced a complete snapshot.
    Completed {
        /// Request that completed.
        request_id: RequestId,
        /// Outcome of the committed batch, with the player-filtered view.
        outcome: Box<TurnOutcome>,
    },
    /// The request failed without changing authoritative state.
    Failed {
        /// Request that failed.
        request_id: RequestId,
        /// Typed failure.
        error: TurnWorkerError,
    },
}

impl TurnWorkerMessage {
    /// Request identifier attached to this message.
    #[must_use]
    pub const fn request_id(&self) -> RequestId {
        match self {
            Self::Progress { request_id, .. }
            | Self::Completed { request_id, .. }
            | Self::Failed { request_id, .. } => *request_id,
        }
    }
}

enum TurnWorkerCommand {
    Submit {
        request_id: RequestId,
        batch: OrderBatch,
    },
}

/// Handle to the desktop turn worker.
///
/// Dropping the handle requests shutdown and joins the thread.
pub struct TurnWorkerHandle {
    commands: Option<Sender<TurnWorkerCommand>>,
    join: Option<JoinHandle<()>>,
}

impl TurnWorkerHandle {
    /// Starts a worker that owns the given authoritative state.
    ///
    /// # Errors
    ///
    /// Returns [`TurnWorkerError::SessionRejected`] for an invalid state and
    /// [`TurnWorkerError::SpawnFailed`] if the thread cannot start.
    pub fn spawn(state: GameState) -> Result<(Self, Receiver<TurnWorkerMessage>), TurnWorkerError> {
        let session =
            GameSession::new(state).map_err(|error| TurnWorkerError::SessionRejected {
                message: error.to_string(),
            })?;
        let (command_sender, command_receiver) = mpsc::channel();
        let (message_sender, message_receiver) = mpsc::channel();

        let join = thread::Builder::new()
            .name("stars-turn-worker".to_owned())
            .spawn(move || turn_worker_loop(session, &command_receiver, &message_sender))
            .map_err(|error| TurnWorkerError::SpawnFailed {
                message: error.to_string(),
            })?;

        Ok((
            Self {
                commands: Some(command_sender),
                join: Some(join),
            },
            message_receiver,
        ))
    }

    /// Submits one request. Requests are executed in submission order.
    ///
    /// # Errors
    ///
    /// Returns [`TurnWorkerError::CommandChannelClosed`] if the worker stopped.
    pub fn submit(&self, request_id: RequestId, batch: OrderBatch) -> Result<(), TurnWorkerError> {
        self.commands
            .as_ref()
            .ok_or(TurnWorkerError::CommandChannelClosed)?
            .send(TurnWorkerCommand::Submit { request_id, batch })
            .map_err(|_| TurnWorkerError::CommandChannelClosed)
    }

    /// Stops the worker and joins its thread.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        // Dropping the sender closes the channel, so the worker exits its
        // receive loop immediately instead of draining a queued backlog.
        self.commands.take();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for TurnWorkerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn turn_worker_loop(
    mut session: GameSession,
    commands: &Receiver<TurnWorkerCommand>,
    messages: &Sender<TurnWorkerMessage>,
) {
    let mut last_request: Option<RequestId> = None;

    while let Ok(command) = commands.recv() {
        match command {
            TurnWorkerCommand::Submit { request_id, batch } => {
                if last_request.is_some_and(|last| request_id <= last) {
                    let _ = messages.send(TurnWorkerMessage::Failed {
                        request_id,
                        error: TurnWorkerError::StaleRequest {
                            request: request_id,
                            last_request,
                        },
                    });
                    continue;
                }
                last_request = Some(request_id);

                let result = session.commit_observed(&batch, |progress| {
                    let _ = messages.send(TurnWorkerMessage::Progress {
                        request_id,
                        progress,
                    });
                });

                match result {
                    Ok(outcome) => {
                        if messages
                            .send(TurnWorkerMessage::Completed {
                                request_id,
                                outcome: Box::new(outcome),
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => {
                        if messages
                            .send(TurnWorkerMessage::Failed {
                                request_id,
                                error: TurnWorkerError::Order(error),
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
        }
    }
}

/// Result of feeding one worker message into [`TurnSessionModel`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnApplyOutcome {
    /// Message applied to the active request.
    Applied,
    /// Message belongs to an older or unknown request and was ignored.
    IgnoredStale,
}

/// UI-side activity for the turn worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnActivity {
    /// No request has been started.
    Idle,
    /// A request is executing; the last committed snapshot remains current.
    Running {
        /// Active request.
        request_id: RequestId,
        /// Last reported phase, if any.
        phase: Option<Phase>,
    },
    /// A request committed and its snapshot is current.
    Applied {
        /// Request that committed.
        request_id: RequestId,
        /// New authoritative turn.
        turn: u32,
        /// New calendar year.
        year: u32,
    },
    /// A request failed; the previous snapshot remains current.
    Failed {
        /// Request that failed.
        request_id: RequestId,
        /// Human-readable failure reason.
        message: String,
    },
}

/// Headless UI model for the turn worker protocol.
#[derive(Debug)]
pub struct TurnSessionModel {
    active_request: Option<RequestId>,
    last_accepted_request: Option<RequestId>,
    view: Option<PlayerView>,
    activity: TurnActivity,
}

impl Default for TurnSessionModel {
    fn default() -> Self {
        Self::new()
    }
}

impl TurnSessionModel {
    /// Creates an idle model with no committed view.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active_request: None,
            last_accepted_request: None,
            view: None,
            activity: TurnActivity::Idle,
        }
    }

    /// Marks a newly submitted request as active.
    ///
    /// Returns `false` without changing state unless the identifier is strictly
    /// newer than both the active and last accepted request.
    pub fn begin_request(&mut self, request_id: RequestId) -> bool {
        if self
            .active_request
            .is_some_and(|active| request_id <= active)
            || self
                .last_accepted_request
                .is_some_and(|accepted| request_id <= accepted)
        {
            return false;
        }

        self.active_request = Some(request_id);
        self.activity = TurnActivity::Running {
            request_id,
            phase: None,
        };
        true
    }

    /// Applies one worker message atomically.
    ///
    /// Completion replaces the filtered view; the authoritative state stays in
    /// the worker session. A completion for a request newer than the last
    /// accepted one is always applied — even if that request was superseded
    /// while active — because the worker commits batches serially and its
    /// authoritative state already includes it. Such a completion does not
    /// clear a newer active request. Failure retains the last committed view.
    /// Other messages for a non-active request are ignored.
    pub fn apply(&mut self, message: TurnWorkerMessage) -> TurnApplyOutcome {
        match &message {
            TurnWorkerMessage::Progress {
                request_id,
                progress,
            } => {
                if self.active_request != Some(*request_id) {
                    return TurnApplyOutcome::IgnoredStale;
                }
                self.activity = TurnActivity::Running {
                    request_id: *request_id,
                    phase: Some(progress.phase),
                };
                TurnApplyOutcome::Applied
            }
            TurnWorkerMessage::Completed {
                request_id,
                outcome,
            } => {
                let is_active = self.active_request == Some(*request_id);
                if !is_active
                    && self
                        .last_accepted_request
                        .is_some_and(|accepted| *request_id <= accepted)
                {
                    return TurnApplyOutcome::IgnoredStale;
                }

                self.last_accepted_request = Some(*request_id);
                self.view = Some(outcome.view.clone());
                if is_active {
                    self.active_request = None;
                    self.activity = TurnActivity::Applied {
                        request_id: *request_id,
                        turn: outcome.turn,
                        year: outcome.year,
                    };
                }
                TurnApplyOutcome::Applied
            }
            TurnWorkerMessage::Failed { request_id, error } => {
                if self.active_request != Some(*request_id) {
                    return TurnApplyOutcome::IgnoredStale;
                }

                self.active_request = None;
                self.activity = TurnActivity::Failed {
                    request_id: *request_id,
                    message: error.to_string(),
                };
                TurnApplyOutcome::Applied
            }
        }
    }

    /// Active request, if any.
    #[must_use]
    pub const fn active_request(&self) -> Option<RequestId> {
        self.active_request
    }

    /// Last request whose outcome was committed.
    #[must_use]
    pub const fn last_accepted_request(&self) -> Option<RequestId> {
        self.last_accepted_request
    }

    /// Last committed player-filtered view, retained across failures.
    #[must_use]
    pub const fn view(&self) -> Option<&PlayerView> {
        self.view.as_ref()
    }

    /// Current status activity.
    #[must_use]
    pub const fn activity(&self) -> &TurnActivity {
        &self.activity
    }

    /// Short status text for a status area.
    #[must_use]
    pub fn status_text(&self) -> String {
        match &self.activity {
            TurnActivity::Idle => "No turn has been generated.".to_owned(),
            TurnActivity::Running { request_id, phase } => match phase {
                Some(phase) => format!("{request_id}: phase {}.", phase.name()),
                None => format!("{request_id}: turn generation started."),
            },
            TurnActivity::Applied {
                request_id,
                turn,
                year,
            } => format!("{request_id}: committed turn {turn} (year {year})."),
            TurnActivity::Failed {
                request_id,
                message,
            } => format!("{request_id} failed: {message}. Last committed snapshot retained."),
        }
    }
}
