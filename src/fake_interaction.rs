use std::sync::Arc;
use std::time::SystemTime;

use esp_idf_hal::delay::FreeRtos;
use fan_control_graphics::InterfaceState;

use crate::threads::debug_dump_stack_info;

pub fn fake_interaction_loop(state: Arc<InterfaceState>) {
    debug_dump_stack_info();
    let start = SystemTime::now();
    let mut last_iteration = SystemTime::now();
    loop {
        let now = SystemTime::now();
        fake_interaction(&state, start, last_iteration);
        last_iteration = now;

        FreeRtos::delay_ms(100);
    }
}
fn fake_interaction(state: &Arc<InterfaceState>, start: SystemTime, last_iteration: SystemTime) {
    use std::sync::atomic::Ordering;
    let clock_ms = start.elapsed().unwrap_or_default().as_millis() as u32;
    let delta_ms = last_iteration.elapsed().unwrap_or_default().as_millis() as u32;

    let clock_s = clock_ms as f32 / 1000.0;

    // PWM is now directly controlled by encoder, so we just need to simulate the RPM response
    let current_pwm = state.fan_pwm.load(Ordering::Relaxed) as f32;
    let current_rpm = state.fan_rpm.load(Ordering::Relaxed) as f32;

    // Calculate target RPM based on PWM (linear relationship)
    const MAX_RPM: f32 = 2400.0;
    let target_rpm = (current_pwm / 100.0 * MAX_RPM).clamp(0.0, MAX_RPM);

    // Simulate fan physics
    // We want to cover the full RPM range (2400) in 2 seconds
    const RPM_PER_MS: f32 = 0.6; // 600 RPM per second = full range in 2 seconds

    let rpm_error = target_rpm - current_rpm;
    let max_rpm_change = RPM_PER_MS * delta_ms as f32;
    let rpm_change = rpm_error.clamp(-max_rpm_change, max_rpm_change);

    // Add very subtle random variation (±2 RPM maximum)
    let jitter = (clock_s * 2.0).sin() * 1.0;

    let new_rpm = (current_rpm + rpm_change + jitter).round() as u32;
    state.fan_rpm.store(new_rpm, Ordering::Relaxed);
}
