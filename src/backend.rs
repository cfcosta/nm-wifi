use std::{error::Error, future::Future, pin::Pin};

use crate::{
    network::{self, ConnectionRequest},
    wifi::WifiNetwork,
};

pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

pub trait NetworkBackend {
    fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>>;
    fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>>;
    fn scan_networks(&self) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>>;
    fn connect(&self, request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>>;
    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CurrentNetworkBackend;

impl NetworkBackend for CurrentNetworkBackend {
    fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>> {
        network::get_connected_ssid()
    }

    fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>> {
        network::get_wifi_adapter_name()
    }

    fn scan_networks(&self) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>> {
        Box::pin(network::scan_wifi_networks())
    }

    fn connect(&self, request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
        network::connect_to_network(request)
    }

    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
        network::disconnect_from_network(network)
    }
}
