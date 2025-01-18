use config::MODE_3;
use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::raw::RawU16;
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

// use mipidsi::{Builder, Orientation};

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
        .baudrate(26.MHz().into())
        .data_mode(MODE_3);

    let device = SpiDeviceDriver::new_single(
        spi,
        sclk,
        sda,
        None::<Gpio12>, // Explicitly specify the pin type for SDI/MISO
        None::<Gpio5>,  // Explicitly specify the pin type for CS
        &SpiDriverConfig::new(),
        &config,
    )?;

    // display interface abstraction from SPI and DC
    let mut buffer = [0_u8; 512];
    let di = SpiInterface::new(device, dc, &mut buffer);

    // create driver
    // let mut display = ST7789::new(di, Some(rst), None, 240, 240);
    // display.init(&mut delay)?;
    let mut display = Builder::new(mipidsi::models::ST7789, di)
        .reset_pin(rst)
        .display_size(240, 240)
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

    log::info!("Parsing gif");
    let gif_data = include_bytes!("./leek-spin.gif");

    loop {
        let frame_iterator = GifFrameIterator::new(gif_data)?;

        // Process one frame at a time
        for frame_result in frame_iterator {
            let frame = frame_result?;
            let raw_image_data = ImageRawLE::new(&frame.data, frame.width);
            let image = Image::with_center(&raw_image_data, Point::new(240 / 2, 240 / 2));

            display.clear(Rgb565::BLACK).unwrap();
            image.draw(&mut display).unwrap();
            FreeRtos::delay_ms(frame.delay_ms);
        }
    }

    log::info!("Image printed!");

    let mut led = PinDriver::output(peripherals.pins.gpio22)?;

    loop {
        log::info!("Blinking LED");
        led.set_high()?;
        // we are sleeping here to make sure the watchdog isn't triggered
        FreeRtos::delay_ms(1000);

        led.set_low()?;
        FreeRtos::delay_ms(1000);
    }
}

struct GifFrame565 {
    width: u32,
    height: u32,
    data: Vec<u8>,
    delay_ms: u32,
}

struct GifFrameIterator<'a> {
    decoder: gif::Decoder<std::io::Cursor<&'a [u8]>>,
    rgba_buffer: Vec<u8>,   // Buffer for RGBA data
    rgb565_buffer: Vec<u8>, // Buffer for RGB565 data
}

impl<'a> GifFrameIterator<'a> {
    fn new(gif_data: &'a [u8]) -> Result<Self, gif::DecodingError> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let reader = std::io::Cursor::new(gif_data);
        let decoder = options.read_info(reader)?;

        // Calculate buffer sizes
        let width = decoder.width() as usize;
        let height = decoder.height() as usize;

        // Allocate minimum required buffers
        // We'll process the image in rows to save memory
        let rgba_buffer = vec![0; width * 4]; // One row of RGBA data
        let rgb565_buffer = vec![0; width * height * 2]; // Full frame of RGB565

        log::info!(
            "Allocated buffers - RGBA row: {} bytes, RGB565 frame: {} bytes",
            rgba_buffer.len(),
            rgb565_buffer.len()
        );

        Ok(Self {
            decoder,
            rgba_buffer,
            rgb565_buffer,
        })
    }

    fn convert_rgba_to_rgb565(rgba: &[u8], rgb565: &mut [u8], pixels: usize) {
        for i in 0..pixels {
            let r = rgba[i * 4];
            let g = rgba[i * 4 + 1];
            let b = rgba[i * 4 + 2];

            let rgb = Rgb565::new(r, g, b);
            let components: RawU16 = rgb.into();
            let bytes = components.to_le_bytes();

            rgb565[i * 2] = bytes[0];
            rgb565[i * 2 + 1] = bytes[1];
        }
    }
}

impl<'a> Iterator for GifFrameIterator<'a> {
    type Item = Result<GifFrame565, gif::DecodingError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.decoder.read_next_frame() {
            Ok(Some(frame)) => {
                let width = frame.width as usize;
                let height = frame.height as usize;

                // Process one row at a time
                for y in 0..height {
                    // Copy one row of RGBA data
                    let start = y * width * 4;
                    let end = start + width * 4;
                    self.rgba_buffer[..width * 4].copy_from_slice(&frame.buffer[start..end]);

                    // Convert this row to RGB565
                    let rgb565_start = y * width * 2;
                    let rgb565_end = rgb565_start + width * 2;
                    Self::convert_rgba_to_rgb565(
                        &self.rgba_buffer[..width * 4],
                        &mut self.rgb565_buffer[rgb565_start..rgb565_end],
                        width,
                    );
                }

                Some(Ok(GifFrame565 {
                    width: width as u32,
                    height: height as u32,
                    data: self.rgb565_buffer.clone(),
                    delay_ms: frame.delay as u32 * 10,
                }))
            }
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}
