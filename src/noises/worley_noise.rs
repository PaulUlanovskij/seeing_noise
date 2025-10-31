use std::cell::LazyCell;

use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{HtmlElement, HtmlInputElement};

use super::noise::Noise;
use crate::{
    drawer::{draw_circle, IMAGE_BYTES_COUNT},
    noises::helpers::{lerp, shuffle},
    *,
};

struct WorleyNoiseImpl {
    permutation: [usize; 256],
}

impl WorleyNoiseImpl {
    pub fn new(seed: u32) -> Self {
        let mut permutation: [usize; 256] = std::array::from_fn(|i| i);
        shuffle(&mut permutation, seed);

        WorleyNoiseImpl { permutation }
    }

    #[inline]
    fn hash2d(&self, x: i32, y: i32) -> (f64, f64) {
        let xi = (x & 255) as usize;
        let yi = (y & 255) as usize;
        let h = self.permutation[(self.permutation[xi] + yi) & 255];
        
        // Generate pseudo-random offset within cell [0, 1)
        let fx = ((h * 127) % 256) as f64 / 256.0;
        let fy = ((h * 311) % 256) as f64 / 256.0;
        (fx, fy)
    }

    #[inline]
    fn worley_distance(&self, x: f64, y: f64, distance_metric: DistanceMetric) -> (f64, f64) {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - xi as f64;
        let yf = y - yi as f64;

        let mut min_dist1 = f64::MAX;
        let mut min_dist2 = f64::MAX;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let cell_x = xi + dx;
                let cell_y = yi + dy;
                
                let (offset_x, offset_y) = self.hash2d(cell_x, cell_y);
                let point_x = dx as f64 + offset_x;
                let point_y = dy as f64 + offset_y;

                let dist = match distance_metric {
                    DistanceMetric::Euclidean => {
                        let dx = point_x - xf;
                        let dy = point_y - yf;
                        (dx * dx + dy * dy).sqrt()
                    }
                    DistanceMetric::Manhattan => {
                        (point_x - xf).abs() + (point_y - yf).abs()
                    }
                    DistanceMetric::Chebyshev => {
                        (point_x - xf).abs().max((point_y - yf).abs())
                    }
                    DistanceMetric::Minkowski => {
                        let p = 3.0; 
                        let dx = (point_x - xf).abs();
                        let dy = (point_y - yf).abs();
                        (dx.powf(p) + dy.powf(p)).powf(1.0 / p)
                    }
                };

                if dist < min_dist1 {
                    min_dist2 = min_dist1;
                    min_dist1 = dist;
                } else if dist < min_dist2 {
                    min_dist2 = dist;
                }
            }
        }

        (min_dist1, min_dist2)
    }

    fn generate_coloring(&self, settings: WorleyNoiseSettings) -> Vec<u8> {
        let mut v = Vec::with_capacity(IMAGE_BYTES_COUNT as usize);
        let scale = settings.scale.value();

        for y in 0..RESOLUTION {
            for x in 0..RESOLUTION {
                let nx = ((x as f64) - (HALF_RESOLUTION as f64)) / scale;
                let ny = ((y as f64) - (HALF_RESOLUTION as f64)) / scale;

                let noise_val = match settings.noise_type.clone() {
                    NoiseType::F1 => self.fbm_f1(nx, ny, &settings),
                    NoiseType::F2MinusF1 => self.fbm_f2_minus_f1(nx, ny, &settings),
                    NoiseType::Crackle => self.fbm_crackle(nx, ny, &settings),
                    NoiseType::DomainWarp => self.fbm_domain_warp(nx, ny, &settings),
                };

                let normalized = noise_val.clamp(-1.0, 1.0);

                if normalized < 0. {
                    let t = normalized + 1.;
                    v.push(255);
                    v.push(lerp(t, 0.0, 255.0) as u8);
                    v.push(255);
                    v.push(255);
                } else {
                    let t = normalized;
                    v.push(lerp(t, 255.0, 0.0) as u8);
                    v.push(255);
                    v.push(lerp(t, 255.0, 0.0) as u8);
                    v.push(255);
                }
            }
        }
        v
    }

    pub fn fbm_f1(&self, x: f64, y: f64, settings: &WorleyNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();
        let distance_metric = settings.distance_metric.clone();

        for i in 1..=octaves {
            let (f1, _) = self.worley_distance(
                x * frequency, 
                y * frequency, 
                distance_metric.clone()
            );

            let include = match settings.visualization {
                Visualization::Final => true,
                Visualization::SingleOctave => i == show_octave,
                Visualization::AccumulatedOctaves => i <= show_octave,
            };
            
            if include {
                let noise_val = 1.0 - f1.min(1.0);
                total += noise_val * amplitude;
                max_value += amplitude;
            }
            
            amplitude *= gain;
            frequency *= lacunarity;
        }

        (total / max_value) * 2.0 - 1.0
    }

    pub fn fbm_f2_minus_f1(&self, x: f64, y: f64, settings: &WorleyNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();
        let distance_metric = settings.distance_metric.clone();

        for i in 1..=octaves {
            let (f1, f2) = self.worley_distance(
                x * frequency, 
                y * frequency, 
                distance_metric.clone()
            );

            let include = match settings.visualization {
                Visualization::Final => true,
                Visualization::SingleOctave => i == show_octave,
                Visualization::AccumulatedOctaves => i <= show_octave,
            };
            
            if include {
                let noise_val = (f2 - f1).min(1.0);
                total += noise_val * amplitude;
                max_value += amplitude;
            }
            
            amplitude *= gain;
            frequency *= lacunarity;
        }

        (total / max_value) * 2.0 - 1.0
    }

    pub fn fbm_crackle(&self, x: f64, y: f64, settings: &WorleyNoiseSettings) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        let mut max_value = 0.0;

        let octaves = settings.octaves.value();
        let show_octave = settings.show_octave.value();
        let gain = settings.gain.value();
        let lacunarity = settings.lacunarity.value();
        let distance_metric = settings.distance_metric.clone();
        let crackle_power = settings.crackle_power.value();

        for i in 1..=octaves {
            let (f1, _) = self.worley_distance(
                x * frequency, 
                y * frequency, 
                distance_metric.clone()
            );

            let include = match settings.visualization {
                Visualization::Final => true,
                Visualization::SingleOctave => i == show_octave,
                Visualization::AccumulatedOctaves => i <= show_octave,
            };
            
            if include {
                let noise_val = f1.min(1.0).powf(crackle_power);
                total += noise_val * amplitude;
                max_value += amplitude;
            }
            
            amplitude *= gain;
            frequency *= lacunarity;
        }

        1.0 - (total / max_value) * 2.0
    }

    pub fn fbm_domain_warp(&self, x: f64, y: f64, settings: &WorleyNoiseSettings) -> f64 {
        let warp_amount = settings.warp_amount.value();

        let adjusted_settings = WorleyNoiseSettings {
            noise_type: NoiseType::F1,
            ..settings.clone()
        };
        
        let qx = self.fbm_f1(x, y, &adjusted_settings);
        let qy = self.fbm_f1(x + 5.2, y + 1.3, &adjusted_settings);

        let rx = x + warp_amount * qx;
        let ry = y + warp_amount * qy;

        self.fbm_f1(rx, ry, &adjusted_settings)
    }
}

