use embassy_net::tcp::client::TcpClient;

use crate::mqtt::mqtt_client::MqttClient;

type AppMqttClient = MqttClient<'static, TcpClient<'static, 1, 1500, 1500>>;

#[embassy_executor::task]
pub async fn start_mqtt_loop(mut client: AppMqttClient) {
    client.connect().await;
    client.run_loop().await;
}
