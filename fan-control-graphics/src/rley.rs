use embedded_graphics::{
    image::ImageDrawable,
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::{DrawTarget, OriginDimensions, Point, Size},
    primitives::Rectangle,
};

#[derive(Debug)]
pub struct Rgb565Rle<'a> {
    width: u32,
    height: u32,
    palette: Vec<Rgb565>,
    data: &'a [u8],
    y_range: Option<(u32, u32)>,
}
impl Rgb565Rle<'_> {
    pub fn limit(mut self, y_range: (u32, u32)) -> Self {
        if y_range.0 < y_range.1 {
            self.y_range = Some(y_range);
        }
        self
    }
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
            y_range: None,
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

        // Pre-allocate a single large buffer for the entire frame
        let buffer_rows = 64usize;
        let buffer_limit = self.width as usize * buffer_rows;
        let mut buffer_row_start = 0;
        let mut pixel_buffer = Vec::with_capacity(buffer_limit);

        while i < self.data.len() {
            if x >= self.width {
                x = 0;
                y += 1;
            }
            if y >= self.y_range.map_or(self.height, |range| range.1) {
                break;
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

                // Add RLE pixels in bulk
                if y >= self.y_range.map_or(0, |range| range.0) {
                    if pixel_buffer.len() < 1 {
                        buffer_row_start = y;
                    }
                    pixel_buffer.extend((0..count).map(|_| color));
                }

                x += count;
                i += 2;
            } else {
                if y >= self.y_range.map_or(0, |range| range.0) {
                    if pixel_buffer.len() < 1 {
                        buffer_row_start = y;
                    }
                    pixel_buffer.push(color);
                }
                x += 1;
                i += 1;
            }

            // Draw in larger batches instead of tiny ones
            if pixel_buffer.len() >= buffer_limit {
                target.fill_contiguous(
                    &Rectangle::new(
                        Point::new(0, buffer_row_start as i32),
                        Size::new(self.width, buffer_rows as u32),
                    ),
                    pixel_buffer.iter().cloned(),
                )?;
                pixel_buffer.clear();
            }
        }

        // Draw any remaining pixels
        if !pixel_buffer.is_empty() {
            target.fill_contiguous(
                &Rectangle::new(
                    Point::new(0, buffer_row_start as i32),
                    Size::new(
                        self.width,
                        (buffer_rows as u32).min(self.height - buffer_row_start as u32),
                    ),
                ),
                pixel_buffer.iter().cloned(),
            )?;
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
