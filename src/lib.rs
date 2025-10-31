#![recursion_limit = "1024"]

use std::{cell::LazyCell, sync::Mutex};

use wasm_bindgen::prelude::*;
mod noises;
use noises::perlin_noise::PerlinNoise;
use web_sys::{Document, Element, HtmlSelectElement};

use crate::{
    drawer::{HALF_RESOLUTION, RESOLUTION, draw_grid, draw_noise},
    noises::{
        anisotropic_noise::AnisotropicNoise, gabor_noise::GaborNoise, noise::Noise,
        simplex_noise::SimplexNoise, wavelet_noise::WaveletNoise, worley_noise::WorleyNoise,
    },
};
mod drawer;
mod log;
mod macros;

thread_local! {
    pub static DOCUMENT: LazyCell<Document> = LazyCell::new(||{
        web_sys::window().unwrap().document().unwrap()
    });
}
elements!(noise, (select, HtmlSelectElement),);
static CURRENT_NOISE: Mutex<String> = Mutex::new(String::new());

pub fn get_element_by_id(id: &str) -> Element {
    DOCUMENT.with(|doc| {
        doc.get_element_by_id(id).unwrap_or_else(|| {
            console_log!("Failed to get element with id {id}");
            unreachable!()
        })
    })
}

fn change_noise() {
    let new_noise = parse_value!(select, String);
    let mut current_noise = CURRENT_NOISE.lock().unwrap();

    match current_noise.as_str() {
        "perlin" => PerlinNoise::deselect(),
        "simplex" => SimplexNoise::deselect(),
        "wavelet" => WaveletNoise::deselect(),
        "gabor" => GaborNoise::deselect(),
        "anisotropic" => AnisotropicNoise::deselect(),
        "worley" => WorleyNoise::deselect(),
        _ => (),
    }

    match new_noise.as_str() {
        "perlin" => PerlinNoise::select(),
        "simplex" => SimplexNoise::select(),
        "wavelet" => WaveletNoise::select(),
        "gabor" => GaborNoise::select(),
        "anisotropic" => AnisotropicNoise::select(),
        "worley" => WorleyNoise::select(),
        e => {
            console_log!("Unknown noise was selected: {e}");
            return;
        }
    }
    current_noise.clear();
    current_noise.push_str(new_noise.as_str());
}

#[wasm_bindgen(start)]
fn start() {
    add_callback!(select, "input", change_noise);
    PerlinNoise::setup();
    SimplexNoise::setup();
    WaveletNoise::setup();
    GaborNoise::setup();
    AnisotropicNoise::setup();
    WorleyNoise::setup();
}
