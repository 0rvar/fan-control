use std::sync::{atomic::AtomicU32, Arc};

use animations::LeekSpin;
use color::rgb888_to_rgb565;
use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
    Drawable,
};
use profont::{PROFONT_14_POINT, PROFONT_24_POINT};

pub mod animations;
pub mod color;
pub mod rley;

pub struct InterfaceState {
    pub fan_rpm: AtomicU32,
    pub fan_pwm: AtomicU32,
    pub target_rpm: AtomicU32,
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

    pub fn render<D>(&mut self, target: &mut D, clock_ms: u32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let (y_min, y_max) = if clock_ms == 0 { (0, 240) } else { (30, 210) };
        self.animation.render(target, clock_ms, (y_min, y_max))?;

        let top_bg = rgb888_to_rgb565(255u8, 182u8, 140u8);
        {
            let rpm_label = format!(
                "{: >4} RPM",
                self.state
                    .fan_rpm
                    .load(std::sync::atomic::Ordering::Relaxed)
            );
            let mut text_style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::BLACK);

            text_style.background_color = Some(top_bg);
            Text::new(&rpm_label, Point::new(8, 24 + 2), text_style).draw(target)?;
        }

        {
            if clock_ms == 0 {
                Rectangle::new(Point::new(0, 210), Size::new(240, 30))
                    .into_styled(PrimitiveStyle::with_fill(top_bg))
                    .draw(target)?;
            }
            let mut text_style = MonoTextStyle::new(&PROFONT_14_POINT, Rgb565::BLACK);
            text_style.background_color = Some(top_bg);

            let target_label = format!(
                "T: {: <4}RPM",
                self.state
                    .target_rpm
                    .load(std::sync::atomic::Ordering::Relaxed)
            );
            Text::new(&target_label, Point::new(10, 228), text_style).draw(target)?;

            let pwm_label = format!(
                "PWM: {: <3}",
                self.state
                    .fan_pwm
                    .load(std::sync::atomic::Ordering::Relaxed)
            );
            Text::new(&pwm_label, Point::new(150, 228), text_style).draw(target)?;
        }

        Ok(())
    }
}
