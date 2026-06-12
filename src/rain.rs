//! The rain simulation and a double-buffered diff renderer.
//!
//! Each column hosts independent falling "streams". A stream owns its own
//! column of glyphs, a fractional head position (so motion is smooth and
//! sub-cell), a speed, and a length. Trails fade via the theme gradient, and
//! glyphs mutate in place so the rain shimmers instead of scrolling rigidly.

use crate::glyphs;
use crate::ripple::Ripples;
use crate::theme::Theme;
use crossterm::style::Color;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// One falling stream within a column.
struct Stream {
    /// Row of the leading glyph, fractional for smooth motion. Grows downward.
    head: f32,
    /// Rows per second.
    speed: f32,
    /// Number of glyphs from head to faded tail.
    len: usize,
    /// Glyphs, index 0 == head, increasing index == further up the trail.
    glyphs: Vec<char>,
}

/// A single rendered cell: glyph + foreground color + weight.
#[derive(Clone, Copy, PartialEq)]
struct Cell {
    ch: char,
    color: Color,
    bold: bool,
}

impl Cell {
    const BLANK: Cell = Cell {
        ch: ' ',
        color: Color::Reset,
        bold: false,
    };
    /// The cell to the right of a double-width glyph: painted by that glyph,
    /// so the renderer must never write to it itself.
    const SHADOW: Cell = Cell {
        ch: '\0',
        color: Color::Reset,
        bold: false,
    };
}

pub struct Rain {
    cols: u16,
    rows: u16,
    theme: Theme,
    /// Per-column list of active streams.
    streams: Vec<Vec<Stream>>,
    /// Brightness composite buffer (reused each frame to avoid allocation).
    bright: Vec<f32>,
    /// Back buffer being drawn into this frame.
    back: Vec<Cell>,
    /// Front buffer = what is currently on screen.
    front: Vec<Cell>,
    rng: SmallRng,
    /// Spawn density: higher = more streams per column.
    density: f32,
    /// How fast glyphs mutate, scaled per second.
    mutation: f32,
    pool: Vec<char>,
    /// `pool[..narrow]` are single-column glyphs; the rest are double-width
    /// kanji, which must not spawn where there's no second column.
    narrow: usize,
    /// Travelling brightness waves layered over the rain.
    ripples: Ripples,
}

impl Rain {
    pub fn new(cols: u16, rows: u16, theme: Theme, density: f32, mutation: f32) -> Self {
        let n = cols as usize * rows as usize;
        let (pool, narrow) = glyphs::alphabet();
        let mut rain = Rain {
            cols,
            rows,
            theme,
            streams: (0..cols).map(|_| Vec::new()).collect(),
            bright: vec![0.0; n],
            back: vec![Cell::BLANK; n],
            front: vec![Cell::BLANK; n],
            rng: SmallRng::from_entropy(),
            density,
            mutation,
            pool,
            narrow,
            ripples: Ripples::new(cols, rows),
        };
        // Seed the screen so it isn't empty on the first frame.
        for _ in 0..(cols as usize / 2) {
            let c = rain.rng.gen_range(0..cols);
            let head = rain.rng.gen_range(0.0..rows as f32);
            rain.spawn(c, head);
        }
        rain
    }

