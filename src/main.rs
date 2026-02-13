// Xilem Sudoku v0.2
// (c) S. Salewski 2025, 2026
// 13-FEB-2026

use std::time::{Duration, Instant};

use masonry::dpi::LogicalSize;
use masonry::parley::FontStack;
use masonry::layout::Length;
use masonry::layout::AsUnit;
use xilem::masonry::theme::{DEFAULT_GAP};
//use masonry::properties::types::{AsUnit, Length};

use tokio::time;
use winit::error::EventLoopError;

use xilem::core::fork;
use xilem::style::Style; // required for style extension methods
use xilem::view::{
    FlexExt, FlexSpacer, GridExt, button, flex_col, flex_row, grid, label, sized_box, slider, task,
    text_button,
};
use xilem::{Color, EventLoop, TextAlign, WidgetView, WindowOptions, Xilem};
use xilem_core::Edit;

mod sudoku;

const DEFAULT_DIFFICULTY: f64 = sudoku::SUGGESTED_DIFFICULTY_LEVEL as f64;

// Board geometry
const SIDE: usize = 9;
const CELL_COUNT: usize = SIDE * SIDE;
const BLOCK_SIDE: usize = 3;
const BOARD_BLOCKS: usize = SIDE / BLOCK_SIDE;

const GRID_GAP: Length = Length::const_px(3.0);
const GAP: Length = Length::const_px(4.0);

// Colors
const SOURCE_BG: Color = Color::from_rgb8(0x3a, 0x3a, 0x9a);
const CLUE_TEXT_COLOR: Color = Color::from_rgb8(0x7f, 0x7f, 0x7f);
const GUESS_TEXT_COLOR: Color = Color::from_rgb8(0xff, 0xff, 0xff);
const FAIL_TEXT_COLOR: Color = Color::from_rgb8(0xff, 0x00, 0x00);
const SUDOKU_BACKGROUND_COLOR: Color = Color::from_rgb8(0x33, 0x33, 0x33);
const SUDOKU_HIGHLIGHT_COLOR: Color = Color::from_rgb8(0x28, 0x28, 0x28);
const SELECTED_BACKGROUND_COLOR: Color = Color::from_rgb8(0x66, 0x66, 0x66);

const TIMER_TICK_MS: u64 = 50;

// --- Small helpers for board indexing ---------------------------------------------------------

#[inline]
fn row_of(index: usize) -> usize {
    index / SIDE
}

#[inline]
fn col_of(index: usize) -> usize {
    index % SIDE
}

#[inline]
fn row_start(index: usize) -> usize {
    row_of(index) * SIDE
}

#[inline]
fn block_origin(index: usize) -> usize {
    let block_row = row_of(index) / BLOCK_SIDE;
    let block_col = col_of(index) / BLOCK_SIDE;
    block_row * SIDE * BLOCK_SIDE + block_col * BLOCK_SIDE
}

// --- Application state ------------------------------------------------------------------------

/// Full application state.
struct AppState {
    /// Whether the periodic timer task is active (reserved for pause/resume).
    active: bool,
    /// Current puzzle grid (0 = empty).
    sudoku: [i8; CELL_COUNT],
    /// Fully solved grid used to check correctness.
    solved: [i8; CELL_COUNT],
    /// Marks which cells are original clues (not editable).
    is_clue: [bool; CELL_COUNT],
    /// Highlight mask (row/column/block of selected cell).
    highlight: [bool; CELL_COUNT],
    /// Currently selected cell index, if any.
    selected_cell: Option<usize>,
    /// Cell index of last failed guess, if any.
    fail: Option<usize>,
    /// Number of failed guesses.
    fails: i32,
    /// True if the selected cell currently conflicts with peers.
    collision: bool,
    /// Difficulty slider value.
    difficulty: f64,
    /// Number of remaining empty cells.
    voids: usize,
    /// Time when the current game started.
    start_time: Instant,
    /// Frozen elapsed time (in seconds) once solved, otherwise `None`.
    stopped_time: Option<u64>,
}

impl AppState {
    fn new(difficulty: f64) -> Self {
        // Properly destructure the tuple struct `Sudoku`
        let sudoku::Sudoku(puzzle, solution) = sudoku::Sudoku::new(difficulty as u8);

        let voids = puzzle.iter().filter(|&&n| n == 0).count();

        Self {
            active: true,
            sudoku: puzzle,
            solved: solution,
            is_clue: puzzle.map(|v| v != 0),
            highlight: [false; CELL_COUNT],
            selected_cell: None,
            fail: None,
            fails: 0,
            collision: false,
            difficulty,
            voids,
            start_time: Instant::now(),
            stopped_time: None,
        }
    }

