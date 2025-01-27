use std::{
    sync::{atomic::AtomicU32, Arc},
    time::SystemTime,
};

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
    boot_time: SystemTime,
}

impl Interface {
    pub fn new(state: Arc<InterfaceState>) -> Self {
        Self {
            state,
            animation: LeekSpin::new(),
            boot_time: SystemTime::now(),
        }
    }

    pub fn render<D>(&mut self, target: &mut D, clock_ms: u32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let (y_min, y_max) = if clock_ms == 0 { (0, 240) } else { (30, 210) };
        let rpm = self
            .state
            .fan_rpm
            .load(std::sync::atomic::Ordering::Relaxed);
        self.animation
            .render(target, clock_ms, (y_min, y_max), rpm)?;

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

            let target_rpm = self
                .state
                .target_rpm
                .load(std::sync::atomic::Ordering::Relaxed);
            let target_label = format!("T: {target_rpm: <4}");
            Text::new(&target_label, Point::new(10, 228), text_style).draw(target)?;

            let uptime = self.boot_time.elapsed().unwrap().as_secs();
            let uptime_label = format_uptime_secs(uptime);
            Text::new(&uptime_label, Point::new(104, 228), text_style).draw(target)?;

            let pwm = self
                .state
                .fan_pwm
                .load(std::sync::atomic::Ordering::Relaxed);
            let pwm = pwm - (pwm % 5);
            let pwm_label = format!("PWM:{pwm: >3}");
            Text::new(&pwm_label, Point::new(160, 228), text_style).draw(target)?;
        }

        Ok(())
    }
}

fn format_uptime_secs(secs: u64) -> String {
    if secs < 60 {
        return format!("{secs:02}s");
    }
    let minutes = secs / 60;
    if minutes < 60 {
        return format!("{minutes: >2}m");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours: >2}h");
    }
    let days = hours / 24;
    if days < 7 {
        return format!("{days: >2}d");
    }
    let weeks = days / 7;
    if weeks < 52 {
        return format!("{weeks: >2}w");
    }
    let years = weeks / 52;
    format!("{years: <2}y")
}
