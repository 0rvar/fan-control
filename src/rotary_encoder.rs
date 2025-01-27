use std::{sync::Arc, time::SystemTime};

use esp_idf_hal::{delay::Delay, gpio::InputPin, pcnt::Pcnt, peripheral::Peripheral};
use fan_control_graphics::InterfaceState;

pub fn rotary_encoder_thread<PCNT: Pcnt>(
    pcnt: impl Peripheral<P = PCNT>,
    clk: impl Peripheral<P = impl InputPin>,
    dt: impl Peripheral<P = impl InputPin>,
    state: Arc<InterfaceState>,
) {
    let encoder = encoder::Encoder::new(pcnt, clk, dt).unwrap();
    const TARGET_HZ: u32 = 30;
    const TARGET_PERIOD_US: u32 = 1_000_000 / TARGET_HZ;
    let delay = Delay::new(TARGET_PERIOD_US / 10);
    let mut last_value = encoder.get_value().unwrap();
    loop {
        let start = SystemTime::now();
        let value = encoder.get_value();

        match value {
            Ok(value) => {
                use std::sync::atomic::Ordering;
                let diff = value - last_value;
                if diff != 0 {
                    let pwm = state.fan_pwm.load(Ordering::Relaxed);
                    last_value = value;
                    let new_pwm = pwm as i32 + diff;
                    state
                        .fan_pwm
                        .store(new_pwm.max(0).min(100) as u32, Ordering::Relaxed);
                }
            }
            Err(e) => {
                log::error!("Error: {:?}", e);
                delay.delay_ms(1000);
            }
        }
        let elapsed_micros = start.elapsed().unwrap().as_micros();
        delay.delay_us(TARGET_PERIOD_US.saturating_sub(elapsed_micros as u32));
    }
}

// Shamelessly stolen from:
// https://github.com/esp-rs/esp-idf-hal/blob/518a6419a5d4f3577c972f67b01ac97e1085e434/examples/pcnt_rotary_encoder.rs#L55
mod encoder {
    use std::cmp::min;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use esp_idf_hal::gpio::AnyInputPin;
    use esp_idf_hal::gpio::InputPin;
    use esp_idf_hal::pcnt::*;
    use esp_idf_hal::peripheral::Peripheral;
    use esp_idf_hal::sys::EspError;

    const LOW_LIMIT: i16 = -100;
    const HIGH_LIMIT: i16 = 100;

    pub struct Encoder<'d> {
        unit: PcntDriver<'d>,
        approx_value: Arc<AtomicI32>,
    }

    impl<'d> Encoder<'d> {
        pub fn new<PCNT: Pcnt>(
            pcnt: impl Peripheral<P = PCNT> + 'd,
            pin_a: impl Peripheral<P = impl InputPin> + 'd,
            pin_b: impl Peripheral<P = impl InputPin> + 'd,
        ) -> Result<Self, EspError> {
            let mut unit = PcntDriver::new(
                pcnt,
                Some(pin_a),
                Some(pin_b),
                Option::<AnyInputPin>::None,
                Option::<AnyInputPin>::None,
            )?;
            unit.channel_config(
                PcntChannel::Channel0,
                PinIndex::Pin0,
                PinIndex::Pin1,
                &PcntChannelConfig {
                    lctrl_mode: PcntControlMode::Reverse,
                    hctrl_mode: PcntControlMode::Keep,
                    pos_mode: PcntCountMode::Decrement,
                    neg_mode: PcntCountMode::Increment,
                    counter_h_lim: HIGH_LIMIT,
                    counter_l_lim: LOW_LIMIT,
                },
            )?;
            unit.channel_config(
                PcntChannel::Channel1,
                PinIndex::Pin1,
                PinIndex::Pin0,
                &PcntChannelConfig {
                    lctrl_mode: PcntControlMode::Reverse,
                    hctrl_mode: PcntControlMode::Keep,
                    pos_mode: PcntCountMode::Increment,
                    neg_mode: PcntCountMode::Decrement,
                    counter_h_lim: HIGH_LIMIT,
                    counter_l_lim: LOW_LIMIT,
                },
            )?;

            unit.set_filter_value(min(10 * 80, 1023))?;
            unit.filter_enable()?;

            let approx_value = Arc::new(AtomicI32::new(0));
            // unsafe interrupt code to catch the upper and lower limits from the encoder
            // and track the overflow in `value: Arc<AtomicI32>` - I plan to use this for
            // a wheeled robot's odomerty
            unsafe {
                let approx_value = approx_value.clone();
                unit.subscribe(move |status| {
                    let status = PcntEventType::from_repr_truncated(status);
                    if status.contains(PcntEvent::HighLimit) {
                        approx_value.fetch_add(HIGH_LIMIT as i32, Ordering::SeqCst);
                    }
                    if status.contains(PcntEvent::LowLimit) {
                        approx_value.fetch_add(LOW_LIMIT as i32, Ordering::SeqCst);
                    }
                })?;
            }
            unit.event_enable(PcntEvent::HighLimit)?;
            unit.event_enable(PcntEvent::LowLimit)?;
            unit.counter_pause()?;
            unit.counter_clear()?;
            unit.counter_resume()?;

            Ok(Self { unit, approx_value })
        }

        pub fn get_value(&self) -> Result<i32, EspError> {
            let value =
                self.approx_value.load(Ordering::Relaxed) + self.unit.get_counter_value()? as i32;
            Ok(value)
        }
    }
}
