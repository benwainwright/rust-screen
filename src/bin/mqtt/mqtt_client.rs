use core::{
    net::{Ipv4Addr, SocketAddr},
    str::FromStr,
};

use esp_println::println;

use alloc::{borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String};
use embedded_nal_async::TcpConnect;
use rust_mqtt::{
    Bytes,
    buffer::AllocBuffer,
    client::{
        Client,
        event::{Event, Publish, Suback},
        options::{ConnectOptions, RetainHandling, SubscriptionOptions},
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
    callbacks: BTreeMap<String, Box<dyn FnMut(Bytes) + 'a>>,
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
            callbacks: BTreeMap::new(),
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

    pub async fn run_loop(&mut self) {
        loop {
            match self.mqtt_client.poll().await {
                Ok(Event::Publish(Publish { topic, message, .. })) => {
                    for (key, callback) in self.callbacks.iter_mut() {
                        let mqtt_string = MqttString::from_str(key).unwrap();
                        let filter_string = mqtt_string.as_borrowed();
                        let filter = TopicFilter::new(filter_string).unwrap();

                        if topic_matches_filter(&topic, &filter) {
                            callback(message.clone())
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

    pub async fn subscribe<F>(&mut self, topic: &str, callback: F)
    where
        F: FnMut(Bytes) + 'a,
    {
        let topic_string = MqttString::from_str(topic).unwrap();
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
                return;
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
                return;
            }
            Err(e) => {
                println!("Failed to receive Suback {e:?}");
                return;
            }
        }

        self.callbacks
            .insert(String::from_str(topic).unwrap(), Box::new(callback));
    }
}
