//! Brightness ripples: rings of light that travel across the rain.
//!
//! One mechanism covers two looks. Every ripple is an expanding circular
//! wavefront with a gaussian cross-section. Born on-screen it reads as a
//! point splash; born far off-screen, the arc that reaches the screen is so
//! shallow it reads as a wide planar wave rolling over everything.
//!
//! Terminal cells are roughly twice as tall as they are wide, so all
//! geometry runs in "visual" coordinates (x = col, y = 2 * row) to keep
//! circles looking circular.

use rand::rngs::SmallRng;
use rand::Rng;

struct Ripple {
    /// Origin in visual coordinates (may be far off-screen for waves).
    cx: f32,
    cy: f32,
    age: f32,
    ttl: f32,
    /// Front speed, visual units per second.
    speed: f32,
    /// Gaussian half-thickness of the front.
    width: f32,
    /// Peak brightness boost.
    amp: f32,
}

impl Ripple {
    fn intensity(&self, x: f32, y: f32) -> f32 {
        let r = self.speed * self.age;
        let dist = ((x - self.cx).powi(2) + (y - self.cy).powi(2)).sqrt();
        let d = dist - r;
        // Beyond a few widths the gaussian contributes nothing visible.
        if d.abs() > self.width * 3.0 {
            return 0.0;
        }
        // Quick attack, flat middle, fade over the last quarter of life —
        // a flat middle matters for waves, which spend most of their life
        // travelling in from off-screen before anyone can see them.
        let fade_in = (self.age * 4.0).min(1.0);
        let fade_out = ((self.ttl - self.age) / (self.ttl * 0.25)).clamp(0.0, 1.0);
        self.amp * fade_in * fade_out * (-d * d / (2.0 * self.width * self.width)).exp()
    }
}

pub struct Ripples {
    active: Vec<Ripple>,
    vis_w: f32,
    vis_h: f32,
    /// Ambient spawn rates, events per second. Zero disables.
    splash_rate: f32,
    wave_rate: f32,
}

impl Ripples {
    pub fn new(cols: u16, rows: u16) -> Self {
        Ripples {
            active: Vec::new(),
            vis_w: (cols as f32).max(1.0),
            vis_h: (rows as f32 * 2.0).max(1.0),
            splash_rate: 0.5,
            wave_rate: 0.25,
        }
    }

    /// Age, retire, and occasionally auto-spawn ripples.
    pub fn update(&mut self, dt: f32, rng: &mut SmallRng) {
        for r in self.active.iter_mut() {
            r.age += dt;
        }
        self.active.retain(|r| r.age < r.ttl);

        if rng.r#gen::<f32>() < self.splash_rate * dt {
            let x = rng.gen_range(0.0..self.vis_w);
            let y = rng.gen_range(0.0..self.vis_h);
            self.splash_at(x, y, rng);
        }
        if rng.r#gen::<f32>() < self.wave_rate * dt {
            self.wave(rng);
        }
    }

    /// Point ripple centered on a cell (e.g. a mouse click).
    pub fn splash(&mut self, col: u16, row: u16, rng: &mut SmallRng) {
        self.splash_at(col as f32, row as f32 * 2.0, rng);
    }

    fn splash_at(&mut self, x: f32, y: f32, rng: &mut SmallRng) {
        self.active.push(Ripple {
            cx: x,
            cy: y,
            age: 0.0,
            ttl: rng.gen_range(2.0..3.5),
            speed: rng.gen_range(18.0..36.0),
            width: rng.gen_range(3.0..6.0),
            amp: rng.gen_range(0.5..0.9),
        });
    }

    /// Wide planar wave: the same ring, born ~1.5 screen-diagonals away in a
    /// random direction, sized to die just after it finishes crossing.
    pub fn wave(&mut self, rng: &mut SmallRng) {
        let diag = (self.vis_w.powi(2) + self.vis_h.powi(2)).sqrt();
        let dist = diag * 1.5;
        let theta = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(33.0..59.0);
        self.active.push(Ripple {
            cx: self.vis_w / 2.0 + theta.cos() * dist,
            cy: self.vis_h / 2.0 + theta.sin() * dist,
            age: 0.0,
            ttl: (dist + diag) / speed,
            speed,
            width: rng.gen_range(10.0..18.0),
            amp: rng.gen_range(0.35..0.6),
        });
    }

    /// Total brightness boost at a cell, summed over active ripples.
    pub fn boost(&self, col: u16, row: u16) -> f32 {
        let (x, y) = (col as f32, row as f32 * 2.0);
        self.active.iter().map(|r| r.intensity(x, y)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn splash_brightens_near_its_front_then_dies() {
        let mut rng = SmallRng::seed_from_u64(7);
        let mut ripples = Ripples::new(80, 24);
        ripples.splash_rate = 0.0;
        ripples.wave_rate = 0.0;

        ripples.splash(40, 12, &mut rng);
        ripples.update(0.1, &mut rng);
        assert!(
            ripples.boost(40, 12) > 0.0,
            "young ripple should light its origin"
        );

        for _ in 0..400 {
            ripples.update(0.016, &mut rng); // ~6.4s, past any splash ttl
        }
        assert!(ripples.active.is_empty(), "expired ripples must retire");
    }

    #[test]
    fn wave_front_reaches_the_screen() {
        let mut rng = SmallRng::seed_from_u64(7);
        let mut ripples = Ripples::new(80, 24);
        ripples.splash_rate = 0.0;
        ripples.wave_rate = 0.0;

        ripples.wave(&mut rng);
        // Step until the wave has lit at least one on-screen cell.
        let mut lit = false;
        for _ in 0..1200 {
            ripples.update(0.016, &mut rng);
            lit = (0..24u16).any(|r| (0..80u16).any(|c| ripples.boost(c, r) > 0.05));
            if lit {
                break;
            }
        }
        assert!(lit, "an off-screen wave should sweep across the screen");
    }
}
