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

impl PwmControl {
    pub fn new(ledc: LEDC, pin: impl Peripheral<P = impl OutputPin> + 'static) -> Result<Self> {
        // Configure timer for 25kHz operation (standard for PC fans)
        let timer_config = config::TimerConfig::new()
            .frequency(25.kHz().into())
            .resolution(Resolution::Bits10);

        let timer = LedcTimerDriver::new(ledc.timer0, &timer_config)?;

        // Configure channel
        let channel = LedcDriver::new(ledc.channel0, timer, pin)?;

        Ok(Self { channel })
    }

    pub fn set_duty(&mut self, percent: u32) -> Result<()> {
        // Convert percentage (0-100) to duty cycle value (0-1023 for 10-bit resolution)
        let duty = ((percent as u32) * 1023) / 100;
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
