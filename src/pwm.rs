use anyhow::Result;
use esp_idf_hal::gpio::*;
use esp_idf_hal::ledc::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::units::FromValueType;
use fan_control_graphics::InterfaceState;
use std::sync::Arc;

pub struct PwmControl {
    channel: LedcDriver<'static>,
}

/// PWM resolution used for fan control.
///
/// Using 10-bit resolution (2^10 - 1 = 1023) gives us:
/// - Good granularity for fan speed control (1024 steps)
/// - Compatible with the standard 25kHz frequency for PC fans
/// - Duty cycle converts from 0-100% to 0-1023:
///   - 0%   = 0 (constant LOW, fan at full speed)
///   - 100% = 1023 (constant HIGH, fan stopped)
const RESOLUTION: Resolution = Resolution::Bits10;

impl PwmControl {
    pub fn new(
        timer: TIMER0,
        channel: CHANNEL0,
        pin: impl Peripheral<P = impl OutputPin> + 'static,
    ) -> Result<Self> {
        // Configure timer for 25kHz operation (standard for PC fans)
        let timer_config = config::TimerConfig::new()
            .frequency(25.kHz().into())
            .resolution(RESOLUTION);

        let timer = LedcTimerDriver::new(timer, &timer_config)?;

        // Configure channel
        let channel = LedcDriver::new(channel, timer, pin)?;

        Ok(Self { channel })
    }

    pub fn set_duty(&mut self, percent: u32) -> Result<()> {
        const RESOLUTION_MAX_VALUE: u32 = 2u32.pow(RESOLUTION.bits() as u32) - 1;
        // Invert percentage to match fan speed (0% = full speed, 100% = stopped)
        // When we pull pin high, it uses a transistor to pull the pwm line down
        let percent = (100 - percent).clamp(0, 100);
        // Convert percentage (0-100) to duty cycle value (0-1023 for 10-bit resolution)
        let duty = ((percent as u32) * RESOLUTION_MAX_VALUE) / 100;
        self.channel.set_duty(duty)?;
        Ok(())
    }
}

pub fn pwm_control_thread(mut pwm: PwmControl, state: Arc<InterfaceState>) {
    use std::sync::atomic::Ordering;

    let mut last_pwm_value = 0;
    loop {
        let pwm_value = state.fan_pwm.load(Ordering::Relaxed);
        if pwm_value != last_pwm_value {
            last_pwm_value = pwm_value;
            if let Err(e) = pwm.set_duty(pwm_value) {
                log::error!("Failed to set PWM duty cycle: {:?}", e);
            }
        }
        // Small delay to avoid hammering the PWM
        esp_idf_hal::delay::FreeRtos::delay_ms(100);
    }
}
