use std::cell::LazyCell;

use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{HtmlElement, HtmlInputElement};

use super::noise::Noise;
use crate::{
    drawer::{IMAGE_BYTES_COUNT, draw_arrow},
    noises::helpers::{lerp, perlin_grad, shuffle},
    *,
};

struct AnisotropicNoiseImpl {
    permutation: [usize; 256],
}

impl AnisotropicNoiseImpl {
    pub fn new(seed: u32) -> Self {
        let mut permutation: [usize; 256] = std::array::from_fn(|i| i);
        shuffle(&mut permutation, seed);

        AnisotropicNoiseImpl { permutation }
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
    fn noise_anisotropic(&self, x: f64, y: f64, angle: f64, anisotropy: f64) -> f64 {
        let scale_x = 1.0;
        let scale_y = 1.0 / anisotropy.max(0.1); 

        let sx = x * scale_x;
        let sy = y * scale_y;

        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let rx = sx * cos_a - sy * sin_a;
        let ry = sx * sin_a + sy * cos_a;

        let xi = rx.floor() as i32;
        let yi = ry.floor() as i32;

        let xf = rx - xi as f64;
        let yf = ry - yi as f64;

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

    fn generate_coloring(&self, settings: AnisotropicNoiseSettings) -> Vec<u8> {
        let mut v = Vec::with_capacity(IMAGE_BYTES_COUNT as usize);
        let scale = settings.scale.value();

        for y in 0..RESOLUTION {
            for x in 0..RESOLUTION {
                let nx = ((x as f64) - (HALF_RESOLUTION as f64)) / scale;
                let ny = ((y as f64) - (HALF_RESOLUTION as f64)) / scale;

                let noise_val = match settings.noise_type {
                    NoiseType::Standard => self.fbm_standard(nx, ny, &settings),
                    NoiseType::Turbulence => self.fbm_turbulence(nx, ny, &settings),
                    NoiseType::Ridge => self.fbm_ridge(nx, ny, &settings),
                    NoiseType::Directional => self.fbm_directional(nx, ny, &settings),
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

    pub fn fbm_standard(&self, x: f64, y: f64, settings: &AnisotropicNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let h_exponent = settings.h_exponent.value();
        let lacunarity = settings.lacunarity.value();
        let angle = settings.angle.value().to_radians();
        let anisotropy = settings.anisotropy.value();
        
        for i in 1..=octaves {
            let noise_val = self.noise_anisotropic(
                x * frequency, 
                y * frequency, 
                angle,
                anisotropy
            );

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

    pub fn fbm_turbulence(&self, x: f64, y: f64, settings: &AnisotropicNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();
        let angle = settings.angle.value().to_radians();
        let anisotropy = settings.anisotropy.value();
        
        for i in 1..=octaves {
            let noise_val = self.noise_anisotropic(
                x * frequency, 
                y * frequency, 
                angle,
                anisotropy
            ).abs();

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

    pub fn fbm_ridge(&self, x: f64, y: f64, settings: &AnisotropicNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;
        let mut weight = 1.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();
        let angle = settings.angle.value().to_radians();
        let anisotropy = settings.anisotropy.value();
        
        for i in 1..=octaves {
            let noise_val = self.noise_anisotropic(
                x * frequency, 
                y * frequency, 
                angle,
                anisotropy
            ).abs();
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

    pub fn fbm_directional(&self, x: f64, y: f64, settings: &AnisotropicNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();
        let base_angle = settings.angle.value().to_radians();
        let angle_step = settings.angle_step.value().to_radians();
        let anisotropy = settings.anisotropy.value();
        
        for i in 1..=octaves {
            let current_angle = base_angle + angle_step * (i - 1) as f64;
            
            let noise_val = self.noise_anisotropic(
                x * frequency, 
                y * frequency, 
                current_angle,
                anisotropy
            );

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
}

impl AnisotropicNoise {
    fn on_setup() {}
    
    fn on_update() {
        let octaves = Octaves::parse().value();
        SHOW_OCTAVE.with(|e| e.set_max(format!("{octaves}").as_str()));
    }
    
    fn generate_and_draw(settings: AnisotropicNoiseSettings) {
        let anisotropic = AnisotropicNoiseImpl::new(settings.seed.value());

        let coloring = anisotropic.generate_coloring(settings.clone());

        draw_noise(coloring.as_slice());

        if settings.show_grid.value() {
            draw_grid(settings.scale.value(), "#000000");
        }

        if settings.show_direction.value() {
            Self::draw_direction_indicator(&settings);
        }
    }

    fn draw_direction_indicator(settings: &AnisotropicNoiseSettings) {
        let angle = settings.angle.value().to_radians();
        let center_x = HALF_RESOLUTION as f64;
        let center_y = HALF_RESOLUTION as f64;
        let length = 80.0;
        
        let end_x = center_x + angle.cos() * length;
        let end_y = center_y + angle.sin() * length;
        draw_arrow(center_x, center_y, end_x, end_y, 15.0, "#00ff00");
        
        let perp_angle = angle + std::f64::consts::PI / 2.0;
        let anisotropy = settings.anisotropy.value();
        let perp_length = length * anisotropy;
        let perp_end_x = center_x + perp_angle.cos() * perp_length;
        let perp_end_y = center_y + perp_angle.sin() * perp_length;
        draw_arrow(center_x, center_y, perp_end_x, perp_end_y, 10.0, "#0088ff");
    }
}

define_noise!(anisotropic,
    sliders:[
        (seed, u32, 0., 42., 1000.),
        (scale, f64, 10., 50., 200.),
        (octaves, u32, 1., 1., 8.),
        (lacunarity, f64, 1., 2., 4.),
        (gain, f64, 0., 0.5, 1.),
        (h_exponent, f64, 0., 1., 2.),
        (ridge_offset, f64, 0., 1., 2.),
        (angle, f64, 0.0, 0.0, 360.0),          
        (anisotropy, f64, 0.1, 1.0, 5.0),     
        (angle_step, f64, -90., 0.0, 90.),     
        (show_octave, u32, 1., 1., 8.)
    ];
    radios:[
        (visualization, 
            (final, hide: [show_octave]), 
            (single_octave), 
            (accumulated_octaves)
        ),
        (noise_type, 
            (standard, hide: [ridge_offset, angle_step]), 
            (turbulence, hide:[h_exponent, ridge_offset, angle_step]), 
            (ridge, hide:[h_exponent, angle_step]), 
            (directional, hide:[h_exponent, ridge_offset])
        )
    ];
    checkboxes:[show_grid, show_direction];
);
