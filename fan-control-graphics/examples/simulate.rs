use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb565,
    prelude::{RgbColor, Size},
};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
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
        println!("Rendering frame");
        animation.render(&mut display).unwrap();
        window.update(&mut display);

        if window.events().any(|e| e == SimulatorEvent::Quit) {
            return;
        }
        let elapsed = before.elapsed().as_millis();
        let delay = 100u64.saturating_sub(elapsed as u64).max(1);
        std::thread::sleep(std::time::Duration::from_millis(delay));
    }
}