    /// Resize the grid, preserving the theme/params and reseeding buffers.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        *self = Rain::new(cols, rows, self.theme, self.density, self.mutation);
    }

    /// The glyph pool a column may draw from: the last column can't host
    /// double-width kanji (no second column to paint into).
    fn col_pool(pool: &[char], narrow: usize, col: u16, cols: u16) -> &[char] {
        if col + 1 < cols {
            pool
        } else {
            &pool[..narrow]
        }
    }

    fn spawn(&mut self, col: u16, head: f32) {
        let max_len = ((self.rows as f32) * 0.7) as usize;
        let len = self.rng.gen_range(6..=max_len.max(8));
        let speed = self.rng.gen_range(6.0..26.0);
        let pool = Self::col_pool(&self.pool, self.narrow, col, self.cols);
        let glyphs = (0..len)
            .map(|_| glyphs::random_glyph(&mut self.rng, pool))
            .collect();
        self.streams[col as usize].push(Stream {
            head,
            speed,
            len,
            glyphs,
        });
    }

    /// Advance the simulation by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        let rows = self.rows as f32;

        self.ripples.update(dt, &mut self.rng);

        // Borrow the fields the hot loop touches disjointly so the closure
        // below doesn't conflict with the `self.streams` borrow.
        let Rain {
            streams,
            rng,
            pool,
            narrow,
            mutation,
            density,
            cols,
            ..
        } = self;
        let mutation = *mutation;
        let density = *density;

        for col in 0..*cols {
            let pool = Self::col_pool(pool, *narrow, col, *cols);
            let streams = &mut streams[col as usize];

            // Advance, mutate, and retire streams.
            streams.retain_mut(|s| {
                let old_head = s.head.floor() as i32;
                s.head += s.speed * dt;
                let new_head = s.head.floor() as i32;

                // Glyphs must stay anchored to screen rows. The trail is
                // indexed relative to the head, so each time the head crosses
                // a row boundary we shift a fresh glyph in at the head and
                // drop the tail's — otherwise every row's character would
                // visibly jump up one cell.
                let crossed = (new_head - old_head).clamp(0, s.len as i32);
                for _ in 0..crossed {
                    s.glyphs.pop();
                    s.glyphs.insert(0, glyphs::random_glyph(rng, pool));
                }

                // Occasionally swap glyphs so the trail shimmers. Probability
                // scales with dt so it's frame-rate independent.
                let p = (mutation * dt).min(1.0);
                for g in s.glyphs.iter_mut() {
                    if rng.r#gen::<f32>() < p {
                        *g = pool[rng.gen_range(0..pool.len())];
                    }
                }

                // Keep until the whole trail has scrolled off the bottom.
                s.head - s.len as f32 <= rows
            });

            // Spawn new streams once the topmost has descended enough to leave a
            // gap, gated by density so columns don't saturate.
            let spawn_ok = streams
                .iter()
                .all(|s| s.head >= s.len as f32 + rng.gen_range(0.0..6.0));
            if spawn_ok && rng.r#gen::<f32>() < density * dt {
                let len = rng.gen_range(6..=((rows * 0.7) as usize).max(8));
                let speed = rng.gen_range(6.0..26.0);
                let glyphs = (0..len).map(|_| glyphs::random_glyph(rng, pool)).collect();
                streams.push(Stream {
                    head: 0.0,
                    speed,
                    len,
                    glyphs,
                });
            }
        }

        self.composite();
    }

    /// Composite all streams into the brightness + back buffers.
    fn composite(&mut self) {
        for b in self.bright.iter_mut() {
            *b = -1.0; // sentinel: "no glyph here"
        }
        for c in self.back.iter_mut() {
            *c = Cell::BLANK;
        }

        let cols = self.cols as usize;
        let rows = self.rows as i32;

        for col in 0..self.cols {
            for s in &self.streams[col as usize] {
                let head_i = s.head.floor() as i32;
                for d in 0..s.len {
                    let r = head_i - d as i32;
                    if r < 0 || r >= rows {
                        continue;
                    }
                    // Brightness: 1.0 at head, fading linearly up the trail.
                    let brightness = 1.0 - (d as f32 / s.len as f32);
                    let idx = r as usize * cols + col as usize;

                    // When streams overlap, the brighter one wins.
                    if brightness <= self.bright[idx] {
                        continue;
                    }
                    self.bright[idx] = brightness;

                    let is_head = d == 0;
                    // Ripples boost how lit the cell is, but don't take part
                    // in the overlap contest above — that stays on the
                    // trail's own brightness. Past 1.0 the theme bleeds the
                    // color toward white, so overdriven cells glow hot.
                    let lit = brightness + self.ripples.boost(col, r as u16);
                    let color = self.theme.color(lit, is_head, col, self.cols);
                    self.back[idx] = Cell {
                        ch: s.glyphs[d],
                        color,
                        bold: is_head,
                    };
                }
            }
        }

        // Double-width glyphs paint into the next column, so shadow the cell
        // to their right: the renderer skips shadows, and any glyph another
        // stream composited there would otherwise fight with the kanji.
        for row in 0..self.rows as usize {
            for col in 0..cols.saturating_sub(1) {
                let idx = row * cols + col;
                if glyphs::is_wide(self.back[idx].ch) {
                    self.back[idx + 1] = Cell::SHADOW;
                }
            }
        }
    }

    /// Drop a point ripple centered on a cell (e.g. a mouse click).
    pub fn splash(&mut self, col: u16, row: u16) {
        self.ripples.splash(col, row, &mut self.rng);
    }

    /// Drop a point ripple somewhere random on screen.
    pub fn splash_random(&mut self) {
        let col = self.rng.gen_range(0..self.cols.max(1));
        let row = self.rng.gen_range(0..self.rows.max(1));
        self.splash(col, row);
    }

    /// Launch a wide wave that sweeps in from off-screen.
    pub fn wave(&mut self) {
        self.ripples.wave(&mut self.rng);
    }

    /// Emit only the cells that changed since the last frame.
    ///
    /// Returns a list of `(col, row, glyph, color, bold)` draw ops; the
    /// caller turns these into terminal writes. Front buffer is updated to
    /// match.
    pub fn diff(&mut self) -> Vec<(u16, u16, char, Color, bool)> {
        let cols = self.cols as usize;
        let mut ops = Vec::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let idx = row as usize * cols + col as usize;
                let nb = self.back[idx];
                if nb != self.front[idx] {
                    self.front[idx] = nb;
                    // Shadow cells are painted by the wide glyph to their
                    // left (whose op precedes this cell in row-major order),
                    // so track them in the front buffer but emit nothing.
                    if nb != Cell::SHADOW {
                        ops.push((col, row, nb.ch, nb.color, nb.bold));
                    }
                }
            }
        }
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_run_and_draw_without_panic() {
        let mut rain = Rain::new(80, 24, Theme::Green, 1.5, 8.0);
        let mut total_ops = 0;
        // Many frames at a fixed dt should advance, retire, and respawn streams
        // across the whole grid without ever indexing out of bounds.
        for _ in 0..600 {
            rain.update(1.0 / 60.0);
            total_ops += rain.diff().len();
        }
        assert!(total_ops > 0, "expected the rain to draw something");
    }

    #[test]
    fn trail_glyphs_stay_anchored_to_screen_rows() {
        // With mutation disabled, a character drawn at a given cell must stay
        // the same as the stream descends past it — the head reveals new
        // glyphs below, it never shifts the existing trail. One column and
        // zero density guarantee a single stream, so overlap (where the
        // brighter stream legitimately wins a cell) can't confuse the check.
        let mut rain = Rain::new(1, 40, Theme::Green, 0.0, 0.0);
        rain.streams[0].clear();
        rain.spawn(0, 5.0);
        rain.update(1.0 / 60.0);
        let before = rain.back.clone();
        for _ in 0..30 {
            rain.update(1.0 / 60.0);
        }
        for idx in 0..before.len() {
            let (b, a) = (before[idx], rain.back[idx]);
            if b.ch != ' ' && a.ch != ' ' {
                assert_eq!(
                    b.ch, a.ch,
                    "glyph at cell {idx} changed while still inside a trail"
                );
            }
        }
    }

    #[test]
    fn resize_to_tiny_grid_is_safe() {
        let mut rain = Rain::new(80, 24, Theme::Cyan, 1.0, 5.0);
        rain.update(0.016);
        rain.resize(1, 1);
        rain.update(0.5); // large dt jump shouldn't panic
        let _ = rain.diff();
    }
}
