use std::sync::Arc;

use anyhow::Context;
use esp_idf_hal::peripherals::Peripherals;
use fan_control_graphics::InterfaceState;
use screen::ScreenBuilder;
use threads::EspThread;

mod pwm;
mod rotary_encoder;
mod screen;
mod tacho;
mod threads;
mod wifi_control;

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

    let state = Arc::new(InterfaceState::with_initial_pwm(50));
    let interface = fan_control_graphics::Interface::new(state.clone());

    let dt = peripherals.pins.gpio33;
    let clk = peripherals.pins.gpio32;
    let pcnt = peripherals.pcnt0;
    let state_clone = state.clone();
    let rotary_encoder_thread = EspThread::new("rotary_encoder::rotary_encoder_thread")
        .spawn(move || rotary_encoder::rotary_encoder_thread(pcnt, clk, dt, state_clone));

    let pcnt = peripherals.pcnt1;
    let pin = peripherals.pins.gpio27;
    let tacho = tacho::Tacho::new(pcnt, pin).context("Failed to initialize tacho")?;
    let state_clone = state.clone();
    let tacho_thread =
        EspThread::new("tacho::tacho_thread").spawn(move || tacho::tacho_loop(state_clone, tacho));

    let ledc = peripherals.ledc;
    let pwm = pwm::PwmControl::new(ledc.timer0, ledc.channel0, peripherals.pins.gpio26)
        .context("Failed to initialize PWM control")?;
    let state_clone = state.clone();
    let pwm_thread = EspThread::new("pwm::pwm_control_thread")
        .spawn(move || pwm::pwm_control_thread(pwm, state_clone));

    log::info!("Spawning render thread");
    let render_thread = EspThread::new("screen::render_loop")
        .with_stack_size(16)
        .spawn(move || screen::render_loop(interface, screen));

    let wifi_thread = wifi_control::spawn_wifi_control_thread(state, peripherals.modem);

    wifi_thread.join().unwrap();
    render_thread.join().unwrap();
    rotary_encoder_thread.join().unwrap();
    pwm_thread.join().unwrap();
    tacho_thread.join().unwrap();
    Ok(())
}
