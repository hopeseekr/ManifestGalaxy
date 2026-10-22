//! Headless acceptance tests for the desktop shell model and worker protocol.

use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use stars_desktop::StarsApp;
use stars_desktop::preview::{DEFAULT_PREVIEW_COLUMNS, PreviewSnapshot, preview_snapshot};
use stars_desktop::shell::{ApplyOutcome, ShellActivity, ShellModel};
use stars_desktop::worker::{RequestId, SyntheticJob, WorkerError, WorkerHandle, WorkerMessage};

fn make_snapshot(request_id: RequestId, seed: u64) -> PreviewSnapshot {
    preview_snapshot(request_id, seed, 6, DEFAULT_PREVIEW_COLUMNS)
}

#[test]
fn blank_shell_starts_idle_without_a_snapshot() {
    let shell = ShellModel::new();

    assert_eq!(shell.activity(), &ShellActivity::Idle);
    assert_eq!(shell.active_request(), None);
    assert_eq!(shell.last_accepted_request(), None);
    assert!(shell.snapshot().is_none());
    assert_eq!(shell.progress_fraction(), None);
}

#[test]
fn progress_is_visible_before_an_atomic_commit() {
    let mut shell = ShellModel::new();
    let request = RequestId::new(10);

    assert!(shell.begin_request(request, 6));
    assert_eq!(
        shell.apply(WorkerMessage::Progress {
            request_id: request,
            completed_rows: 2,
            total_rows: 6,
        }),
        ApplyOutcome::Applied
    );

    assert!(shell.snapshot().is_none());
    assert_eq!(shell.progress_fraction(), Some(2.0 / 6.0));
    assert!(shell.status_text().contains("2/6"));

    assert_eq!(
        shell.apply(WorkerMessage::Completed {
            request_id: request,
            snapshot: make_snapshot(request, 77),
        }),
        ApplyOutcome::Applied
    );
    assert_eq!(
        shell.snapshot(),
        Some(&make_snapshot(request, 77)),
        "the committed snapshot must equal the completed worker result"
    );
    assert_eq!(shell.last_accepted_request(), Some(request));
}

#[test]
fn worker_failure_retains_the_last_accepted_ui_snapshot() {
    let mut shell = ShellModel::new();
    let first = RequestId::new(20);
    let second = RequestId::new(21);

    shell.begin_request(first, 6);
    shell.apply(WorkerMessage::Completed {
        request_id: first,
        snapshot: make_snapshot(first, 1),
    });
    let accepted = shell.snapshot().cloned().expect("accepted snapshot");

    shell.begin_request(second, 6);
    shell.apply(WorkerMessage::Failed {
        request_id: second,
        error: WorkerError::SyntheticFailure { row: 2 },
    });

    assert_eq!(shell.snapshot(), Some(&accepted));
    assert_eq!(shell.last_accepted_request(), Some(first));
    assert_eq!(shell.active_request(), None);
    assert!(matches!(
        shell.activity(),
        ShellActivity::Failed { request_id, .. } if *request_id == second
    ));
}

#[test]
fn stale_results_cannot_overwrite_a_newer_request() {
    let mut shell = ShellModel::new();
    let older = RequestId::new(30);
    let newer = RequestId::new(31);

    shell.begin_request(older, 6);
    shell.begin_request(newer, 6);

    assert_eq!(
        shell.apply(WorkerMessage::Completed {
            request_id: older,
            snapshot: make_snapshot(older, 2),
        }),
        ApplyOutcome::IgnoredStale
    );
    assert!(shell.snapshot().is_none());

    assert_eq!(
        shell.apply(WorkerMessage::Completed {
            request_id: newer,
            snapshot: make_snapshot(newer, 3),
        }),
        ApplyOutcome::Applied
    );
    assert_eq!(shell.last_accepted_request(), Some(newer));
}

