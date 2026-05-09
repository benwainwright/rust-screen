use core::str::FromStr;

use alloc::{format, string::String};
use embedded_nal_async::TcpConnect;
use serde::{Serialize, Serializer};

use crate::mqtt::mqtt_client::MqttClient;

#[derive(Serialize)]
struct MqttEntityDefaultConfig {
    device_class: String,
    state_topic: String,
    unique_id: String,
    platform: String,
    command_topic: String,
    name: String,
}

pub struct MqttEntityInit<'a, TConnection: TcpConnect, TAdditionalProperties> {
    pub friendly_name: String,
    pub discovery_prefix: String,
    pub device_class: String,
    pub default_state: String,
    pub unique_id: String,
    pub platform: String,
    additional_properties: Option<alloc::boxed::Box<dyn erased_serde::Serialize + 'a>>,
    pub mqtt_client: &'a mut MqttClient<'a, TConnection>,
}

#[derive(Serialize)]
pub struct MqttEntity<'a, TConnnection: TcpConnect, TAdditionalProperties>
where
    TAdditionalProperties: Serialize,
{
    discovery_prefix: String,
    default_state: String,

    #[serde(skip_serializing)]
    state: String,

    #[serde(flatten)]
    default_config: MqttEntityDefaultConfig,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    additional_properties: Option<alloc::boxed::Box<dyn erased_serde::Serialize + 'a>>,

    #[serde(skip_serializing)]
    mqtt_client: &'a mut MqttClient<'a, TConnnection>,
}

impl<'a, TConnection, TAdditionalProperties> MqttEntity<'a, TConnection, TAdditionalProperties>
where
    TConnection: TcpConnect,
    TAdditionalProperties: Serialize,
{
    pub fn new(init: MqttEntityInit<'a, TConnection, TAdditionalProperties>) -> Self {
        let state_topic = format!(
            "{}/{}/{}/{}/state",
            &init.discovery_prefix, init.unique_id, init.platform, init.device_class
        );
        let command_topic = format!(
            "{}/{}/{}/{}/set",
            init.discovery_prefix, init.unique_id, init.platform, init.device_class
        );
        Self {
            discovery_prefix: init.discovery_prefix,
            default_state: init.default_state,
            state: String::from_str("").unwrap(),
            additional_properties: init.additional_properties,
            default_config: MqttEntityDefaultConfig {
                device_class: init.device_class,
                state_topic,
                unique_id: init.unique_id,
                platform: init.platform,
                name: init.friendly_name,
                command_topic,
            },
            mqtt_client: init.mqtt_client,
        }
    }

    pub async fn poll(&self) {
        if let Some(message) = self
            .mqtt_client
            .wait_for_message_on_topic(&self.default_config.command_topic)
            .await
        {
            self.state = message.message;
        }
        Ok(())
    }
}
