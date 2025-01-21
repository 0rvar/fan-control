use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::rley::Rgb565Rle;

pub struct LeekSpin {
    next_frame: u8,
}

impl LeekSpin {
    pub fn new() -> Self {
        Self { next_frame: 0 }
    }

    pub fn render<D>(
        &mut self,
        target: &mut D,
        clock_ms: u32,
        y_range: (u32, u32),
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let compressed_data: &[u8] = match self.next_frame {
            0 => include_bytes!("./leek_spin-0.rle"),
            1 => include_bytes!("./leek_spin-1.rle"),
            2 => include_bytes!("./leek_spin-2.rle"),
            3 => include_bytes!("./leek_spin-3.rle"),
            _ => panic!("Invalid frame index"),
        };

        if let Some(image) = Rgb565Rle::new(compressed_data).map(|x| x.limit(y_range)) {
            image.draw(target)?;
        }
        self.next_frame = (self.next_frame + 1) % 4;

        Ok(())
    }
}
