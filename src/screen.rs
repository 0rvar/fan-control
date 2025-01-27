use std::time::SystemTime;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_hal::spi::MODE_3;
use esp_idf_hal::delay::{Ets, FreeRtos};
use esp_idf_hal::gpio::*;
use esp_idf_hal::spi::{Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SPI2};
use esp_idf_hal::units::FromValueType;
use fan_control_graphics::Interface;
use mipidsi::interface::SpiInterface;

use crate::threads::debug_dump_stack_info;

pub struct ScreenBuilder {
    pub spi: SPI2,
    pub rst: Gpio4,
    pub dc: Gpio2,
    pub sclk: Gpio18,
    pub sda: Gpio23,
}
impl ScreenBuilder {
    pub fn build(self) -> anyhow::Result<Screen> {
        let Self {
            spi,
            rst,
            dc,
            sclk,
            sda,
        } = self;
        // For the ST7789 display:
        // DC (Data/Command) - Any digital GPIO pin
        // RST (Reset) - Any digital GPIO pin
        // MOSI (Master Out Slave In) - GPIO 23 (for SPI2/HSPI) or GPIO 13 (for SPI1/VSPI)
        // SCK (Clock) - GPIO 18 (for SPI2/HSPI) or GPIO 14 (for SPI1/VSPI)
        // CS (Chip Select) - Any digital GPIO pin, typically GPIO 5
        // VCC - 3.3V
        // GND - Ground

        // For the remaining ST7789 pins, we use the following pins:
        // * GPIO4 for RES
        // * GPIO2 for DC
        // * BLK is not connected
        let rst = PinDriver::output(rst)?;
        let dc = PinDriver::output(dc)?;

        // configuring the spi interface, note that in order for the ST7789 to work, the data_mode needs to be set to MODE_3
        let config = esp_idf_hal::spi::config::Config::new()
            .baudrate(60.MHz().into()) // Absolute max before the display won't be driven anymore
            .data_mode(MODE_3)
            .write_only(true);
        let driver_config = SpiDriverConfig::new().dma(Dma::Channel1(1024 * 32));

        let device = SpiDeviceDriver::new_single(
            spi,
            sclk,
            sda,
            None::<Gpio12>, // Explicitly specify the pin type for SDI/MISO
            None::<Gpio5>,  // Explicitly specify the pin type for CS
            &driver_config,
            &config,
        )?;

        Ok(Screen { device, dc, rst })
    }
}
pub struct Screen {
    device: SpiDeviceDriver<'static, SpiDriver<'static>>,
    dc: PinDriver<'static, Gpio2, Output>,
    rst: PinDriver<'static, Gpio4, Output>,
}

pub fn render_loop<'a>(mut interface: Interface, screen: Screen) {
    debug_dump_stack_info();

    let Screen { device, dc, rst } = screen;

    log::info!("Creating screen buffer");
    let mut buffer = [0_u8; 2048];

    log::info!("Initializing display SPI");
    let di = SpiInterface::new(device, dc, &mut buffer);

    log::info!("Initializing mipidsi display");
    let mut display = mipidsi::Builder::new(mipidsi::models::ST7789, di)
        .reset_pin(rst)
        .display_size(240, 240)
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        // .orientation(Orientation::default().rotate(mipidsi::options::Rotation::Deg90))
        .init(&mut Ets)
        .unwrap();

    log::info!("Allocating timings vector");
    let mut timings = Vec::with_capacity(100);

    log::info!("Clearing display and starting render loop");
    display.clear(Rgb565::BLACK).unwrap();
    let start = SystemTime::now();
    display.clear(Rgb565::WHITE).unwrap();
    interface.render(&mut display, 0).unwrap();
    loop {
        let before = SystemTime::now();
        let clock_ms = start.elapsed().unwrap_or_default().as_millis() as u32;
        interface.render(&mut display, clock_ms).unwrap();

        let elapsed_ms = before.elapsed().unwrap_or_default().as_millis();
        timings.push(elapsed_ms);
        if timings.len() >= 100 {
            timings.sort();
            let sum: u128 = timings.iter().sum();
            let avg = sum / timings.len() as u128;
            let min = timings[0];
            let max = timings[timings.len() - 1];
            let p50 = timings[timings.len() / 2];
            let p90 = timings[(timings.len() as f32 * 0.9) as usize];
            let p99 = timings[(timings.len() as f32 * 0.99) as usize];
            log::info!("Average render timings:\n * min: {min}ms\n * max: {max}ms\n * avg: {avg}ms\n * p50: {p50}ms\n * p90: {p90}ms\n * p99: {p99}ms");
            timings.clear();
        }

        FreeRtos::delay_ms(10);
    }
}
