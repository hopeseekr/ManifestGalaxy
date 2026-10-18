//! Deterministic synthetic map data used by the desktop foundation shell.
//!
//! This is presentation data for exercising the background worker and the
//! renderer. It is not a game universe, a star catalog, or a gameplay rule.
//! Coordinates and tones are integers so the same seed always produces the
//! same preview on every platform.

use crate::worker::RequestId;

/// Number of rows in the default synthetic preview.
pub const DEFAULT_PREVIEW_ROWS: u32 = 16;

/// Number of columns inspected per synthetic preview row.
pub const DEFAULT_PREVIEW_COLUMNS: u32 = 24;

/// Integer coordinate range used by [`PreviewStar`].
pub const PREVIEW_COORDINATE_SCALE: u16 = 10_000;

/// One procedural placeholder star in the map preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewStar {
    /// Horizontal position in `0..PREVIEW_COORDINATE_SCALE`.
    pub x: u16,
    /// Vertical position in `0..PREVIEW_COORDINATE_SCALE`.
    pub y: u16,
    /// Visual brightness selector in `0..=255`.
    pub intensity: u8,
    /// Visual size selector in `0..=3`.
    pub size: u8,
}

/// A completed synthetic preview produced by one worker request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewSnapshot {
    /// Request that produced this snapshot.
    pub request_id: RequestId,
    /// Seed used by the procedural generator.
    pub seed: u64,
    /// Number of rows requested.
    pub rows: u32,
    /// Number of columns inspected per row.
    pub columns: u32,
    /// Generated placeholder stars in stable row-then-column order.
    pub stars: Vec<PreviewStar>,
}

impl PreviewSnapshot {
    /// Number of placeholder stars in this snapshot.
    #[must_use]
    pub fn star_count(&self) -> usize {
        self.stars.len()
    }
}

/// Generates one row of the synthetic preview.
///
/// Each row is independent of the others so a worker can report progress
/// between rows. Passing zero rows or columns returns an empty row.
#[must_use]
pub fn preview_row(seed: u64, row: u32, rows: u32, columns: u32) -> Vec<PreviewStar> {
    if rows == 0 || columns == 0 || row >= rows {
        return Vec::new();
    }

    let mut state = splitmix64(seed ^ u64::from(row).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    let mut stars = Vec::new();

    for column in 0..columns {
        state = splitmix64(state);

        // Two in five cells contain a placeholder star.
        if state % 5 > 1 {
            continue;
        }

        let x = ((u64::from(column) * 2 + 1) * u64::from(PREVIEW_COORDINATE_SCALE))
            / (u64::from(columns) * 2);
        let y = ((u64::from(row) * 2 + 1) * u64::from(PREVIEW_COORDINATE_SCALE))
            / (u64::from(rows) * 2);

        stars.push(PreviewStar {
            x: x as u16,
            y: y as u16,
            intensity: (state >> 32) as u8,
            size: ((state >> 40) % 4) as u8,
        });
    }

    stars
}

/// Builds a complete preview synchronously.
///
/// The desktop worker uses [`preview_row`] directly so it can report progress.
/// This helper is used by tests and headless callers.
#[must_use]
pub fn preview_snapshot(
    request_id: RequestId,
    seed: u64,
    rows: u32,
    columns: u32,
) -> PreviewSnapshot {
    let mut stars = Vec::new();
    for row in 0..rows {
        stars.extend(preview_row(seed, row, rows, columns));
    }

    PreviewSnapshot {
        request_id,
        seed,
        rows,
        columns,
        stars,
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut mixed = value;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_PREVIEW_COLUMNS, preview_row, preview_snapshot};
    use crate::worker::RequestId;

    #[test]
    fn a_seed_produces_a_stable_preview() {
        let first = preview_snapshot(RequestId::new(4), 99, 8, 12);
        let second = preview_snapshot(RequestId::new(4), 99, 8, 12);
        let different = preview_snapshot(RequestId::new(4), 100, 8, 12);

        assert_eq!(first, second);
        assert_ne!(first.stars, different.stars);
        assert!(first.stars.iter().all(|star| star.x < 10_000));
        assert!(first.stars.iter().all(|star| star.y < 10_000));
    }

    #[test]
    fn rows_are_bounded_and_independent() {
        assert!(preview_row(7, 0, 0, 10).is_empty());
        assert!(preview_row(7, 0, 8, 0).is_empty());
        assert!(preview_row(7, 8, 8, DEFAULT_PREVIEW_COLUMNS).is_empty());

        let row = preview_row(7, 3, 8, DEFAULT_PREVIEW_COLUMNS);
        let again = preview_row(7, 3, 8, DEFAULT_PREVIEW_COLUMNS);
        assert_eq!(row, again);
        assert!(row.len() <= DEFAULT_PREVIEW_COLUMNS as usize);
    }
}
