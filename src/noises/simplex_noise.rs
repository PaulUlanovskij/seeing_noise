use std::cell::LazyCell;

use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{HtmlElement, HtmlInputElement};

use super::noise::Noise;
use crate::{
    drawer::{IMAGE_BYTES_COUNT, draw_arrow},
    noises::helpers::{lerp, perlin_grad, shuffle},
    *,
};

struct SimplexNoiseImpl {
    permutation: [usize; 256],
}

impl SimplexNoiseImpl {
    const F2: f64 = 0.3660254037844386; // (sqrt(3) - 1) / 2 Because .sqrt() is not const. Why?!
    const G2: f64 = 0.21132486540518708; // (1 - 1/sqrt(3)) / 2

    pub fn new(seed: u32) -> Self {
        let mut permutation: [usize; 256] = std::array::from_fn(|i| i);
        shuffle(&mut permutation, seed);

        SimplexNoiseImpl { permutation }
    }

    #[inline]
    fn get_perm(&self, i: usize) -> usize {
        self.permutation[i & 255]
    }

    fn noise(&self, x: f64, y: f64) -> f64 {
        let s = (x + y) * Self::F2;
        let i = (x + s).floor();
        let j = (y + s).floor();

        let t = (i + j) * Self::G2;
        let x0_origin = i - t;
        let y0_origin = j - t;

        let x0 = x - x0_origin;
        let y0 = y - y0_origin;

        let (i1, j1) = if x0 > y0 {
            (1, 0) // Lower triangle, XY order: (0,0)->(1,0)->(1,1)
        } else {
            (0, 1) // Upper triangle, YX order: (0,0)->(0,1)->(1,1)
        };

        let x1 = x0 - i1 as f64 + Self::G2;
        let y1 = y0 - j1 as f64 + Self::G2;

        let x2 = x0 - 1.0 + 2.0 * Self::G2;
        let y2 = y0 - 1.0 + 2.0 * Self::G2;

        let ii = i as i32 as usize;
        let jj = j as i32 as usize;

        let gi0 = self.get_perm(ii + self.get_perm(jj));
        let gi1 = self.get_perm(ii + i1 + self.get_perm(jj + j1));
        let gi2 = self.get_perm(ii + 1 + self.get_perm(jj + 1));

        let mut n0 = 0.0;
        let mut n1 = 0.0;
        let mut n2 = 0.0;

        let t0 = 0.5 - x0 * x0 - y0 * y0;
        if t0 >= 0.0 {
            let t0_sq = t0 * t0;
            n0 = t0_sq * t0_sq * perlin_grad(gi0, x0, y0);
        }

        let t1 = 0.5 - x1 * x1 - y1 * y1;
        if t1 >= 0.0 {
            let t1_sq = t1 * t1;
            n1 = t1_sq * t1_sq * perlin_grad(gi1, x1, y1);
        }

        let t2 = 0.5 - x2 * x2 - y2 * y2;
        if t2 >= 0.0 {
            let t2_sq = t2 * t2;
            n2 = t2_sq * t2_sq * perlin_grad(gi2, x2, y2);
        }

        70.0 * (n0 + n1 + n2)
    }

    fn generate_coloring(
        &self,
        settings: &SimplexNoiseSettings,
    ) -> Vec<u8> {
        let scale = settings.scale.value();
        let octaves = settings.octaves.value();
        let persistence = settings.persistence.value();
        let coloring = &settings.coloring;

        let mut v = Vec::with_capacity(IMAGE_BYTES_COUNT as usize);

        for y in 0..RESOLUTION {
            for x in 0..RESOLUTION {
                let nx = (x as f64 - HALF_RESOLUTION as f64) / scale;
                let ny = (y as f64 - HALF_RESOLUTION as f64) / scale;

                let noise_val = self.octave_noise(nx, ny, octaves, persistence, coloring);

                let (r, g, b) = if noise_val < 0.0 {
                    let t = noise_val + 1.0;
                    (255, lerp(t, 0.0, 255.0) as u8, 255)
                } else {
                    let t = (noise_val + 1.0) * 0.5 - 0.5;
                    let t = t * 2.0;
                    let val = lerp(t, 255.0, 0.0) as u8;
                    (val, 255, val)
                };

                v.extend_from_slice(&[r, g, b, 255]);
            }
        }
        v
    }