#[test]
fn worker_reports_monotonic_progress_then_a_snapshot() {
    let (worker, messages) = WorkerHandle::spawn().expect("worker starts");
    let request = RequestId::new(40);
    let rows = 5;
    let columns = 8;

    worker
        .submit(SyntheticJob::new(
            request,
            request.value(),
            rows,
            columns,
            Duration::ZERO,
        ))
        .expect("job submitted");

    let mut previous_progress = 0;
    let completed = loop {
        let message = messages
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|error| panic!("worker did not finish: {error}"));

        match message {
            WorkerMessage::Progress {
                request_id,
                completed_rows,
                total_rows,
            } => {
                assert_eq!(request_id, request);
                assert_eq!(total_rows, rows);
                assert!(completed_rows > previous_progress);
                previous_progress = completed_rows;
            }
            WorkerMessage::Completed {
                request_id,
                snapshot,
            } => {
                assert_eq!(request_id, request);
                break snapshot;
            }
            WorkerMessage::Failed { error, .. } => panic!("unexpected worker failure: {error}"),
        }
    };

    worker.shutdown();
    assert_eq!(previous_progress, rows);
    assert_eq!(
        completed,
        preview_snapshot(request, request.value(), rows, columns)
    );
}

#[test]
fn injected_worker_failure_is_reported_with_its_request_id() {
    let (worker, messages) = WorkerHandle::spawn().expect("worker starts");
    let request = RequestId::new(50);

    worker
        .submit(SyntheticJob::new(request, 9, 8, 8, Duration::ZERO).failing_at(3))
        .expect("job submitted");

    loop {
        match messages.recv_timeout(Duration::from_secs(10)) {
            Ok(WorkerMessage::Failed { request_id, error }) => {
                assert_eq!(request_id, request);
                assert_eq!(error, WorkerError::SyntheticFailure { row: 3 });
                break;
            }
            Ok(WorkerMessage::Progress { .. }) => {}
            Ok(other) => panic!("unexpected worker message: {other:?}"),
            Err(RecvTimeoutError::Timeout) => panic!("worker did not report its failure"),
            Err(RecvTimeoutError::Disconnected) => panic!("worker disconnected"),
        }
    }

    worker.shutdown();
}

#[test]
fn shutdown_interrupts_a_long_step_delay_instead_of_waiting_it_out() {
    let (worker, messages) = WorkerHandle::spawn().expect("worker starts");
    let request = RequestId::new(60);

    worker
        .submit(SyntheticJob::new(
            request,
            11,
            4,
            8,
            Duration::from_secs(30),
        ))
        .expect("job submitted");

    let first = messages
        .recv_timeout(Duration::from_secs(10))
        .expect("worker reports its first row");
    assert!(
        matches!(
            first,
            WorkerMessage::Progress {
                request_id,
                completed_rows: 1,
                total_rows: 4,
            } if request_id == request
        ),
        "expected progress for the first row, got {first:?}"
    );

    let started = Instant::now();
    worker.shutdown();
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "shutdown waited {elapsed:?} for a 30s step delay; the delay must observe cancellation"
    );
}

#[test]
fn shell_layout_runs_at_required_scales_and_arbitrary_aspect_ratios() {
    let mut app = StarsApp::new().expect("app starts");
    let context = eframe::egui::Context::default();

    for (scale, width, height) in [
        (1.0_f32, 640.0_f32, 420.0_f32),
        (1.5, 1280.0, 720.0),
        (2.0, 1000.0, 1000.0),
        (1.0, 1600.0, 600.0),
    ] {
        app.set_ui_scale(scale);
        assert!((app.ui_scale() - scale).abs() < f32::EPSILON);

        for _ in 0..2 {
            let input = eframe::egui::RawInput {
                screen_rect: Some(eframe::egui::Rect::from_min_size(
                    eframe::egui::Pos2::ZERO,
                    eframe::egui::vec2(width, height),
                )),
                ..Default::default()
            };
            let output = context.run_ui(input, |ui| app.draw(ui));
            output.drop_without_applying_deltas();
        }
    }

    assert!(app.shell().snapshot().is_none());
}
