use core::convert::TryInto;
use std::{sync::Arc, thread::JoinHandle, time::SystemTime};

use embedded_svc::{
    http::{Headers, Method},
    io::{Read, Write},
    wifi::{ClientConfiguration, Configuration},
};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::EspHttpServer,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};
use fan_control_graphics::InterfaceState;
use log::*;
use serde::{Deserialize, Serialize};

use crate::threads;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

// Max payload length for POST requests
const MAX_LEN: usize = 128;

const STACK_SIZE_KB: usize = 10;
const STACK_SIZE: usize = STACK_SIZE_KB * 1024;

#[derive(Serialize)]
struct FanStatus {
    pwm_percent: u32,
    fan_rpm: u32,
    uptime_secs: u64,
}

#[derive(Deserialize)]
struct PwmCommand {
    percent: u32,
}

pub fn spawn_wifi_control_thread(
    state: Arc<InterfaceState>,
    modem: esp_idf_hal::modem::Modem,
) -> JoinHandle<()> {
    threads::EspThread::new("wifi_control")
        .with_stack_size(STACK_SIZE_KB)
        .spawn(move || {
            if let Err(e) = wifi_control_thread(state, modem) {
                error!("WiFi control thread failed: {:?}", e);
            }
        })
}

fn wifi_control_thread(
    state: Arc<InterfaceState>,
    modem: esp_idf_hal::modem::Modem,
) -> anyhow::Result<()> {
    let start_time = SystemTime::now();
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sys_loop.clone(), Some(nvs))?, sys_loop)?;

    connect_wifi(&mut wifi)?;

    let mut server = create_server()?;

    // GET / - Returns current status
    let state_clone = state.clone();
    server.fn_handler("/", Method::Get, move |req| {
        let status = create_fan_status(&state_clone, &start_time);

        let json = serde_json::to_string(&status)?;
        let mut resp = req.into_ok_response()?;
        resp.write_all(json.as_bytes())?;
        Result::<(), anyhow::Error>::Ok(())
    })?;

    // POST /pwm - Sets PWM and returns status
    let state_clone = state.clone();
    server.fn_handler("/pwm", Method::Post, move |mut req| {
        let len = req.content_len().unwrap_or(0) as usize;
        if len > MAX_LEN {
            req.into_status_response(413)?
                .write_all("Request too big".as_bytes())?;
            return Result::<(), anyhow::Error>::Ok(());
        }

        let mut buf = vec![0; len];
        req.read_exact(&mut buf)?;

        match serde_json::from_slice::<PwmCommand>(&buf) {
            Ok(cmd) => {
                // Update PWM
                state_clone
                    .fan_pwm
                    .store(cmd.percent.min(100), std::sync::atomic::Ordering::Relaxed);

                let status = create_fan_status(&state_clone, &start_time);

                let json = serde_json::to_string(&status)?;
                let mut resp = req.into_ok_response()?;
                resp.write_all(json.as_bytes())?;
            }
            Err(e) => {
                req.into_status_response(400)?
                    .write_all(format!("Invalid JSON: {}", e).as_bytes())?;
            }
        }
        Ok(())
    })?;

    loop {
        if !wifi.is_connected()? {
            error!("WiFi connection lost, attempting to reconnect...");
            if let Err(e) = wifi.connect() {
                error!("Failed to reconnect: {:?}", e);
            } else if let Err(e) = wifi.wait_netif_up() {
                error!(
                    "Network interface failed to come up after reconnect: {:?}",
                    e
                );
            } else {
                info!("WiFi reconnected successfully");
                if let Ok(ip_info) = wifi.wifi().sta_netif().get_ip_info() {
                    info!("IP info after reconnect: {:#?}", ip_info);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

fn create_fan_status(state: &InterfaceState, start_time: &SystemTime) -> FanStatus {
    let uptime = SystemTime::now()
        .duration_since(*start_time)
        .unwrap_or_default()
        .as_secs();

    FanStatus {
        pwm_percent: state.fan_pwm.load(std::sync::atomic::Ordering::Relaxed),
        fan_rpm: state.fan_rpm.load(std::sync::atomic::Ordering::Relaxed),
        uptime_secs: uptime,
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("WiFi started");

    wifi.connect()?;
    info!("WiFi connected");

    wifi.wait_netif_up()?;
    info!("WiFi network interface up");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("IP info: {:#?}", ip_info);

    Ok(())
}

fn create_server() -> anyhow::Result<EspHttpServer<'static>> {
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    Ok(EspHttpServer::new(&server_configuration)?)
}
