use std::cell::LazyCell;

use rayon::prelude::*;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{HtmlElement, HtmlInputElement};

use super::noise::Noise;
use crate::{
    drawer::{IMAGE_BYTES_COUNT, draw_arrow},
    noises::helpers::{lerp, shuffle},
    *,
};

struct GaborNoiseImpl {
    permutation: [usize; 256],
}

impl GaborNoiseImpl {
    pub fn new(seed: u32) -> Self {
        let mut permutation: [usize; 256] = std::array::from_fn(|i| i);
        shuffle(&mut permutation, seed);

        GaborNoiseImpl { permutation }
    }

    #[inline]
    fn hash(&self, x: i32, y: i32) -> usize {
        let xi = (x & 255) as usize;
        let yi = (y & 255) as usize;
        self.permutation[(self.permutation[xi] + yi) & 255]
    }

    #[inline]
    fn hash_to_float(&self, hash: usize, offset: u32) -> f64 {
        squirrel_noise5::f32_zero_to_one_1d(hash as i32, offset as i32) as f64
    }

    fn sample_gabor_sparse(
        &self,
        x: f64,
        y: f64,
        frequency: f64,
        bandwidth: f64,
        kernel_radius: u32,
    ) -> f64 {
        let kernel_radius = kernel_radius as f64;
        let mut sum = 0.0;
        let mut weight = 0.0;
        
        let cell_x = x.floor() as i32;
        let cell_y = y.floor() as i32;
        
        let cell_radius = (kernel_radius * bandwidth).ceil() as i32;
        
        for dy in -cell_radius..=cell_radius {
            for dx in -cell_radius..=cell_radius {
                let cx = cell_x + dx;
                let cy = cell_y + dy;
                
                let cell_hash = self.hash(cx, cy);
                
                let ix = cx as f64 + 0.5 + (self.hash_to_float(cell_hash, 0) - 0.5) * 0.8;
                let iy = cy as f64 + 0.5 + (self.hash_to_float(cell_hash, 1) - 0.5) * 0.8;
                
                let dx = x - ix;
                let dy = y - iy;
                let dist_sq = dx * dx + dy * dy;
                
                let max_dist = kernel_radius * bandwidth;
                if dist_sq > max_dist * max_dist {
                    continue;
                }
                
                let theta = self.hash_to_float(cell_hash, 2) * 2.0 * std::f64::consts::PI;
                let phi = self.hash_to_float(cell_hash, 3) * 2.0 * std::f64::consts::PI;
                
                let gaussian_exp = -std::f64::consts::PI * dist_sq / (bandwidth * bandwidth);
                let gaussian = gaussian_exp.exp();
                
                let u = dx * theta.cos() - dy * theta.sin();
                let harmonic = (frequency * u + phi).cos();
                
                let kernel_value = gaussian * harmonic;
                sum += kernel_value;
                weight += gaussian;
            }
        }
        
        if weight > 0.001 {
            sum / weight.sqrt()
        } else {
            0.0
        }
    }

    fn generate_coloring(&self, settings: GaborNoiseSettings) -> Vec<u8> {
        let scale = settings.scale.value();

        (0..(RESOLUTION * RESOLUTION) as usize)
            .into_par_iter()
            .flat_map(|i| {
                let x = i % RESOLUTION as usize;
                let y = i / RESOLUTION as usize;
                let nx = ((x as f64) - (HALF_RESOLUTION as f64)) / scale;
                let ny = ((y as f64) - (HALF_RESOLUTION as f64)) / scale;

                let noise_val = match settings.noise_type {  // Removed clone()
                    NoiseType::Standard => self.fbm_standard(nx, ny, &settings),
                    NoiseType::Turbulence => self.fbm_turbulence(nx, ny, &settings),
                    NoiseType::Anisotropic => self.fbm_anisotropic(nx, ny, &settings),
                    NoiseType::DomainWarp => self.fbm_domain_warp(nx, ny, &settings),
                };

                if noise_val < 0.0 {
                    let t = noise_val + 1.0;
                    [255u8, lerp(t, 0.0, 255.0) as u8, 255, 255]
                } else {
                    [lerp(noise_val, 255.0, 0.0) as u8, 255, lerp(noise_val, 255.0, 0.0) as u8, 255]
                }
            })
            .collect()
    }

