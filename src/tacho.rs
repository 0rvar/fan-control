use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::Context;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyInputPin;
use esp_idf_hal::{
    gpio::InputPin,
    pcnt::{
        Pcnt, PcntChannel, PcntChannelConfig, PcntControlMode, PcntCountMode, PcntDriver, PinIndex,
    },
    peripheral::Peripheral,
};
use fan_control_graphics::InterfaceState;

pub struct Tacho {
    pcnt_driver: PcntDriver<'static>, // Store the driver
}

impl Tacho {
    pub fn new(
        pcnt: impl Peripheral<P = impl Pcnt> + 'static,
        pin: impl Peripheral<P = impl InputPin> + 'static,
    ) -> anyhow::Result<Self> {
        let mut pcnt_driver: PcntDriver<'static> = PcntDriver::new(
            pcnt,
            Some(pin),
            Option::<AnyInputPin>::None,
            Option::<AnyInputPin>::None,
            Option::<AnyInputPin>::None,
        )?;

        const EXPECTED_PULSES_PER_SEC: i16 = 160; // For 2400 RPM
        pcnt_driver.channel_config(
            PcntChannel::Channel0,
            PinIndex::Pin0,
            PinIndex::Pin1,
            &PcntChannelConfig {
                lctrl_mode: PcntControlMode::Keep,
                hctrl_mode: PcntControlMode::Keep,
                pos_mode: PcntCountMode::Increment,
                neg_mode: PcntCountMode::Increment,
                counter_h_lim: EXPECTED_PULSES_PER_SEC * 2, // Allow up to 2x expected speed
                counter_l_lim: 0,
            },
        )?;

        pcnt_driver.set_filter_value(100)?; // Filter pulses shorter than 100 clock cycles
        pcnt_driver.filter_enable()?;

        Ok(Self { pcnt_driver })
    }

    pub fn read_rpm(&mut self) -> anyhow::Result<u32> {
        self.pcnt_driver.counter_pause()?;
        self.pcnt_driver.counter_clear()?;
        let start = SystemTime::now();
        self.pcnt_driver.counter_resume()?;

        const APPROX_SAMPLE_TIME_MS: u32 = 1000;
        // Sample for 1 second
        FreeRtos::delay_ms(APPROX_SAMPLE_TIME_MS);

        self.pcnt_driver.counter_pause()?;
        let elapsed = start
            .elapsed()
            .context("Can't get elapsed time")?
            .as_millis() as u32;
        let count = self.pcnt_driver.get_counter_value()?;

        // Convert pulses per second to RPM
        let rpm = ((count as f32 / 4.0) / elapsed as f32) * 60_000.0;
        Ok(rpm as u32)
    }
}

pub fn tacho_loop(state: Arc<InterfaceState>, mut tacho: Tacho) {
    loop {
        if let Ok(rpm) = tacho.read_rpm() {
            state.fan_rpm.store(rpm, Ordering::Relaxed);
        }

        // Optional small delay between readings
        FreeRtos::delay_ms(100);
    }
}
