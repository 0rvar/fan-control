use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_hal::delay::DelayNs;

pub struct LeekSpin {
    frame_index: u8,
}
impl LeekSpin {
    pub fn new() -> Self {
        Self { frame_index: 0 }
    }
    pub fn render<D, E>(&mut self, display: &mut D)
    where
        D: DrawTarget<Color = Rgb565, Error = E>,
        E: std::fmt::Debug,
    {
        let bytes = match self.frame_index {
            0 => include_bytes!("./animations/leek_spin-0.bmp"),
            1 => include_bytes!("./animations/leek_spin-1.bmp"),
            2 => include_bytes!("./animations/leek_spin-2.bmp"),
            3 => include_bytes!("./animations/leek_spin-3.bmp"),
            _ => panic!("Invalid frame index"),
        };
        self.frame_index = (self.frame_index + 1) % 4;
        let spin_frame = tinybmp::Bmp::from_slice(bytes).unwrap();
        spin_frame.draw(display).unwrap();
    }
}