impl WorleyNoise {
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
            NoiseType::F1 => {
                set_hidden!(crackle_power_control, true);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::F2MinusF1 => {
                set_hidden!(crackle_power_control, true);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::Crackle => {
                set_hidden!(crackle_power_control, false);
                set_hidden!(warp_amount_control, true);
            }
            NoiseType::DomainWarp => {
                set_hidden!(crackle_power_control, true);
                set_hidden!(warp_amount_control, false);
            }
        }
    }
    
    fn generate_and_draw(settings: WorleyNoiseSettings) {
        let worley = WorleyNoiseImpl::new(settings.seed.value());

        let coloring = worley.generate_coloring(settings.clone());

        draw_noise(coloring.as_slice());

        if settings.show_grid.value() {
            draw_grid(settings.scale.value(), "#000000");
        }

        if settings.show_points.value() {
            Self::draw_feature_points(&settings, worley);
        }
    }

    fn draw_feature_points(settings: &WorleyNoiseSettings, noise: WorleyNoiseImpl) {
        let scale = settings.scale.value();

        for i in 0..settings.octaves.value() {
            let octave_scale = scale / 2_f64.powi(i as i32);
            let half_range = (HALF_RESOLUTION as f64 / octave_scale).floor() as isize;

            for x in -half_range..=half_range {
                for y in -half_range..=half_range {
                    let (offset_x, offset_y) = noise.hash2d(x as i32, y as i32);
                    
                    let xf = HALF_RESOLUTION as f64 - (x as f64 + offset_x) * octave_scale;
                    let yf = HALF_RESOLUTION as f64 - (y as f64 + offset_y) * octave_scale;

                    let radius = octave_scale / 10.0;
                    draw_circle(xf, yf, radius, "#ee0000");
                }
            }
        }
    }
}

define_noise!(worley,
    sliders:[
        (seed, u32, 42.),
        (scale, f64, 50.),
        (octaves, u32, 1.),
        (lacunarity, f64, 2.0),
        (gain, f64, 0.5),
        (crackle_power, f64, 2.0),
        (warp_amount, f64, 0.5),
        (show_octave, u32, 1.)
    ];
    radios:[
        (visualization, final, single_octave, accumulated_octaves),
        (noise_type, f1, f2_minus_f1, crackle, domain_warp),
        (distance_metric, euclidean, manhattan, chebyshev, minkowski)
    ];
    checkboxes:[show_grid, show_points];
);