    fn new_game(&mut self) {
        *self = Self::new(self.difficulty);
    }

    fn elapsed_seconds(&self) -> u64 {
        self.stopped_time
            .unwrap_or_else(|| self.start_time.elapsed().as_secs())
    }

    fn recompute_voids_and_maybe_stop_timer(&mut self) {
        self.voids = self.sudoku.iter().filter(|&&n| n == 0).count();
        if self.voids == 0 && self.stopped_time.is_none() {
            self.stopped_time = Some(self.start_time.elapsed().as_secs());
        }
    }

    fn clear_last_fail(&mut self) {
        if let Some(idx) = self.fail.take() {
            self.sudoku[idx] = 0;
        }
    }

    /// Check if the value in `index` conflicts with same values in its row/col/block.
    fn has_conflict(&self, index: usize) -> bool {
        let value = self.sudoku[index];
        if value == 0 {
            return false;
        }

        // Row
        let start = row_start(index);
        for offset in 0..SIDE {
            let i = start + offset;
            if i != index && self.sudoku[i] == value {
                return true;
            }
        }

        // Column
        let col = col_of(index);
        for row in 0..SIDE {
            let i = col + row * SIDE;
            if i != index && self.sudoku[i] == value {
                return true;
            }
        }

        // Block
        let origin = block_origin(index);
        for br in 0..BLOCK_SIDE {
            for bc in 0..BLOCK_SIDE {
                let i = origin + bc + br * SIDE;
                if i != index && self.sudoku[i] == value {
                    return true;
                }
            }
        }

        false
    }

    /// Apply a user guess to `index`.
    fn apply_guess(&mut self, index: usize, digit: i8) {
        if self.is_clue[index] {
            return;
        }

        self.sudoku[index] = digit;
        self.recompute_voids_and_maybe_stop_timer();

        self.fail = None;
        self.collision = false;

        // Only treat as a failure if it's not the correct solution and it conflicts.
        if self.sudoku[index] != self.solved[index] && self.has_conflict(index) {
            self.collision = true;
            self.fails += 1;
            self.fail = Some(index);
        }
    }

    fn clear_highlight(&mut self) {
        self.highlight = [false; CELL_COUNT];
    }

    fn highlight_row_col_block(&mut self, index: usize) {
        self.clear_highlight();

        // Row
        let start = row_start(index);
        for offset in 0..SIDE {
            self.highlight[start + offset] = true;
        }

        // Column
        let col = col_of(index);
        for row in 0..SIDE {
            self.highlight[col + row * SIDE] = true;
        }

        // Block
        let origin = block_origin(index);
        for br in 0..BLOCK_SIDE {
            for bc in 0..BLOCK_SIDE {
                self.highlight[origin + bc + br * SIDE] = true;
            }
        }
    }

    fn select_cell(&mut self, index: usize) {
        self.clear_last_fail();

        if !self.is_clue[index] {
            self.selected_cell = Some(index);
        }

        self.highlight_row_col_block(index);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(DEFAULT_DIFFICULTY)
    }
}

// --- Views ------------------------------------------------------------------------------------

fn number_grid() -> impl WidgetView<Edit<AppState>> + use<> {
    // Digit buttons 1–9 (explicit loop instead of iterator `.map()` to avoid ICE)
    let mut number_cells = Vec::new();
    for i in 0..9 {
        let digit = i + 1;
        let btn = text_button(format!("{digit}"), move |state: &mut AppState| {
            if let Some(index) = state.selected_cell {
                state.apply_guess(index, digit as i8);
            }
        })
        .padding(0.0)
        .background_color(SOURCE_BG)
        .corner_radius(0.0)
        .border_color(Color::TRANSPARENT)
        .grid_pos(i, 0);
        number_cells.push(btn);
    }

    grid(number_cells, 9, 1).gap(GRID_GAP)
}

