pub trait Noise {
    fn setup();
    fn select();
    fn update();
    fn deselect();
    fn reset();
}
