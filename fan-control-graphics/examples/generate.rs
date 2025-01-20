use std::{collections::HashMap, fs};

use embedded_graphics::{image::ImageDrawable, pixelcolor::{raw::ToBytes, Rgb565}, prelude::Size};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay};
use fan_control_graphics::{color::{rgb565_to_rgb888, rgb888_to_rgb565}, rley::Rgb565Rle};
use image::GenericImageView;

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

fn build_palette(pixels: &[Rgb565], max_colors: usize) -> (Vec<Rgb565>, HashMap<Rgb565, usize>) {
  let rgba_pixels = pixels
      .iter()
      .flat_map(|&color| {
          let (r, g, b) = rgb565_to_rgb888(color);
          vec![r, g, b, 255]
      })
      .collect::<Vec<_>>();
  let nq = color_quant::NeuQuant::new(10, max_colors, &rgba_pixels);
  let indixes: Vec<u8> = rgba_pixels
      .chunks(4)
      .map(|pix| nq.index_of(pix) as u8)
      .collect();
  let palette = nq
      .color_map_rgb()
      .chunks_exact(3)
      .map(|x| rgb888_to_rgb565(x[0], x[1], x[2]))
      .collect::<Vec<_>>();
  let mut palette_index_map = HashMap::new();
  for (i, color) in pixels.iter().enumerate() {
      palette_index_map.insert(*color, indixes[i] as usize);
  }
  (palette, palette_index_map)
}

fn main() {
    // Step 1: generate pngs from gif
    std::process::Command::new("convert")
        .args(&[
            "leek_spin.gif",
            "-coalesce",
            "-gravity", "center",
            "-crop",  "240x240-40+0",
            "-alpha",
            "remove",
            "-alpha",
            "off",
            "leek_spin-%d.png",
        ])
        .current_dir("src/animations")
        .output()
        .expect("Failed to convert GIF to PNG");
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
        let rle_data = encode(width, height, &pixels);
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
