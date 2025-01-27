use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use anyhow::Context;
use esp_idf_hal::peripherals::Peripherals;
use fan_control_graphics::InterfaceState;
use screen::ScreenBuilder;
use threads::EspThread;

mod fake_interaction;
mod screen;
mod threads;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;

    let screen = ScreenBuilder {
        spi: peripherals.spi2,
        dc: peripherals.pins.gpio2,
        rst: peripherals.pins.gpio4,
        // SPI2 means we use GPIO 18 for SCLK and GPIO 23 for MOSI
        sclk: peripherals.pins.gpio18,
        sda: peripherals.pins.gpio23,
    }
    .build()
    .context("Failed to initialize screen")?;

    let state = Arc::new(InterfaceState {
        fan_rpm: AtomicU32::new(0),
        fan_pwm: AtomicU32::new(0),
        target_rpm: AtomicU32::new(0),
    });
    let interface = fan_control_graphics::Interface::new(state.clone());

    log::info!("Spawning fake interaction thread");
    let interaction_thread = EspThread::new("fake_interaction::fake_interaction_loop")
        .spawn(move || fake_interaction::fake_interaction_loop(state));

    log::info!("Spawning render thread");
    let render_thread = EspThread::new("screen::render_loop")
        .with_stack_size(16)
        // .pin_to_core(Core::Core1)
        .with_priority(15)
        .spawn(move || screen::render_loop(interface, screen));

    interaction_thread.join().unwrap();
    render_thread.join().unwrap();
    Ok(())
}
