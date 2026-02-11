// Plain Sudoku generator v0.2
// (c) S. Salewski 2025
// 25-NOV-2025

use rand::{rng, seq::SliceRandom};
use rand::RngExt;

use std::fmt;

pub const MAX_DIFFICULTY_LEVEL: u8 = 7; // up to 7*7+8 zeros
pub const SUGGESTED_DIFFICULTY_LEVEL: u8 = 3;

const SIDE: usize = 9;
const CELL_COUNT: usize = SIDE * SIDE;
const BLOCK_SIDE: usize = 3;

type Row = [i8; SIDE];
type Col = [i8; SIDE];
type Block = [i8; SIDE];

fn shuffled_array_0_to_8() -> [i8; SIDE] {
    let mut arr = std::array::from_fn(|i| i as i8);
    arr.shuffle(&mut rng());
    arr
}

fn shuffled_squares() -> [usize; CELL_COUNT] {
    let mut arr = std::array::from_fn(|i| i);
    arr.shuffle(&mut rng());
    arr
}

/// Tuple struct:
/// - .0 = puzzle grid (0 = empty)
/// - .1 = fully solved grid
#[derive(Clone, Copy, Debug)]
pub struct Sudoku(pub [i8; CELL_COUNT], pub [i8; CELL_COUNT]);

