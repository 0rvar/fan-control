use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::rley::Rgb565Rle;

pub struct LeekSpin {
    next_frame: u8,
    next_frame_at_ms: u32,
}

impl LeekSpin {
    pub fn new() -> Self {
        Self {
            next_frame: 0,
            next_frame_at_ms: 0,
        }
    }

    pub fn render<D>(
        &mut self,
        target: &mut D,
        clock_ms: u32,
        y_range: (u32, u32),
        rpm: u32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        if self.next_frame_at_ms > clock_ms {
            return Ok(());
        }
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

        // We want the speed to depend on the RPM
        // 0 RPM = 300ms per frame
        // 2000+ rpm = 90ms per frame
        let rpm_percent = (rpm as f32 / 2000.0).clamp(0.0, 1.0);
        let frame_delay = (1000.0 - 910.0 * rpm_percent) as u32;

        self.next_frame_at_ms = clock_ms + frame_delay;

        Ok(())
    }
}
