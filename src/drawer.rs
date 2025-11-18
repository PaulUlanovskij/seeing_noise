use std::cell::LazyCell;
use std::f64::consts::PI;
use wasm_bindgen::prelude::*;

use web_sys::CanvasRenderingContext2d;

use crate::log;
use crate::console_log;

pub const GRID_THICKNESS: u32 = 2;
pub const HALF_GRID_THICKNESS: u32 = GRID_THICKNESS / 2;
pub const RESOLUTION: u32 = 400;
pub const HALF_RESOLUTION: u32 = RESOLUTION / 2;
pub const IMAGE_BYTES_COUNT: u32 = RESOLUTION * RESOLUTION * 4;

thread_local! {
    pub static CANVAS_CONTEXT: LazyCell<CanvasRenderingContext2d> = LazyCell::new(||{
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        canvas.set_width(RESOLUTION);
        canvas.set_height(RESOLUTION);

        canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap()
    });
}

pub fn draw_noise(data: &[u8]) {
    assert!(data.len() as u32 == IMAGE_BYTES_COUNT);

    let clamped = wasm_bindgen::Clamped(data);
    let imagedata =
        web_sys::ImageData::new_with_u8_clamped_array_and_sh(clamped, RESOLUTION, RESOLUTION)
            .map_err(|_| console_log!("Creating image data failed"))
            .unwrap();
    CANVAS_CONTEXT
        .with(|ctx| ctx.put_image_data(&imagedata, 0., 0.))
        .map_err(|_| console_log!("Drawing noise to canvas failed"))
        .unwrap();
}

pub fn draw_grid(scale: f64, fill_style: &str) {
    CANVAS_CONTEXT.with(|context| {
        context.set_fill_style_str(fill_style);
        for i in 0..=(HALF_RESOLUTION as f64 / scale) as usize {
            let raw_offset = scale * i as f64;

            let offset = HALF_RESOLUTION as f64 - raw_offset - HALF_GRID_THICKNESS as f64;
            context.fill_rect(offset, 0., GRID_THICKNESS as f64, RESOLUTION as f64);
            context.fill_rect(0., offset, RESOLUTION as f64, GRID_THICKNESS as f64);

            let offset = HALF_RESOLUTION as f64 + raw_offset - HALF_GRID_THICKNESS as f64;
            context.fill_rect(offset, 0., GRID_THICKNESS as f64, RESOLUTION as f64);
            context.fill_rect(0., offset, RESOLUTION as f64, GRID_THICKNESS as f64);
        }
    });
}

pub fn draw_arrow(from_x: f64, from_y: f64, to_x: f64, to_y: f64, head_length: f64, fill_style: &str) {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let angle = dy.atan2(dx);

    CANVAS_CONTEXT.with(|context| {
        context.set_stroke_style_str(fill_style);
        context.begin_path();
        context.move_to(from_x, from_y);
        context.line_to(to_x, to_y);

        context.line_to(
            to_x - head_length * (angle - std::f64::consts::PI / 6.0).cos(),
            to_y - head_length * (angle - std::f64::consts::PI / 6.0).sin(),
        );
        context.move_to(to_x, to_y);
        context.line_to(
            to_x - head_length * (angle + std::f64::consts::PI / 6.0).cos(),
            to_y - head_length * (angle + std::f64::consts::PI / 6.0).sin(),
        );

        context.stroke();
    });
}

pub fn draw_circle(x: f64, y: f64, radius: f64, fill_style: &str) {

    CANVAS_CONTEXT.with(|context| {
        context.set_fill_style_str(fill_style);
        context.begin_path();
        let _ = context.arc(x, y, radius, 0., 2.*PI).ok();
        context.fill();
    });
}
