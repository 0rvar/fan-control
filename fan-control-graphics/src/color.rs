use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};

pub fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> Rgb565 {
    // Linear rescaling to maximize color accuracy
    let r5 = ((r as u16 * 31) / 255) as u8; // 8 bits -> 5 bits (0-31)
    let g6 = ((g as u16 * 63) / 255) as u8; // 8 bits -> 6 bits (0-63)
    let b5 = ((b as u16 * 31) / 255) as u8; // 8 bits -> 5 bits (0-31)

    Rgb565::new(r5, g6, b5)
}

pub fn rgb565_to_rgb888(color: Rgb565) -> (u8, u8, u8) {
    // Extract individual components
    let r5 = color.r(); // 5 bits (0-31)
    let g6 = color.g(); // 6 bits (0-63)
    let b5 = color.b(); // 5 bits (0-31)

    // Linear rescaling back to 8 bits
    let r8 = ((r5 as u16 * 255) / 31) as u8; // 5 bits -> 8 bits (0-255)
    let g8 = ((g6 as u16 * 255) / 63) as u8; // 6 bits -> 8 bits (0-255)
    let b8 = ((b5 as u16 * 255) / 31) as u8; // 5 bits -> 8 bits (0-255)

    (r8, g8, b8)
}