fn cell(state: &mut AppState, index: usize) -> impl WidgetView<Edit<AppState>> + use<> {
    let value = state.sudoku[index];

    let text = match value {
        0 => String::new(),
        n => n.to_string(),
    };

    let color = if state.is_clue[index] {
        CLUE_TEXT_COLOR
    } else if value != 0 && state.selected_cell == Some(index) && state.collision {
        FAIL_TEXT_COLOR
    } else {
        GUESS_TEXT_COLOR
    };

    let background = if state.selected_cell == Some(index) {
        SELECTED_BACKGROUND_COLOR
    } else if state.highlight[index] {
        SUDOKU_HIGHLIGHT_COLOR
    } else {
        SUDOKU_BACKGROUND_COLOR
    };

    let cell_label = label(text)
        .text_alignment(TextAlign::Center)
        .text_size(24.0)
        .color(color);

    button(cell_label, move |state: &mut AppState| {
        state.select_cell(index);
    })
    .padding(0.0)
    .background_color(background)
    .corner_radius(0.0)
    .border_color(Color::TRANSPARENT)
}

fn info_bar(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
    let elapsed = state.elapsed_seconds();
    let minutes = elapsed / 60;
    let seconds = elapsed % 60;

    flex_row((
    FlexSpacer::Fixed(DEFAULT_GAP),
        label(format!("Time: {minutes}:{seconds:02}")).font(FontStack::Source("monospace".into())),
        FlexSpacer::Flex(1.0),
        label(format!("Voids left: {}", state.voids)),
                FlexSpacer::Flex(1.0),
        label(format!("Fails: {}", state.fails)),
                FlexSpacer::Flex(1.0),
        label(format!("Difficulty: {:.0}", state.difficulty)),
                //FlexSpacer::Flex(1.0),
        //sized_box(
            slider(
                0.0,
                sudoku::MAX_DIFFICULTY_LEVEL as f64,
                state.difficulty,
                |state: &mut AppState, val| {
                    state.difficulty = val;
                },
            )
            .step(1.0)
            .width(80.px()),
        //)
        //.width(40_i32.px()),
                FlexSpacer::Flex(1.0),
        text_button("New Game", |state: &mut AppState| state.new_game()).padding(8.0),
        FlexSpacer::Fixed(DEFAULT_GAP),
    ))
}

/// Build the full Sudoku board (3×3 blocks of 3×3 cells).
fn build_board(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
    let mut sudoku_blocks = Vec::with_capacity(BOARD_BLOCKS * BOARD_BLOCKS);

    for block_row in 0..BOARD_BLOCKS {
        for block_col in 0..BOARD_BLOCKS {
            let mut block_cells = Vec::with_capacity(BLOCK_SIDE * BLOCK_SIDE);

            for cell_row in 0..BLOCK_SIDE {
                for cell_col in 0..BLOCK_SIDE {
                    let index = block_row * SIDE * BLOCK_SIDE
                        + cell_row * SIDE
                        + block_col * BLOCK_SIDE
                        + cell_col;

                    block_cells.push(cell(state, index).grid_pos(cell_col as i32, cell_row as i32));
                }
            }

            let block_grid = grid(block_cells, BLOCK_SIDE as i32, BLOCK_SIDE as i32);
            sudoku_blocks.push(sized_box(block_grid).grid_pos(block_col as i32, block_row as i32));
        }
    }

    grid(sudoku_blocks, BOARD_BLOCKS as i32, BOARD_BLOCKS as i32).gap(GRID_GAP)
}

fn app_logic(state: &mut AppState) -> impl WidgetView<Edit<AppState>> + use<> {
    let board = build_board(state);

    let layout = flex_col((
        FlexSpacer::Fixed(GAP),
        info_bar(state),
        number_grid().flex(1.0),
        board.flex(9.0),
    ))
    .gap(GAP);

    // Background task: tick regularly to update the timer label.
    fork(
        layout,
        state.active.then(|| {
            task(
                |proxy, _| async move {
                    let mut interval = time::interval(Duration::from_millis(TIMER_TICK_MS));
                    loop {
                        interval.tick().await;
                        if proxy.message(()).is_err() {
                            break;
                        }
                    }
                },
                |_state: &mut AppState, ()| {
                    // No state mutation needed; re-running the view updates the timer display.
                },
            )
        }),
    )
}

fn main() -> Result<(), EventLoopError> {
    let window_options = WindowOptions::new("Sudoku")
        .with_min_inner_size(LogicalSize::new(600.0, 600.0))
        .with_initial_inner_size(LogicalSize::new(700.0, 700.0));

    let app = Xilem::new_simple(AppState::default(), app_logic, window_options);

    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
