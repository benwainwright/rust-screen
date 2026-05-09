use embassy_net::tcp::client::TcpClient;
use rust_mqtt::buffer::AllocBuffer;

use crate::mqtt::mqtt_client::MqttClient;

pub async fn init_mqtt(
    tcp_client: &'static TcpClient<'static, 1, 1500, 1500>,
    buffer: &'static mut AllocBuffer,
) -> MqttClient<'static, TcpClient<'static, 1, 1500, 1500>> {
    const MQTT_USER: &str = env!("MQTT_USER");
    const MQTT_PASSWORD: &str = env!("MQTT_PASSWORD");
    const MQTT_HOST: &str = env!("MQTT_HOST");

    MqttClient::new(
        tcp_client,
        MQTT_USER,
        buffer,
        MQTT_PASSWORD,
        MQTT_HOST,
        1883,
    )
}
