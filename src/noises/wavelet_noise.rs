use std::cell::LazyCell;

use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{HtmlElement, HtmlInputElement};

use super::noise::Noise;
use crate::{
    drawer::IMAGE_BYTES_COUNT,
    noises::helpers::lerp,
    *,
};

const WAVELET_TILE_SIZE: usize = 128;

struct WaveletNoiseImpl {
    noise_tile: Vec<f64>,
}

impl WaveletNoiseImpl {
    pub fn new(seed: u32) -> Self {
        let mut noise_tile = vec![0.0; WAVELET_TILE_SIZE * WAVELET_TILE_SIZE];
        Self::generate_noise_tile(&mut noise_tile, seed);

        WaveletNoiseImpl { noise_tile }
    }

    fn generate_noise_tile(noise_tile: &mut [f64], seed: u32) {
        for (i, p) in noise_tile.iter_mut().enumerate() {
            *p = squirrel_noise5::f32_neg_one_to_one_1d(i as i32, seed as i32) as f64;
        }

        let sum: f64 = noise_tile.iter().sum();
        let mean = sum / noise_tile.len() as f64;
        for val in noise_tile.iter_mut() {
            *val -= mean;
        }

        Self::wavelet_decompose_2d(noise_tile);
    }

    fn wavelet_decompose_2d(data: &mut [f64]) {
        let sz = WAVELET_TILE_SIZE;
        let mut temp = vec![0.0; sz];

        for y in 0..sz {
            for x in 0..sz {
                temp[x] = data[y * sz + x];
            }
            Self::haar_1d(temp.as_mut_slice(), sz);
            for x in 0..sz {
                data[y * sz + x] = temp[x];
            }
        }

        for x in 0..sz {
            for y in 0..sz {
                temp[y] = data[y * sz + x];
            }
            Self::haar_1d(temp.as_mut_slice(), sz);
            for y in 0..sz {
                data[y * sz + x] = temp[y];
            }
        }
    }

    fn haar_1d(data: &mut [f64], n: usize) {
        let mut temp = vec![0.0; n];
        let half = n / 2;

        for i in 0..half {
            let sum = data[2 * i] + data[2 * i + 1];
            let diff = data[2 * i] - data[2 * i + 1];
            temp[i] = sum * 0.5; // Low frequencies
            temp[i + half] = diff * 0.5; // High frequencies
        }

        data[..n].copy_from_slice(&temp[..n]);
    }

    #[inline]
    fn mod_fast(x: i32, n: usize) -> usize {
        let n = n as i32;
        ((x % n + n) % n) as usize
    }

    #[inline]
    fn noise(&self, x: f64, y: f64) -> f64 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;

        let fx = x - x.floor();
        let fy = y - y.floor();

        let x0 = Self::mod_fast(xi, WAVELET_TILE_SIZE);
        let x1 = Self::mod_fast(xi + 1, WAVELET_TILE_SIZE);
        let y0 = Self::mod_fast(yi, WAVELET_TILE_SIZE);
        let y1 = Self::mod_fast(yi + 1, WAVELET_TILE_SIZE);

        let v00 = self.noise_tile[y0 * WAVELET_TILE_SIZE + x0];
        let v10 = self.noise_tile[y0 * WAVELET_TILE_SIZE + x1];
        let v01 = self.noise_tile[y1 * WAVELET_TILE_SIZE + x0];
        let v11 = self.noise_tile[y1 * WAVELET_TILE_SIZE + x1];

        let v0 = lerp(fx, v00, v10);
        let v1 = lerp(fx, v01, v11);
        lerp(fy, v0, v1)
    }

    fn generate_coloring(&self, settings: WaveletNoiseSettings) -> Vec<u8> {
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

    pub fn fbm_standard(&self, x: f64, y: f64, settings: &WaveletNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let h_exponent = settings.h_exponent.value();
        let lacunarity = settings.lacunarity.value();

        for i in 1..=octaves {
            let noise_val = self.noise(x * frequency, y * frequency);

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

    pub fn fbm_turbulence(&self, x: f64, y: f64, settings: &WaveletNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();

        for i in 1..=octaves {
            let noise_val = self.noise(x * frequency, y * frequency).abs();

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

    pub fn fbm_ridge(&self, x: f64, y: f64, settings: &WaveletNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;
        let mut weight = 1.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();

        for i in 1..=octaves {
            let noise_val = self.noise(x * frequency, y * frequency).abs();
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

    pub fn fbm_domain_warp(&self, x: f64, y: f64, settings: &WaveletNoiseSettings) -> f64 {
        let warp_amount = settings.warp_amount.value();

        let adjusted_settings = WaveletNoiseSettings {
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

impl WaveletNoise {
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

    fn generate_and_draw(settings: WaveletNoiseSettings) {
        let wavelet = WaveletNoiseImpl::new(settings.seed.value());

        let coloring = wavelet.generate_coloring(settings.clone());

        draw_noise(coloring.as_slice());

        if settings.show_grid.value() {
            draw_grid(settings.scale.value(), "#000000");
        }
    }
}

define_noise!(wavelet,
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
    checkboxes:[show_grid];
);