    pub fn fbm_standard(&self, x: f64, y: f64, settings: &GaborNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = settings.base_frequency.value();
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let bandwidth = settings.bandwidth.value();
        let kernel_radius = settings.kernel_radius.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();

        for i in 1..=octaves {
            let noise_val = self.sample_gabor_sparse(x, y, frequency, bandwidth, kernel_radius);

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

        total / max_value.max(0.001)
    }

    pub fn fbm_turbulence(&self, x: f64, y: f64, settings: &GaborNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = settings.base_frequency.value();
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let bandwidth = settings.bandwidth.value();
        let kernel_radius = settings.kernel_radius.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();

        for i in 1..=octaves {
            let noise_val = self.sample_gabor_sparse(x, y, frequency, bandwidth, kernel_radius).abs();

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

        total / max_value.max(0.001)
    }

    pub fn fbm_anisotropic(&self, x: f64, y: f64, settings: &GaborNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = settings.base_frequency.value();
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let bandwidth = settings.bandwidth.value();
        let kernel_radius = settings.kernel_radius.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();
        let anisotropy = settings.anisotropy.value();

        for i in 1..=octaves {
            let aniso_x = x * anisotropy;
            let aniso_y = y / anisotropy;
            
            let noise_val = self.sample_gabor_sparse(aniso_x, aniso_y, frequency, bandwidth, kernel_radius);

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

        total / max_value.max(0.001)
    }

    pub fn fbm_domain_warp(&self, x: f64, y: f64, settings: &GaborNoiseSettings) -> f64 {
        let warp_amount = settings.warp_amount.value();

        let qx = self.fbm_standard(x, y, settings);
        let qy = self.fbm_standard(x + 5.2, y + 1.3, settings);

        let rx = x + warp_amount * qx;
        let ry = y + warp_amount * qy;

        self.fbm_standard(rx, ry, settings)
    }

    fn draw_impulse_locations(&self, settings: &GaborNoiseSettings) {
        let scale = settings.scale.value();

        for i in 0..settings.octaves.value() {
            let octave_scale = scale / 2_f64.powi(i as i32);
            let half_range = (HALF_RESOLUTION as f64 / octave_scale).floor() as isize;

            for x in -half_range..=half_range {
                for y in -half_range..=half_range {
                    let cell_hash = self.hash(x as i32, y as i32);
                    
                    let ix = x as f64 + 0.5 + (self.hash_to_float(cell_hash, 0) - 0.5) * 0.8;
                    let iy = y as f64 + 0.5 + (self.hash_to_float(cell_hash, 1) - 0.5) * 0.8;
                    
                    let screen_x = HALF_RESOLUTION as f64 - ix * octave_scale;
                    let screen_y = HALF_RESOLUTION as f64 - iy * octave_scale;
                    
                    let theta = self.hash_to_float(cell_hash, 2) * 2.0 * std::f64::consts::PI;
                    let arrow_len = octave_scale / 3.0;
                    let tx = screen_x + theta.cos() * arrow_len;
                    let ty = screen_y + theta.sin() * arrow_len;
                    
                    draw_arrow(screen_x, screen_y, tx, ty, octave_scale / 8.0, "#ee0000");
                }
            }
        }
    }
}

impl GaborNoise {
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
                set_hidden!(anisotropy_control, true);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::Turbulence => {
                set_hidden!(anisotropy_control, true);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::Anisotropic => {
                set_hidden!(anisotropy_control, false);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::DomainWarp => {
                set_hidden!(anisotropy_control, true);
                set_hidden!(warp_amount_control, false);
            }
        }
    }
    
    fn generate_and_draw(settings: GaborNoiseSettings) {
        let gabor = GaborNoiseImpl::new(settings.seed.value());

        let coloring = gabor.generate_coloring(settings.clone());

        draw_noise(coloring.as_slice());

        if settings.show_grid.value() {
            draw_grid(settings.scale.value(), "#000000");
        }

        if settings.show_impulses.value() {
            gabor.draw_impulse_locations(&settings);
        }
    }
}

define_noise!(gabor,
    sliders:[
        (seed, u32, 42.),
        (scale, f64, 50.),
        (octaves, u32, 1.),
        (lacunarity, f64, 2.0),
        (gain, f64, 0.5),
        (base_frequency, f64, 10.0),
        (bandwidth, f64, 0.5),
        (kernel_radius, u32, 3.),
        (anisotropy, f64, 1.0),
        (warp_amount, f64, 4.0),
        (show_octave, u32, 1.)
    ];
    radios:[
        (visualization, final, single_octave, accumulated_octaves),
        (noise_type, standard, turbulence, anisotropic, domain_warp)
    ];
    checkboxes:[show_grid, show_impulses];
);
