pub fn shuffle(v: &mut [usize; 256], seed: u32) {
    for i in (1..256).rev() {
        let r = squirrel_noise5::squirrel_noise5(i as u32, seed);
        let j = (r as usize) % (i + 1);
        v.swap(i, j);
    }
}

#[inline]
pub const fn perlin_grad(hash: usize, x: f64, y: f64) -> f64 {
    let (xm, ym) = get_perlin_vec(hash);
    xm*x + ym*y
}

#[inline]
pub const fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

#[inline]
pub const fn get_perlin_vec(hash: usize) -> (f64, f64){
    match hash & 7{
        0 => (1., 0.),
        1 => (1., 1.),
        2 => (0., 1.),
        3 => (-1., 1.),
        4 => (-1., 0.),
        5 => (-1., -1.),
        6 => (0., -1.),
        _ => (1., -1.),
    }
}