    fn octave_noise(
        &self,
        x: f64,
        y: f64,
        octaves: u32,
        persistence: f64,
        coloring: &Coloring,
    ) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        for _ in 0..octaves {
            let noise = match coloring {
                Coloring::Full | Coloring::DotProducts => self.noise(x * frequency, y * frequency),
                Coloring::None => 0.0,
            };

            total += noise * amplitude;
            max_value += amplitude;
            amplitude *= persistence;
            frequency *= 2.0;
        }

        total / max_value
    }

    fn get_simplex_corners(&self, x: f64, y: f64) -> SimplexCorners {
        let s = (x + y) * Self::F2;
        let i = (x + s).floor();
        let j = (y + s).floor();

        let t = (i + j) * Self::G2;
        let x0_origin = i - t;
        let y0_origin = j - t;

        let x0 = x - x0_origin;
        let y0 = y - y0_origin;

        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };

        let ii = i as i32 as usize;
        let jj = j as i32 as usize;

        let gi0 = self.get_perm(ii + self.get_perm(jj));
        let gi1 = self.get_perm(ii + i1 + self.get_perm(jj + j1));
        let gi2 = self.get_perm(ii + 1 + self.get_perm(jj + 1));

        SimplexCorners {
            i: i as isize,
            j: j as isize,
            i1,
            j1,
            gi0,
            gi1,
            gi2,
        }
    }
}
struct SimplexCorners {
    i: isize,
    j: isize,
    i1: usize,
    j1: usize,
    gi0: usize,
    gi1: usize,
    gi2: usize,
}

impl SimplexNoise {
    fn on_setup(){}
    fn on_update(){}
    fn generate_and_draw(settings: SimplexNoiseSettings) {
        let simplex = SimplexNoiseImpl::new(settings.seed.value());

        let coloring = simplex.generate_coloring(&settings);

        draw_noise(&coloring);

        if settings.show_grid.value() {
            draw_grid(settings.scale.value(), "#000000");
        }

        if settings.show_vectors.value() {
            Self::draw_gradient_vectors(&simplex, &settings);
        }
    }

    fn draw_gradient_vectors(
        simplex: &SimplexNoiseImpl,
        settings: &SimplexNoiseSettings,
    ) {
        let scale = settings.scale.value();

        for octave in 0..settings.octaves.value() {
            let octave_scale = scale / 2_f64.powi(octave as i32);
            let half_range = (HALF_RESOLUTION as f64 / octave_scale).floor() as isize;

            for gx in -half_range..=half_range {
                for gy in -half_range..=half_range {
                    let world_x = gx as f64 * octave_scale;
                    let world_y = gy as f64 * octave_scale;

                    let nx = world_x / scale;
                    let ny = world_y / scale;

                    let corners = simplex.get_simplex_corners(nx, ny);

                    let offset = octave_scale / 3.0;

                    let screen_x = HALF_RESOLUTION as f64 + world_x;
                    let screen_y = HALF_RESOLUTION as f64 + world_y;
                    Self::draw_gradient_arrow(screen_x, screen_y, corners.gi0, offset);

                    let screen_x1 = screen_x + corners.i1 as f64 * octave_scale;
                    let screen_y1 = screen_y + corners.j1 as f64 * octave_scale;
                    Self::draw_gradient_arrow(screen_x1, screen_y1, corners.gi1, offset);

                    let screen_x2 = screen_x + octave_scale;
                    let screen_y2 = screen_y + octave_scale;
                    Self::draw_gradient_arrow(screen_x2, screen_y2, corners.gi2, offset);
                }
            }
        }
    }

    fn draw_gradient_arrow(xf: f64, yf: f64, gi: usize, offset: f64) {
        let (tx, ty) = match gi & 7 {
            0 => (xf - offset, yf - offset),
            1 => (xf - offset, yf + offset),
            2 => (xf + offset, yf - offset),
            3 => (xf + offset, yf + offset), 
            4 => (xf - offset, yf),
            5 => (xf, yf + offset),
            6 => (xf, yf - offset),
            _ => (xf + offset, yf),
        };

        draw_arrow(xf, yf, tx, ty, offset / 2.0, "#ee0000");
    }
}

define_noise!(simplex,
    sliders:[ (seed, u32, 42.), (scale, f64, 50.), (octaves, u32, 1.), (persistence, f64, 0.5)];
    radios:[(coloring, full, dot_products, none)];
    checkboxes:[show_grid, show_vectors];
);
