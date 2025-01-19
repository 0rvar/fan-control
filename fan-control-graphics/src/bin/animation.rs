use std::f32::consts::E;

use embedded_graphics::{
    draw_target::DrawTarget,
    image::ImageRawLE,
    mono_font::{iso_8859_1::FONT_6X9, MonoTextStyle},
    pixelcolor::{BinaryColor, Rgb565},
    prelude::{Point, RgbColor, Size},
    text::Text,
    Drawable,
};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use embedded_hal::delay::DelayNs;
use fan_control_graphics::LeekSpin;

fn main() {
    let mut display = SimulatorDisplay::<Rgb565>::new(Size::new(240, 240));
    let output_settings = OutputSettingsBuilder::new().build();
    let mut window = Window::new("Hello World", &output_settings);
    display.clear(Rgb565::BLACK).unwrap();
    window.update(&mut display);
    let mut animation = LeekSpin::new();
    loop {
        let before = std::time::Instant::now();
        display.clear(Rgb565::BLACK).unwrap();
        animation.render(&mut display);
        window.update(&mut display);

        if window.events().any(|e| e == SimulatorEvent::Quit) {
            return;
        }
        let elapsed = before.elapsed().as_millis();
        let delay = 100u64.saturating_sub(elapsed as u64);
        std::thread::sleep(std::time::Duration::from_millis(delay));
    }
    // let text_style = MonoTextStyle::new(&FONT_6X9, Rgb565::WHITE);
    // Text::new("Hello World!", Point::new(5, 5), text_style)
    //     .draw(&mut display)
    //     .unwrap();

    // run_animation(&mut display, &mut SimpleDelay).unwrap();

    // let output_settings = OutputSettingsBuilder::new().build();
    // Window::new("Hello World", &output_settings).show_static(&display);
}

struct SimpleDelay;
impl DelayNs for SimpleDelay {
    fn delay_ns(&mut self, ns: u32) {
        std::thread::sleep(std::time::Duration::from_nanos(ns as u64));
    }
}
