use std::sync::{atomic::AtomicU32, Arc};

use animations::leek_spin::LeekSpin;
use color::rgb888_to_rgb565;
use embedded_graphics::{
    mono_font::{iso_8859_1 as font, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
    Drawable,
};

pub mod animations;
pub mod color;
pub mod rley;

pub struct InterfaceState {
    pub fan_rpm: AtomicU32,
    pub fan_pwm: AtomicU32,
    pub target_rpm: AtomicU32,
    pub control_mode: AtomicU32,
}
pub struct Interface {
    state: Arc<InterfaceState>,
    animation: LeekSpin,
}

impl Interface {
    pub fn new(state: Arc<InterfaceState>) -> Self {
        Self {
            state,
            animation: LeekSpin::new(),
        }
    }

    pub fn render<D>(&mut self, target: &mut D, delta_time_ms: u32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        self.animation.render(target, delta_time_ms)?;

        let top_bg = rgb888_to_rgb565(255u8, 182u8, 140u8);
        {
            let rpm_label = format!(
                "{} RPM",
                self.state
                    .fan_rpm
                    .load(std::sync::atomic::Ordering::Relaxed)
            );
            let mut text_style = MonoTextStyle::new(&font::FONT_9X15_BOLD, Rgb565::BLACK);

            text_style.background_color = Some(top_bg);
            Text::new(&rpm_label, Point::new(10, 20), text_style).draw(target)?;
        }

        {
            Rectangle::new(Point::new(0, 210), Size::new(240, 30))
                .into_styled(PrimitiveStyle::with_fill(top_bg))
                .draw(target)?;
            let text_style = MonoTextStyle::new(&font::FONT_7X13_BOLD, Rgb565::BLACK);

            let target_label = format!(
                "Target: {} RPM",
                self.state
                    .target_rpm
                    .load(std::sync::atomic::Ordering::Relaxed)
            );
            Text::new(&target_label, Point::new(10, 228), text_style).draw(target)?;

            let pwm_label = format!(
                "PWM: {}%",
                self.state
                    .fan_pwm
                    .load(std::sync::atomic::Ordering::Relaxed)
            );
            Text::new(&pwm_label, Point::new(160, 228), text_style).draw(target)?;
        }

        Ok(())
    }
}
