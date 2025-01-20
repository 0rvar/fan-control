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

    log::info!("Starting animation");

    let mut animation = fan_control_graphics::LeekSpin::new();
    let mut led_toggle = false;
    let mut led = PinDriver::output(peripherals.pins.gpio22)?;
    display.clear(Rgb565::WHITE).unwrap();
    loop {
        animation.render(&mut display).unwrap();

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
