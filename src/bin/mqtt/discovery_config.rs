use alloc::string::String;
use serde::Serialize;

#[derive(Serialize)]
pub struct DeviceDetails<'a> {
    pub name: &'a String,
    pub ids: &'a String,
}

#[derive(Serialize)]
pub struct OriginDetails<'a> {
    pub name: &'a String,
    pub sw_version: &'a String,
    pub support_url: &'a String,
}

#[derive(Serialize)]
pub struct DiscoveryConfig<'a> {
    pub dev: DeviceDetails<'a>,
    pub origin: OriginDetails<'a>,
}