impl fmt::Display for Sudoku {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in 0..SIDE {
            if r != 0 && r % BLOCK_SIDE == 0 {
                writeln!(f, "------+-------+------")?;
            }
            for c in 0..SIDE {
                if c != 0 && c % BLOCK_SIDE == 0 {
                    write!(f, "| ")?;
                }
                let v = self.0[r * SIDE + c];
                let ch = if v == 0 {
                    '.'
                } else {
                    char::from(b'0' + v as u8)
                };
                write!(f, "{ch} ")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Sudoku {
    // Check that this Sudoku is a complete, valid solution:
    // - no zeros
    // - each row/col/block contains 1..=9 exactly once
    #[allow(dead_code)]
    fn is_valid(&self) -> bool {
        // helper: check array is exactly {1..=9}
        fn is_1_to_9(a: &Row) -> bool {
            let mut v = *a; // copy
            v.sort(); // sort in-place
            v == [1, 2, 3, 4, 5, 6, 7, 8, 9]
        }

        if self.0.contains(&0) {
            return false;
        }

        // all rows
        for r in 0..SIDE {
            if !is_1_to_9(&self.row(r)) {
                return false;
            }
        }

        // all cols
        for c in 0..SIDE {
            if !is_1_to_9(&self.col(c)) {
                return false;
            }
        }

        // all 3x3 blocks
        for br in 0..BLOCK_SIDE {
            for bc in 0..BLOCK_SIDE {
                if !is_1_to_9(&self.block(br, bc)) {
                    return false;
                }
            }
        }
        true
    }

    fn row(&self, n: usize) -> Row {
        assert!(n < SIDE);
        let start = n * SIDE;
        let mut row = [0i8; SIDE];
        row.copy_from_slice(&self.0[start..start + SIDE]);
        row
    }

    fn col(&self, n: usize) -> Col {
        assert!(n < SIDE);
        let mut col = [0i8; SIDE];
        for (r, cell) in col.iter_mut().enumerate() {
            *cell = self.0[r * SIDE + n];
        }

        col
    }

    fn set_col(&mut self, n: usize, vals: &Col) {
        assert!(n < SIDE);
        for (r, v) in vals.iter().enumerate() {
            self.0[r * SIDE + n] = *v;
        }
    }

    fn block(&self, br: usize, bc: usize) -> Block {
        assert!(br < BLOCK_SIDE && bc < BLOCK_SIDE);
        let mut blk = [0i8; SIDE];
        let (r0, c0) = (br * BLOCK_SIDE, bc * BLOCK_SIDE);
        let mut k = 0;
        for r in 0..BLOCK_SIDE {
            for c in 0..BLOCK_SIDE {
                blk[k] = self.0[(r0 + r) * SIDE + (c0 + c)];
                k += 1;
            }
        }
        blk
    }

    fn set_block(&mut self, br: usize, bc: usize, vals: &Block) {
        assert!(br < BLOCK_SIDE && bc < BLOCK_SIDE);
        let (r0, c0) = (br * BLOCK_SIDE, bc * BLOCK_SIDE);
        let mut k = 0;
        for r in 0..BLOCK_SIDE {
            for c in 0..BLOCK_SIDE {
                self.0[(r0 + r) * SIDE + (c0 + c)] = vals[k];
                k += 1;
            }
        }
    }

    #[allow(dead_code)]
    fn print(&self) {
        for i in 0..CELL_COUNT {
            if i == 3 * SIDE || i == 6 * SIDE {
                println!("--- --- ---");
            }
            print!("{}", self.0[i]);
            let c = i % SIDE;
            if c == 2 || c == 5 {
                print!("|");
            }
            if c == SIDE - 1 {
                println!();
            }
        }
    }

    /// Check if `value` can be placed at `idx` without violating Sudoku rules.
    fn can_place(&self, idx: usize, value: i8) -> bool {
        let r = idx / SIDE;
        let c = idx % SIDE;

        // Row
        let row_start = r * SIDE;
        for offset in 0..SIDE {
            if self.0[row_start + offset] == value {
                return false;
            }
        }

        // Column
        for row in 0..SIDE {
            if self.0[row * SIDE + c] == value {
                return false;
            }
        }

        // Block
        let br = r / BLOCK_SIDE;
        let bc = c / BLOCK_SIDE;
        let block_origin = br * SIDE * BLOCK_SIDE + bc * BLOCK_SIDE;
        for br in 0..BLOCK_SIDE {
            for bc in 0..BLOCK_SIDE {
                if self.0[block_origin + bc + br * SIDE] == value {
                    return false;
                }
            }
        }

        true
    }

    fn solve_from(&mut self, idx: usize) -> bool {
        if idx == CELL_COUNT {
            return true;
        }
        if self.0[idx] != 0 {
            return self.solve_from(idx + 1);
        }

        let mut digits = [1i8, 2, 3, 4, 5, 6, 7, 8, 9];
        digits.shuffle(&mut rng());

        for &v in &digits {
            if self.can_place(idx, v) {
                self.0[idx] = v;
                if self.solve_from(idx + 1) {
                    return true;
                }
                self.0[idx] = 0;
            }
        }
        false
    }

    // Internal: count solutions from `idx`, up to `limit`.
    // Returns a number in 0..=limit.
    fn count_solutions_from(&mut self, idx: usize, limit: u32) -> u32 {
        if limit == 0 {
            return 0;
        }
        if idx == CELL_COUNT {
            return 1; // one complete solution
        }
        if self.0[idx] != 0 {
            return self.count_solutions_from(idx + 1, limit);
        }

        let mut count = 0;
        // For counting, randomness isn't required; 1..=9 is fine.
        for v in 1i8..=9 {
            if self.can_place(idx, v) {
                self.0[idx] = v;
                let found = self.count_solutions_from(idx + 1, limit - count);
                count += found;
                self.0[idx] = 0; // backtrack
                if count >= limit {
                    break; // early stop
                }
            }
        }
        count
    }

    // Public: count solutions of the *current puzzle*, but cap at `limit`.
    fn count_solutions(&self, limit: u32) -> u32 {
        let mut copy = *self; // work on a copy so the original isn't modified
        copy.count_solutions_from(0, limit)
    }

    // Does this puzzle have exactly one solution?
    fn has_unique_solution(&self) -> bool {
        self.count_solutions(2) == 1
    }

    /// Generate a fully solved Sudoku grid.
    fn new_solved() -> Self {
        let mut s = Self([0; CELL_COUNT], [0; CELL_COUNT]);
        s.solve_from(0);
        s
    }

    /// Generate a new Sudoku with the given difficulty level.
    ///
    /// The exact difficulty model is heuristic:
    /// - level 0: very easy, roughly one zero per row/column.
    /// - level > 0: progressively more zeros, while preserving uniqueness.
    pub fn new(level: u8) -> Self {
        let mut s = Self::new_solved();
        // Save fully solved version.
        s.1 = s.0;

        if level == 0 {
            // Ensure only one zero per row and column -- very easy start.
            let a = shuffled_array_0_to_8();
            for (row, &col_idx) in a.iter().enumerate() {
                s.0[row * SIDE + col_idx as usize] = 0;
            }
        } else {
            // Allow multiple (or zero) zeros per column.
            for row in 0..SIDE {
                let col = rng().random_range(0..SIDE);
                s.0[row * SIDE + col] = 0;
            }
        }

        // Ensure every column has at least one zero.
        for col in 0..SIDE {
            let mut c = s.col(col);
            if !c.contains(&0) {
                let r = rng().random_range(0..SIDE);
                c[r] = 0;
                s.set_col(col, &c);
            }
        }

        // Ensure every block has at least one zero.
        for br in 0..BLOCK_SIDE {
            for bc in 0..BLOCK_SIDE {
                let mut b = s.block(br, bc);
                if !b.contains(&0) {
                    let idx = rng().random_range(0..SIDE);
                    b[idx] = 0;
                    s.set_block(br, bc, &b);
                }
            }
        }

        // Now we have an easy start; remove a few more clues to increase difficulty.
        let mut more_zeros = level * 7;

        let positions = shuffled_squares();
        for pos in positions {
            if more_zeros == 0 {
                break;
            }
            let val = s.0[pos];
            if val != 0 {
                s.0[pos] = 0;
                if !s.has_unique_solution() {
                    // Revert if uniqueness is lost.
                    s.0[pos] = val;
                } else {
                    more_zeros -= 1;
                }
            }
        }
        s
    }
}

#[allow(dead_code)]
fn main_demo() {
    let s = Sudoku::new(1);
    s.print();
    println!("{:?}", s.row(0));
    println!("{:?}", s.col(0));
    println!("{:?}", s.block(0, 0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_sudoku_is_valid() {
        let s = Sudoku::new_solved();
        assert!(
            s.is_valid(),
            "Generated Sudoku is not a valid solution:\n{s}"
        );
    }
}
