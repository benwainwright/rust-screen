#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::time::{Duration, Instant};

use crate::init::init_hardware::init_hardware;
use crate::mqtt::init_mqtt::init_mqtt;
use crate::mqtt::start_loop::start_mqtt_loop;
use crate::utils::mk_static::mk_static;
use crate::wifi::init_wifi::init_wifi;

mod init;
mod mqtt;
mod utils;
mod wifi;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

fn blocking_delay(duration: Duration) {
    let delay_start = Instant::now();
    while delay_start.elapsed() < duration {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = init_hardware();
    let mut led = Output::new(peripherals.GPIO2, Level::High, OutputConfig::default());

    let tcp_client = init_wifi(
        spawner,
        peripherals.TIMG0,
        peripherals.SW_INTERRUPT,
        peripherals.WIFI,
    )
    .await;

    let buffer = mk_static!(
        rust_mqtt::buffer::AllocBuffer,
        rust_mqtt::buffer::AllocBuffer
    );

    let mqtt_client = init_mqtt(&tcp_client, buffer).await;
    spawner.spawn(start_mqtt_loop(mqtt_client).unwrap());

    loop {
        led.toggle();
        blocking_delay(Duration::from_millis(500));
    }
}
