use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

use config::MODE_3;
use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::*;
use esp_idf_hal::units::FromValueType;
use fan_control_graphics::InterfaceState;
use mipidsi::interface::SpiInterface;
use mipidsi::Builder;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;

    // For the ST7789 display:
    // DC (Data/Command) - Any digital GPIO pin
    // RST (Reset) - Any digital GPIO pin
    // MOSI (Master Out Slave In) - GPIO 23 (for SPI2/HSPI) or GPIO 13 (for SPI1/VSPI)
    // SCK (Clock) - GPIO 18 (for SPI2/HSPI) or GPIO 14 (for SPI1/VSPI)
    // CS (Chip Select) - Any digital GPIO pin, typically GPIO 5
    // VCC - 3.3V
    // GND - Ground

    // We use these pins:
    // * GPIO18 for SCL(K)
    // * GPIO23 for SDA
    // this means we use SPI2
    let spi = peripherals.spi2;

    // For the remaining ST7789 pins, we use the following pins:
    // * GPIO4 for RES
    // * GPIO2 for DC
    // * BLK is not connected
    let rst = PinDriver::output(peripherals.pins.gpio4)?;
    let dc = PinDriver::output(peripherals.pins.gpio2)?;
    let sclk = peripherals.pins.gpio18;
    let sda = peripherals.pins.gpio23;

    let mut delay = Ets;

    // configuring the spi interface, note that in order for the ST7789 to work, the data_mode needs to be set to MODE_3
    let config = config::Config::new()
        .baudrate(50.MHz().into())
        .data_mode(MODE_3)
        .write_only(true);
    let driver_config = SpiDriverConfig::new().dma(Dma::Channel1(8192)); // Try 8KB

    let device = SpiDeviceDriver::new_single(
        spi,
        sclk,
        sda,
        None::<Gpio12>, // Explicitly specify the pin type for SDI/MISO
        None::<Gpio5>,  // Explicitly specify the pin type for CS
        &driver_config,
        &config,
    )?;

    // display interface abstraction from SPI and DC
    let mut buffer = [0_u8; 2048];
    let di = SpiInterface::new(device, dc, &mut buffer);

    // create driver
    // let mut display = ST7789::new(di, Some(rst), None, 240, 240);
    // display.init(&mut delay)?;
    let mut display = Builder::new(mipidsi::models::ST7789, di)
        .reset_pin(rst)
        .display_size(240, 240)
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        // .orientation(Orientation::default().rotate(mipidsi::options::Rotation::Deg90))
        .init(&mut delay)
        .unwrap();

    // turn on the backlight
    // backlight.set_high()?;
    let raw_image_data = ImageRawLE::new(include_bytes!("./ferris.raw"), 86);
    let ferris = Image::with_center(&raw_image_data, Point::new(240 / 2, 240 / 2));

    // draw image on black background
    display.clear(Rgb565::BLACK).unwrap();
    ferris.draw(&mut display).unwrap();

    let state = Arc::new(InterfaceState {
        fan_pwm: AtomicU32::new(0),
        fan_rpm: AtomicU32::new(0),
        target_rpm: AtomicU32::new(0),
    });
    let mut interface = fan_control_graphics::Interface::new(state.clone());

    thread::spawn(move || {
        fake_interaction(state);
    });

    let mut led_toggle = false;
    let mut led = PinDriver::output(peripherals.pins.gpio22)?;
    display.clear(Rgb565::WHITE).unwrap();
    let start = SystemTime::now();
    loop {
        let clock_ms = start.elapsed().unwrap_or_default().as_millis() as u32;
        interface.render(&mut display, clock_ms).unwrap();

        if led_toggle {
            led_toggle = false;
            led.set_high()?;
        } else {
            led_toggle = true;
            led.set_low()?;
        }
        FreeRtos::delay_ms(10);
    }
}

fn fake_interaction(state: Arc<InterfaceState>) {
    use std::sync::atomic::Ordering;

    let start = SystemTime::now();
    let mut last_iteration = SystemTime::now();

    loop {
        let clock_ms = start.elapsed().unwrap_or_default().as_millis() as u32;
        let delta_ms = last_iteration.elapsed().unwrap_or_default().as_millis() as u32;
        last_iteration = SystemTime::now();

        let clock_s = clock_ms as f32 / 1000.0;
        let delta_s = delta_ms as f32 / 1000.0;

        // Define fan speed presets (RPM)
        const SPEED_PRESETS: [u32; 3] = [800, 1400, 2100];

        // Time between target changes (seconds)
        const TARGET_CHANGE_INTERVAL: f32 = 6.0;

        // Update target RPM occasionally
        let preset_index = ((clock_s / TARGET_CHANGE_INTERVAL) as usize) % SPEED_PRESETS.len();
        let target_rpm = SPEED_PRESETS[preset_index];
        state.target_rpm.store(target_rpm, Ordering::Relaxed);

        // Get current RPM
        let current_rpm = state.fan_rpm.load(Ordering::Relaxed) as f32;
        let target_rpm = target_rpm as f32;

        // Calculate base PWM for target RPM (assume linear relationship)
        // Maximum RPM (2400) should correspond to PWM 100%
        const MAX_RPM: f32 = 2400.0;
        let target_pwm = (target_rpm / MAX_RPM * 100.0).clamp(20.0, 100.0);

        // Get current PWM
        let current_pwm = state.fan_pwm.load(Ordering::Relaxed) as f32;

        // Very gentle PWM adjustment (no overshooting)
        let pwm_diff = target_pwm - current_pwm;
        let new_pwm = (current_pwm + pwm_diff * delta_s * 2.0).clamp(0.0, 100.0);
        state
            .fan_pwm
            .store(new_pwm.round() as u32, Ordering::Relaxed);

        // Simulate fan physics
        // We want to cover the full RPM range (2400) in 2 seconds
        // So rate should be 1200 RPM/second or 0.1 RPM/ms
        const RPM_PER_MS: f32 = 0.6; // 600 RPM per second = full range in 2 seconds

        let rpm_error = target_rpm - current_rpm;
        let max_rpm_change = RPM_PER_MS * delta_ms as f32;
        let rpm_change = rpm_error.clamp(-max_rpm_change, max_rpm_change);

        // Add very subtle random variation (±2 RPM maximum)
        let jitter = (clock_s * 2.0).sin() * 1.0;

        let new_rpm = (current_rpm + rpm_change + jitter).round() as u32;
        state.fan_rpm.store(new_rpm, Ordering::Relaxed);
    }
}
