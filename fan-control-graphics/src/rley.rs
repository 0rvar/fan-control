use embedded_graphics::{
    image::ImageDrawable,
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::{DrawTarget, OriginDimensions, Point, Size},
    Pixel,
};

#[derive(Debug)]
pub struct Rgb565Rle<'a> {
    width: u32,
    height: u32,
    palette: Vec<Rgb565>,
    data: &'a [u8],
}

impl<'a> Rgb565Rle<'a> {
    pub fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < 9 {
            // width(4) + height(4) + palette_size(1)
            return None;
        }

        let width = u32::from_le_bytes(data[0..4].try_into().ok()?);
        let height = u32::from_le_bytes(data[4..8].try_into().ok()?);
        let palette_size = data[8] as usize;

        if palette_size > 64 || data.len() < 9 + palette_size * 2 {
            return None;
        }

        // Convert palette data to Rgb565 colors
        let palette_data = &data[9..9 + palette_size * 2];
        let mut palette = Vec::with_capacity(palette_size);
        for chunk in palette_data.chunks_exact(2) {
            let color = u16::from_le_bytes([chunk[0], chunk[1]]);
            palette.push(RawU16::from(color).into());
        }

        Some(Self {
            width,
            height,
            palette,
            data: &data[9 + palette_size * 2..],
        })
    }
}

impl OriginDimensions for Rgb565Rle<'_> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl<'a> ImageDrawable for Rgb565Rle<'a> {
    type Color = Rgb565;

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let mut x = 0;
        let mut y = 0;
        let mut i = 0;

        let bounding_box = target.bounding_box();
        let mut offset_x = 0;
        let mut offset_y = 0;
        if let Some(corner) = bounding_box.bottom_right() {
            offset_x = corner.x / 2 - self.width as i32 / 2;
            offset_y = corner.y / 2 - self.height as i32 / 2;
        }

        // Create a buffer to store pixels before drawing
        let buffer_rows = self.height / 4;
        let mut pixel_buffer = Vec::with_capacity((self.width * buffer_rows) as usize);
        let mut current_buffer_row = 0;

        while i < self.data.len() {
            if x >= self.width {
                x = 0;
                y += 1;
                current_buffer_row += 1;
            }

            // Draw buffered pixels if buffer is full or we're at the end of the image
            if current_buffer_row >= buffer_rows || y >= self.height {
                if !pixel_buffer.is_empty() {
                    target.draw_iter(pixel_buffer.iter().cloned())?;
                    pixel_buffer.clear();
                    current_buffer_row = 0;
                }
            }

            let packet = self.data[i];
            let is_rle = packet & 0x80 != 0;
            let palette_idx = (packet & 0x7F) as usize;

            if palette_idx >= self.palette.len() {
                break;
            }

            let color = self.palette[palette_idx];

            if is_rle {
                if i + 1 >= self.data.len() {
                    break;
                }
                let count = self.data[i + 1] as u32;

                // Add RLE pixels to buffer
                for dx in 0..count {
                    if x + dx < self.width {
                        pixel_buffer.push(Pixel(
                            Point::new((x + dx) as i32 + offset_x, y as i32 + offset_y),
                            color,
                        ));
                    }
                }

                x += count;
                i += 2;
            } else {
                // Add single pixel to buffer
                pixel_buffer.push(Pixel(
                    Point::new(x as i32 + offset_x, y as i32 + offset_y),
                    color,
                ));
                x += 1;
                i += 1;
            }
        }

        // Draw any remaining pixels in the buffer
        if !pixel_buffer.is_empty() {
            target.draw_iter(pixel_buffer.iter().cloned())?;
        }

        Ok(())
    }

    fn draw_sub_image<D>(
        &self,
        target: &mut D,
        _area: &embedded_graphics::primitives::Rectangle,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.draw(target)
    }
}
