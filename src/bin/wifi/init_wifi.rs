// Mostly borrowed from https://github.com/esp-rs/no_std-training/blob/main/intro/http-client/examples/http-client.rs

use embassy_executor::Spawner;
use embassy_net::{
    Runner, StackResources,
    tcp::client::{TcpClient, TcpClientState},
};
use embassy_time::{Duration, Timer};
use esp_hal::{
    interrupt::software::SoftwareInterruptControl,
    peripherals::{SW_INTERRUPT, TIMG0, WIFI},
    rng::Rng,
    timer::timg::TimerGroup,
};

use esp_println::println;

use esp_radio::wifi::{
    Config, ControllerConfig, Interface, WifiController, scan::ScanConfig, sta::StationConfig,
};

macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        STATIC_CELL.uninit().write($val)
    }};
}

pub async fn init_wifi(
    spawner: Spawner,
    timg0: TIMG0<'static>,
    sw_interrupt: SW_INTERRUPT<'static>,
    wifi: WIFI<'static>,
) -> &'static TcpClient<'static, 1, 1500, 1500> {
    const SSID: &str = env!("WIFI_SSID");
    const PASSWORD: &str = env!("WIFI_PASSWORD");

    let timer_group0 = TimerGroup::new(timg0);
    let software_interrupt = SoftwareInterruptControl::new(sw_interrupt);

    esp_rtos::start(timer_group0.timer0, software_interrupt.software_interrupt0);

    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.into()),
    );

    println!("Starting Wi-Fi");

    let (mut controller, interfaces) = esp_radio::wifi::new(
        wifi,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .unwrap();

    println!("Wi-Fi configured and started");

    let wifi_interface = interfaces.station;
    let config = embassy_net::Config::dhcpv4(Default::default());

    let random_number_generator = Rng::new();
    let seed =
        (random_number_generator.random() as u64) << 32 | random_number_generator.random() as u64;

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    println!("Scanning for access points");
    let scan_config = ScanConfig::default().with_max(10);
    let result = controller.scan_async(&scan_config).await.unwrap();

    for ap in result {
        println!("{:?}", ap);
    }

    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    stack.wait_config_up().await;
    if let Some(config) = stack.config_v4() {
        println!("Got IP: {}", config.address);
    }

    let tcp_client = TcpClient::new(
        stack,
        mk_static!(
            TcpClientState<1, 1500, 1500>,
            TcpClientState::<1, 1500, 1500>::new()
        ),
    );

    mk_static!(TcpClient<'static, 1, 1500, 1500>, tcp_client)
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        println!("Connecting to Wi-Fi...");

        match controller.connect_async().await {
            Ok(info) => {
                println!("Wi-Fi connected to {:?}", info);
                let info = controller.wait_for_disconnect_async().await.ok();
                println!("Disconnected: {:?}", info);
            }
            Err(err) => println!("Failed to connect to Wi-Fi: {:?}", err),
        }

        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}
