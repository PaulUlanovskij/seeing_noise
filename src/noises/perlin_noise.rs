use std::cell::LazyCell;

use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{HtmlElement, HtmlInputElement};

use super::noise::Noise;
use crate::{
    drawer::{IMAGE_BYTES_COUNT, draw_arrow},
    noises::helpers::{get_perlin_vec, lerp, perlin_grad, shuffle},
    *,
};

struct PerlinNoiseImpl {
    permutation: [usize; 256],
}

impl PerlinNoiseImpl {
    pub fn new(seed: u32) -> Self {
        let mut permutation: [usize; 256] = std::array::from_fn(|i| i);
        shuffle(&mut permutation, seed);

        PerlinNoiseImpl { permutation }
    }

    #[inline]
    fn fade(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    #[inline]
    fn hash(&self, x: i32, y: i32) -> usize {
        let xi = (x & 255) as usize;
        let yi = (y & 255) as usize;
        self.permutation[(self.permutation[xi] + yi) & 255]
    }

    #[inline]
    fn noise_blend_full(&self, x: f64, y: f64) -> f64 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;

        let xf = x - xi as f64;
        let yf = y - yi as f64;

        let u = Self::fade(xf);
        let v = Self::fade(yf);

        let aa = self.hash(xi, yi);
        let ab = self.hash(xi, yi + 1);
        let ba = self.hash(xi + 1, yi);
        let bb = self.hash(xi + 1, yi + 1);

        let x1 = lerp(u, perlin_grad(aa, xf, yf), perlin_grad(ba, xf - 1.0, yf));
        let x2 = lerp(
            u,
            perlin_grad(ab, xf, yf - 1.0),
            perlin_grad(bb, xf - 1.0, yf - 1.0),
        );

        lerp(v, x1, x2)
    }

    #[inline]
    fn noise_blend_dot_products(&self, x: f64, y: f64) -> f64 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;

        let xf = x - x.floor();
        let yf = y - y.floor();

