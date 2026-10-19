//! Court positions as `.dvw` writes them (`[3SCOUT]` columns 4–6): an
//! index into a 100 × 101 grid over the court and a margin around it,
//! `x + (y − 1) · 100`. Positions are in court units as openvolley gives
//! them: the court is x 0.5–3.5 (9 m) by y 0.5–6.5 (18 m), net at y 3.5.
//!
//! The frame is the play's own (checked on the corpus, FORMAT-NOTES): the
//! team making the play has its half at the bottom (y < 3.5) and faces up,
//! so its zone 4 is on the left (small x). A reception carries the serve's
//! position and a dig the attack's, in the server's and attacker's frame.

/// Columns of the grid (x).
const COLUMNS: u16 = 100;
/// Rows of the grid (y).
const ROWS: u16 = 101;

/// The centre of grid cell `index`, or `None` outside the grid.
pub fn position(index: u16) -> Option<(f64, f64)> {
    if !(1..=COLUMNS * ROWS).contains(&index) {
        return None;
    }
    let xi = f64::from((index - 1) % COLUMNS + 1);
    let yi = f64::from((index - 1) / COLUMNS + 1);
    Some((
        0.5 + (xi - 11.0) / 80.0 * 3.0 + 3.0 / 160.0,
        0.5 + (yi - 11.0) / 81.0 * 6.0 + 3.0 / 81.0,
    ))
}

/// The grid cell a position falls in; positions past the margin go to
/// the edge cell.
pub fn index(x: f64, y: f64) -> u16 {
    let bin = |v: f64, cells: f64, per: f64, top: u16| -> u16 {
        let i = ((v - 0.5) * cells / per).floor() + 11.0;
        // Clamped to the grid, so the cast can't truncate.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let i = i.clamp(1.0, f64::from(top)) as u16;
        i
    };
    let xi = bin(x, 80.0, 3.0, COLUMNS);
    let yi = bin(y, 81.0, 6.0, ROWS);
    xi + (yi - 1) * COLUMNS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values from openvolley's `dv_index2xy`.
    #[test]
    fn grid_cells_match_openvolley() {
        let near =
            |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6;
        assert!(near(position(1).expect("1"), (0.14375, -0.203_703_7)));
        assert!(near(position(100).expect("100"), (3.85625, -0.203_703_7)));
        assert!(near(position(101).expect("101"), (0.14375, -0.129_629_6)));
        assert!(near(position(4868).expect("4868"), (2.65625, 3.351_851_9)));
        assert!(near(
            position(10100).expect("10100"),
            (3.85625, 7.203_703_7)
        ));
        assert_eq!(position(0), None);
        assert_eq!(position(10101), None);
    }

    #[test]
    fn every_cell_centre_maps_back_to_its_cell() {
        for i in 1..=10100 {
            let (x, y) = position(i).expect("cell");
            assert_eq!(index(x, y), i);
        }
        assert_eq!(index(-5.0, -5.0), 1);
        assert_eq!(index(9.0, 9.0), 10100);
    }
}
