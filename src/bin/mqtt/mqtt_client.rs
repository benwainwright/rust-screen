use core::{
    net::{Ipv4Addr, SocketAddr},
    num::NonZero,
    pin::Pin,
    str::FromStr,
};

use esp_println::println;

struct SubscriptionMessage {
    pub topic: String,
    pub message: String,
}

use alloc::{
    borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String, vec::Vec,
};
use embedded_nal_async::TcpConnect;
use rust_mqtt::{
    buffer::AllocBuffer,
    client::{
        Client,
        event::{Event, Publish, Suback},
        options::{
            ConnectOptions, PublicationOptions, RetainHandling, SubscriptionOptions, TopicReference,
        },
    },
    config::SessionExpiryInterval,
    types::{MqttBinary, MqttString, TopicFilter, TopicName, VarByteInt},
};

fn topic_matches_filter(topic: &TopicName, filter: &TopicFilter) -> bool {
    let topic_string = topic.as_ref();
    let filter_string = filter.as_ref();

    topic_string.eq(filter_string)
}

pub struct MqttClient<'a, TConnection: TcpConnect> {
    tcp_client: &'a TConnection,
    mqtt_client: Client<'a, TConnection::Connection<'a>, AllocBuffer, 30, 1500, 1500, 30>,
    user: String,
    port: u16,
    password: String,
    host: String,
}

impl<'a, TConnection: TcpConnect> MqttClient<'a, TConnection> {
    pub fn new(
        tcp_client: &'a TConnection,
        user: &str,
        buffer: &'a mut AllocBuffer,
        password: &str,
        host: &str,
        port: u16,
    ) -> Self {
        Self {
            tcp_client,
            port,
            user: user.to_owned(),
            password: password.to_owned(),
            host: host.to_owned(),
            mqtt_client: Client::new(buffer),
        }
    }

    pub async fn publish(&mut self, topic: &str, message: &str) -> Option<NonZero<u16>> {
        let topic_name = TopicName::new(MqttString::from_str(topic).unwrap()).unwrap();
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name));

        let identifier = match self.mqtt_client.publish(&pub_options, message.into()).await {
            Ok(the_identifier) => {
                println!(
                    "Published message with packet identifier {}",
                    the_identifier.unwrap()
                );
                the_identifier
            }
            Err(e) => {
                println!("Failed to send Publish {e:?}");
                return None;
            }
        };

        loop {
            match self.mqtt_client.poll().await {
                Ok(Event::PublishComplete(_)) => {
                    println!("Publish complete");
                    return Some(identifier.unwrap().get());
                }
                Ok(e) => println!("Received event {e:?}"),
                Err(e) => {
                    println!("Failed to poll: {e:?}");
                    return None;
                }
            }
        }
    }

    pub async fn connect(&mut self) {
        let ip: Ipv4Addr = Ipv4Addr::from_str(&self.host[..]).unwrap();

        let transport = self
            .tcp_client
            .connect(SocketAddr::new(ip.into(), self.port))
            .await
            .unwrap();

        let connect_options = ConnectOptions::new()
            .clean_start()
            .session_expiry_interval(SessionExpiryInterval::NeverEnd)
            .user_name(MqttString::from_str(&self.user).unwrap())
            .password(MqttBinary::from_slice(self.password.as_bytes()).unwrap());

        match self
            .mqtt_client
            .connect(
                transport,
                &connect_options,
                Some(MqttString::from_str("rust-mqtt-demo").unwrap()),
            )
            .await
        {
            Ok(c) => {
                println!("Connected to server: {c:?}");
                println!("{:?}", self.mqtt_client.client_config());
                println!("{:?}", self.mqtt_client.server_config());
                println!("{:?}", self.mqtt_client.shared_config());
                println!("{:?}", self.mqtt_client.session());
            }
            Err(e) => {
                println!("Failed to connect to server: {e:?}");
                return;
            }
        }
    }

    pub async fn wait_for_message_on_topic(
        &mut self,
        subscribed_topic: &str,
    ) -> Option<SubscriptionMessage> {
        loop {
            match self.mqtt_client.poll().await {
                Ok(Event::Publish(Publish {
                    topic: event_topic,
                    message,
                    payload_format_indicator,
                    ..
                })) => {
                    let mqtt_string = MqttString::from_str(subscribed_topic).unwrap();
                    let filter_string = mqtt_string.as_borrowed();
                    let filter = TopicFilter::new(filter_string).unwrap();

                    if let Some(value) = payload_format_indicator {
                        if topic_matches_filter(&event_topic, &filter) && value {
                            let message_str = str::from_utf8(message.as_bytes()).unwrap();
                            return Some(SubscriptionMessage {
                                topic: String::from_str(event_topic.as_ref().as_str()).unwrap(),
                                message: String::from_str(message_str).unwrap(),
                            });
                        }
                    }
                }
                Ok(_e) => {}
                Err(_e) => {
                    println!("Received an error")
                }
            }
        }
    }

    pub async fn subscribe(&mut self, subscribed_topic: &str) {
        let topic_string = MqttString::from_str(subscribed_topic).unwrap();
        let topic_filter = TopicFilter::new(topic_string.as_borrowed()).unwrap();

        let mut sub_options = SubscriptionOptions::new()
            .retain_handling(RetainHandling::SendIfNotSubscribedBefore)
            .retain_as_published()
            .exactly_once();

        if self
            .mqtt_client
            .server_config()
            .subscription_identifiers_supported
        {
            sub_options.subscription_identifier = Some(VarByteInt::from(42u16));
        }

        match self
            .mqtt_client
            .subscribe(topic_filter.as_borrowed(), sub_options)
            .await
        {
            Ok(_) => println!("Sent Subscribe"),
            Err(e) => {
                println!("Failed to subscribe: {e:?}");
            }
        }

        match self.mqtt_client.poll().await {
            Ok(Event::Suback(Suback {
                packet_identifier: _,
                reason_code,
            })) => {
                println!("Subscribed with reason code {reason_code:?}")
            }
            Ok(e) => {
                println!("Expected Suback but received event {e:?}");
            }
            Err(e) => {
                println!("Failed to receive Suback {e:?}");
            }
        }
    }
}
