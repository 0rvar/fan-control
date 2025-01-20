use std::sync::{atomic::AtomicU32, Arc};

use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb565,
    prelude::{RgbColor, Size},
};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use fan_control_graphics::{Interface, InterfaceState};

fn main() {
    let mut display = SimulatorDisplay::<Rgb565>::new(Size::new(240, 240));
    let output_settings = OutputSettingsBuilder::new().build();
    let mut window = Window::new("Hello World", &output_settings);
    display.clear(Rgb565::BLACK).unwrap();
    window.update(&mut display);

    let state = Arc::new(InterfaceState {
        fan_rpm: AtomicU32::new(0),
        fan_pwm: AtomicU32::new(0),
        target_rpm: AtomicU32::new(0),
    });
    update_state(&state, 0, 0);
    let start = std::time::Instant::now();
    let mut last_iteration = std::time::Instant::now();

    let mut interface = Interface::new(state.clone());

    loop {
        let clock_ms = start.elapsed().as_millis() as u32;
        let delta_ms = last_iteration.elapsed().as_millis() as u32;
        last_iteration = std::time::Instant::now();
        update_state(&state, clock_ms, delta_ms);

        display.clear(Rgb565::BLACK).unwrap();
        interface.render(&mut display, clock_ms).unwrap();
        window.update(&mut display);

        if window.events().any(|e| e == SimulatorEvent::Quit) {
            return;
        }
        let delay = 100u64.saturating_sub(delta_ms as u64).max(1);
        std::thread::sleep(std::time::Duration::from_millis(delay));
    }
}
fn update_state(state: &Arc<InterfaceState>, clock_ms: u32, delta_ms: u32) {
    use std::sync::atomic::Ordering;
    let clock_s = clock_ms as f32 / 1000.0;
    let delta_s = delta_ms as f32 / 1000.0;

    // Define fan speed presets (RPM)
    const SPEED_PRESETS: [u32; 3] = [800, 1400, 2100];

    // Time between target changes (seconds)
    const TARGET_CHANGE_INTERVAL: f32 = 6.0;

    // Update target RPM occasionally
    let preset_index = ((clock_s / TARGET_CHANGE_INTERVAL) as usize) % SPEED_PRESETS.len();
    let target_rpm = SPEED_PRESETS[preset_index];
    state.target_rpm.store(target_rpm, Ordering::Relaxed);

    // Get current RPM
    let current_rpm = state.fan_rpm.load(Ordering::Relaxed) as f32;
    let target_rpm = target_rpm as f32;

    // Calculate base PWM for target RPM (assume linear relationship)
    // Maximum RPM (2400) should correspond to PWM 100%
    const MAX_RPM: f32 = 2400.0;
    let target_pwm = (target_rpm / MAX_RPM * 100.0).clamp(20.0, 100.0);

    // Get current PWM
    let current_pwm = state.fan_pwm.load(Ordering::Relaxed) as f32;

    // Very gentle PWM adjustment (no overshooting)
    let pwm_diff = target_pwm - current_pwm;
    let new_pwm = (current_pwm + pwm_diff * delta_s * 2.0).clamp(0.0, 100.0);
    state
        .fan_pwm
        .store(new_pwm.round() as u32, Ordering::Relaxed);

    // Simulate fan physics
    // We want to cover the full RPM range (2400) in 2 seconds
    // So rate should be 1200 RPM/second or 0.1 RPM/ms
    const RPM_PER_MS: f32 = 0.6; // 600 RPM per second = full range in 2 seconds

    let rpm_error = target_rpm - current_rpm;
    let max_rpm_change = RPM_PER_MS * delta_ms as f32;
    let rpm_change = rpm_error.clamp(-max_rpm_change, max_rpm_change);

    // Add very subtle random variation (±2 RPM maximum)
    let jitter = (clock_s * 2.0).sin() * 1.0;

    let new_rpm = (current_rpm + rpm_change + jitter).round() as u32;
    state.fan_rpm.store(new_rpm, Ordering::Relaxed);
}
