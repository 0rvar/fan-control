use std::collections::HashMap;

use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

pub struct LeekSpin {
    frame_index: u8,
}

impl LeekSpin {
    pub fn new() -> Self {
        Self { frame_index: 0 }
    }

    pub fn render<D>(&mut self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let compressed_data: &[u8] = match self.frame_index {
            0 => include_bytes!("./animations/leek_spin-0.rle"),
            1 => include_bytes!("./animations/leek_spin-1.rle"),
            2 => include_bytes!("./animations/leek_spin-2.rle"),
            3 => include_bytes!("./animations/leek_spin-3.rle"),
            _ => panic!("Invalid frame index"),
        };

        if let Some(image) = Rgb565Rle::new(compressed_data) {
            image.draw(target)?;
        }

        self.frame_index = (self.frame_index + 1) % 4;
        Ok(())
    }
}

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

    pub fn encode(width: u32, height: u32, pixels: &[Rgb565]) -> Vec<u8> {
        // First pass: build palette
        let (palette, palette_index_map) = build_palette(pixels, 64);

        let mut output = Vec::new();

        // Write header
        output.extend_from_slice(&width.to_le_bytes());
        output.extend_from_slice(&height.to_le_bytes());
        output.push(palette.len() as u8);

        // Write palette
        for color in &palette {
            output.extend_from_slice(&color.to_le_bytes());
        }

        // Encode pixels
        let mut i = 0;
        while i < pixels.len() {
            let current = pixels[i];
            let current_idx = *palette_index_map.get(&current).unwrap();
            let mut run_length = 1;

            // Calculate max possible run length to end of current row
            let max_run_to_row_end = (width - ((i as u32) % width)) as usize;

            // Count consecutive identical pixels, but stop at row boundary
            while i + run_length < pixels.len() 
                && run_length < max_run_to_row_end  // Stop at row end
                && pixels[i + run_length] == current
                && run_length < 255
            {
                run_length += 1;
            }

            if run_length > 1 {
                // RLE packet: [1|palette_index][count]
                output.push(0x80 | (current_idx as u8));
                output.push(run_length as u8);
            } else {
                // Single pixel: [0|palette_index]
                output.push(current_idx as u8);
            }

            i += run_length;
        }

        output
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

        while i < self.data.len() {
            if x >= self.width {
                x = 0;
                y += 1;
            }

            let packet = self.data[i];
            let is_rle = packet & 0x80 != 0;
            let palette_idx = (packet & 0x7F) as usize;

            if palette_idx >= self.palette.len() {
                break; // Invalid palette index
            }

            let color = self.palette[palette_idx];

            if is_rle {
                // RLE packet needs an extra byte for count
                if i + 1 >= self.data.len() {
                    break;
                }
                let count = self.data[i + 1] as u32;

                target.draw_iter(
                    (x..x + count)
                        .into_iter()
                        .map(|x| Pixel(Point::new(x as i32, y as i32), color)),
                )?;

                x += count;
                i += 2;
            } else {
                // Single pixel
                target.draw_iter(std::iter::once(Pixel(
                    Point::new(x as i32, y as i32),
                    color,
                )))?;
                x += 1;
                i += 1;
            }
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

fn build_palette(pixels: &[Rgb565], max_colors: usize) -> (Vec<Rgb565>, HashMap<Rgb565, usize>) {
    let rgba_pixels = pixels.iter().flat_map(|&color| {
        let (r, g, b) = rgb565_to_rgb888(color);
        vec![r, g, b, 255]
    }).collect::<Vec<_>>();
    let nq = color_quant::NeuQuant::new(10, max_colors, &rgba_pixels);
    let indixes: Vec<u8> = rgba_pixels.chunks(4).map(|pix| nq.index_of(pix) as u8).collect();
    let palette = nq.color_map_rgb().chunks_exact(3).map(|x| {
        rgb888_to_rgb565(x[0], x[1], x[2])
    }).collect::<Vec<_>>();
    let mut palette_index_map = HashMap::new();
    for (i, color) in pixels.iter().enumerate() {
        palette_index_map.insert(*color, indixes[i] as usize);
    }
    (palette, palette_index_map)
}

fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> Rgb565 {
    // Linear rescaling to maximize color accuracy
    let r5 = ((r as u16 * 31) / 255) as u8; // 8 bits -> 5 bits (0-31)
    let g6 = ((g as u16 * 63) / 255) as u8; // 8 bits -> 6 bits (0-63)
    let b5 = ((b as u16 * 31) / 255) as u8; // 8 bits -> 5 bits (0-31)

    Rgb565::new(r5, g6, b5)
}

fn rgb565_to_rgb888(color: Rgb565) -> (u8, u8, u8) {
    // Extract individual components
    let r5 = color.r(); // 5 bits (0-31)
    let g6 = color.g(); // 6 bits (0-63)
    let b5 = color.b(); // 5 bits (0-31)

    // Linear rescaling back to 8 bits
    let r8 = ((r5 as u16 * 255) / 31) as u8;  // 5 bits -> 8 bits (0-255)
    let g8 = ((g6 as u16 * 255) / 63) as u8;  // 6 bits -> 8 bits (0-255)
    let b8 = ((b5 as u16 * 255) / 31) as u8;  // 5 bits -> 8 bits (0-255)

    (r8, g8, b8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay};
    use image::GenericImageView;
    use std::fs;

    #[test]
    fn convert_pngs_to_rle() {
        for i in 0..4 {
            let png_path = format!("src/animations/leek_spin-{}.png", i);
            let rle_path = format!("src/animations/leek_spin-{}.rle", i);
            let roundtrip_path = format!("src/animations/leek_spin-{}-roundtrip.png", i);

            // Read original PNG and convert to RGB565 pixels
            let img = image::open(&png_path).expect("Failed to read PNG file");
            let (width, height) = img.dimensions();

            // Convert image to RGB565 pixels
            let pixels: Vec<Rgb565> = img
                .pixels()
                .map(|(_, _, pixel)| rgb888_to_rgb565(pixel[0], pixel[1], pixel[2]))
                .collect();

            // Create RLE data
            let rle_data = Rgb565Rle::encode(width, height, &pixels);
            fs::write(&rle_path, &rle_data).expect("Failed to write RLE file");

            // Create roundtrip image
            let mut display = SimulatorDisplay::<Rgb565>::new(Size::new(width, height));
            if let Some(rle_image) = Rgb565Rle::new(&rle_data) {
                rle_image
                    .draw(&mut display)
                    .expect("Failed to draw RLE image");
            }

            // Save the display buffer as PNG
            let output_settings = OutputSettingsBuilder::new().scale(1).build();
            display
                .to_rgb_output_image(&output_settings)
                .save_png(&roundtrip_path)
                .expect("Failed to save roundtrip PNG");

            // Print compression stats and palette size
            println!(
                "Frame {}: Original size: {}KB, RLE size: {}KB, Palette: {} colors",
                i,
                fs::metadata(&png_path).unwrap().len() / 1024,
                rle_data.len() / 1024,
                rle_data[8] // palette size is at index 8
            );
        }
    }
}
