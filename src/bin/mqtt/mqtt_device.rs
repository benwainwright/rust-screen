use core::error::Error;

use alloc::{format, string::String, vec};
use embassy_net::tcp::client::TcpConnection;
use embedded_nal_async::TcpConnect;
use rust_mqtt::Bytes;
use serde::Serializer;

const HOMEASSISTANT_STATUS_TOPIC: &str = "homeassistant/status";

use crate::mqtt::{
    discovery_config::{DeviceDetails, DiscoveryConfig, OriginDetails},
    mqtt_client::MqttClient,
    mqtt_entity::MqttEntity,
};

struct MqttDevice<'a, TConnection: TcpConnect> {
    device_id: String,
    name: String,
    origin_name: String,
    sw_version: String,
    support_url: String,
    discovery_prefix: String,
    mqtt_client: &'a mut MqttClient<'a, TConnection>,
    entities: vec::Vec<MqttEntity<'a, TConnection, dyn erased_serde::Serialize + 'a>>,
}

enum MqttDeviceError {
    SerializationError,
    Json(serde_json::Error),
}
impl From<serde_json::Error> for MqttDeviceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl<'a, TcpConnection: TcpConnect> MqttDevice<'a, TcpConnection> {
    async fn trigger_discovery(&mut self) -> Result<(), MqttDeviceError> {
        let discovery_topic = format!("{}/device/{}/config", self.discovery_prefix, self.device_id);
        let discovery_config = DiscoveryConfig {
            dev: DeviceDetails {
                name: &self.name,
                ids: &self.device_id,
            },
            origin: OriginDetails {
                name: &self.origin_name,
                support_url: &self.support_url,
                sw_version: &self.sw_version,
            },
        };
        for entity in self.entities.iter() {}

        let config_json = serde_json::to_value(&discovery_config)?;

        config_json.as_object().unwrap().extend_one(item);

        Ok(())
    }

    pub async fn initialize(&mut self) -> Result<(), MqttDeviceError> {
        self.trigger_discovery().await?;
        self.mqtt_client.subscribe(HOMEASSISTANT_STATUS_TOPIC).await;
        Ok(())
    }

    pub async fn poll(&mut self) -> Result<(), MqttDeviceError> {
        if let Some(message) = self
            .mqtt_client
            .wait_for_message_on_topic(HOMEASSISTANT_STATUS_TOPIC)
            .await
        {
            match message.message.as_str() {
                "online" => self.trigger_discovery().await?,
                _ => {}
            }
        }
        Ok(())
    }
}