        match (xf < 0.5, yf < 0.5) {
            (true, true) => {
                let aa = self.hash(xi, yi);
                let u = Self::fade(xf * 2.);
                let v = Self::fade(yf * 2.);
                perlin_grad(aa, u, v)
            }
            (true, false) => {
                let ab = self.hash(xi, yi + 1);
                let u = Self::fade(xf * 2.);
                let v = Self::fade((yf - 0.5) * 2.);
                perlin_grad(ab, u, v)
            }
            (false, true) => {
                let ba = self.hash(xi + 1, yi);
                let u = Self::fade((xf - 0.5) * 2.);
                let v = Self::fade(yf * 2.);
                perlin_grad(ba, u, v)
            }
            (false, false) => {
                let bb = self.hash(xi + 1, yi + 1);
                let u = Self::fade((xf - 0.5) * 2.);
                let v = Self::fade((yf - 0.5) * 2.);
                perlin_grad(bb, u, v)
            }
        }
    }

    fn generate_coloring(&self, settings: PerlinNoiseSettings) -> Vec<u8> {
        let mut v = Vec::with_capacity(IMAGE_BYTES_COUNT as usize);
        let scale = settings.scale.value();

        for y in 0..RESOLUTION {
            for x in 0..RESOLUTION {
                let nx = ((x as f64) - (HALF_RESOLUTION as f64)) / scale;
                let ny = ((y as f64) - (HALF_RESOLUTION as f64)) / scale;

                let noise_val = match settings.noise_type.clone() {
                    NoiseType::Standard => self.fbm_standard(nx, ny, &settings),
                    NoiseType::Turbulence => self.fbm_turbulence(nx, ny, &settings),
                    NoiseType::Ridge => self.fbm_ridge(nx, ny, &settings),
                    NoiseType::DomainWarp => self.fbm_domain_warp(nx, ny, &settings),
                };

                if noise_val < 0. {
                    let t = noise_val + 1.;
                    v.push(255);
                    v.push(lerp(t, 0.0, 255.0) as u8);
                    v.push(255);
                    v.push(255);
                } else {
                    let t = noise_val;
                    v.push(lerp(t, 255.0, 0.0) as u8);
                    v.push(255);
                    v.push(lerp(t, 255.0, 0.0) as u8);
                    v.push(255);
                }
            }
        }
        v
    }

    fn sample_noise(&self, x: f64, y: f64, use_dot_products: bool) -> f64 {
        if use_dot_products {
            self.noise_blend_dot_products(x, y)
        } else {
            self.noise_blend_full(x, y)
        }
    }

    pub fn fbm_standard(&self, x: f64, y: f64, settings: &PerlinNoiseSettings) -> f64 {

        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let use_dot_products = settings.show_dot_products.value();
        let gain = settings.gain.value();
        let h_exponent = settings.h_exponent.value();
        let lacunarity = settings.lacunarity.value();

        for i in 1..=octaves {
            let noise_val = self.sample_noise(x * frequency, y * frequency, use_dot_products);

            let include = match settings.visualization {
                Visualization::Final => true,
                Visualization::SingleOctave => i == show_octave,
                Visualization::AccumulatedOctaves => i <= show_octave,
            };
            if include {
                total += noise_val * amplitude;
                max_value += amplitude;
            }
            amplitude *= gain.powf(h_exponent);
            frequency *= lacunarity;
        }

        total / max_value
    }

    pub fn fbm_turbulence(&self, x: f64, y: f64, settings: &PerlinNoiseSettings) -> f64 {

        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let use_dot_products = settings.show_dot_products.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();

        for i in 1..=octaves {
            let noise_val = self
                .sample_noise(x * frequency, y * frequency, use_dot_products)
                .abs();

            let include = match settings.visualization {
                Visualization::Final => true,
                Visualization::SingleOctave => i == show_octave,
                Visualization::AccumulatedOctaves => i <= show_octave,
            };
            if include {
                total += noise_val * amplitude;
                max_value += amplitude;
            }
            amplitude *= gain;
            frequency *= lacunarity;
        }

        total / max_value
    }

    pub fn fbm_ridge(&self, x: f64, y: f64, settings: &PerlinNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;
        let mut weight = 1.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let use_dot_products = settings.show_dot_products.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();
        for i in 1..=octaves {
            let noise_val = self
                .sample_noise(x * frequency, y * frequency, use_dot_products)
                .abs();
            let noise_val = settings.ridge_offset.value() - noise_val;

            let include = match settings.visualization {
                Visualization::Final => true,
                Visualization::SingleOctave => i == show_octave,
                Visualization::AccumulatedOctaves => i <= show_octave,
            };
            if include {
                let noise_val = noise_val * noise_val * weight;
                total += noise_val * amplitude;
                max_value += amplitude;
            }

            weight = (noise_val * 2.0).clamp(0.0, 1.0);
            amplitude *= gain;
            frequency *= lacunarity;
        }

        total / max_value
    }

    pub fn fbm_domain_warp(&self, x: f64, y: f64, settings: &PerlinNoiseSettings) -> f64 {
        let warp_amount = settings.warp_amount.value();

        let adjusted_settings = PerlinNoiseSettings {
            h_exponent: HExponent(1.0),
            ..settings.clone()
        };
        let qx = self.fbm_standard(x, y, &adjusted_settings);
        let qy = self.fbm_standard(x + 5.2, y + 1.3, &adjusted_settings);

        let rx = x + warp_amount * qx;
        let ry = y + warp_amount * qy;

        self.fbm_standard(rx, ry, &adjusted_settings)
    }
}
impl PerlinNoise {
    fn on_setup() {}
    fn on_update() {
        let octaves = Octaves::parse().value();
        SHOW_OCTAVE.with(|e| e.set_max(format!("{octaves}").as_str()));

        if Visualization::parse() == Visualization::Final {
            set_hidden!(show_octave_control, true);
        } else {
            set_hidden!(show_octave_control, false);
        }

        match NoiseType::parse() {
            NoiseType::Standard => {
                set_hidden!(h_exponent_control, false);
                set_hidden!(ridge_offset_control, true);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::Turbulence => {
                set_hidden!(h_exponent_control, true);
                set_hidden!(ridge_offset_control, true);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::Ridge => {
                set_hidden!(h_exponent_control, true);
                set_hidden!(ridge_offset_control, false);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::DomainWarp => {
                set_hidden!(h_exponent_control, true);
                set_hidden!(ridge_offset_control, true);
                set_hidden!(warp_amount_control, false);
            }
        }
    }
    fn generate_and_draw(settings: PerlinNoiseSettings) {
        let perlin = PerlinNoiseImpl::new(settings.seed.value());

        let coloring = perlin.generate_coloring(settings.clone());

        draw_noise(coloring.as_slice());

        if settings.show_grid.value() {
            draw_grid(settings.scale.value(), "#000000");
        }

        if settings.show_vectors.value() {
            Self::draw_gradient_vectors(&settings, perlin);
        }
    }

    fn draw_gradient_vectors(settings: &PerlinNoiseSettings, noise: PerlinNoiseImpl) {
        let scale = settings.scale.value();

        for i in 0..settings.octaves.value() {
            let octave_scale = scale / 2_f64.powi(i as i32);
            let half_range = (HALF_RESOLUTION as f64 / octave_scale).floor() as isize;

            for x in -half_range..=half_range {
                for y in -half_range..=half_range {
                    let xf = HALF_RESOLUTION as f64 - x as f64 * octave_scale;
                    let yf = HALF_RESOLUTION as f64 - y as f64 * octave_scale;

                    let offset = octave_scale / 3.0;
                    let (mx, my) = get_perlin_vec(noise.hash(x as i32, y as i32));
                    let (tx, ty) = (xf + mx * offset, yf + my * offset);

                    draw_arrow(xf, yf, tx, ty, octave_scale / 5.0, "#ee0000");
                }
            }
        }
    }
}

define_noise!(perlin,
    sliders:[
        (seed, u32, 42.),
        (scale, f64, 50.),
        (octaves, u32, 1.),
        (lacunarity, f64, 2.0),
        (gain, f64, 0.5),
        (h_exponent, f64, 1.0),
        (ridge_offset, f64, 1.0),
        (warp_amount, f64, 4.0),
        (show_octave, u32, 1.)
    ];
    radios:[
        (visualization, final, single_octave, accumulated_octaves),
        (noise_type, standard, turbulence, ridge, domain_warp)
    ];
    checkboxes:[show_grid, show_vectors, show_dot_products];
);
