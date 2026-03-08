use std::{error::Error, future::Future, pin::Pin};

use crate::{network::ConnectionRequest, wifi::WifiNetwork};

pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

pub trait NetworkBackend {
    fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>>;
    fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>>;
    fn scan_networks(&self) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>>;
    fn connect(&self, request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>>;
    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>>;
}

#[cfg(feature = "demo")]
#[derive(Debug, Default, Clone, Copy)]
pub struct DemoNetworkBackend;

#[cfg(feature = "demo")]
impl NetworkBackend for DemoNetworkBackend {
    fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>> {
        crate::network::demo::get_connected_ssid()
    }

    fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>> {
        crate::network::demo::get_wifi_adapter_name()
    }

    fn scan_networks(&self) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>> {
        Box::pin(crate::network::demo::scan_wifi_networks())
    }

    fn connect(&self, request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
        crate::network::demo::connect_to_network(request)
    }

    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
        crate::network::demo::disconnect_from_network(network)
    }
}

#[cfg(not(feature = "demo"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct NetworkManagerBackend;

#[cfg(not(feature = "demo"))]
impl NetworkBackend for NetworkManagerBackend {
    fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>> {
        crate::network::networkmanager::get_connected_ssid()
    }

    fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>> {
        crate::network::networkmanager::get_wifi_adapter_name()
    }

    fn scan_networks(&self) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>> {
        Box::pin(crate::network::networkmanager::scan_wifi_networks())
    }

    fn connect(&self, request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
        crate::network::networkmanager::connect_to_network(request)
    }

    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
        crate::network::networkmanager::disconnect_from_network(network)
    }
}

#[cfg(feature = "demo")]
pub fn default_backend() -> Box<dyn NetworkBackend> {
    Box::new(DemoNetworkBackend)
}

#[cfg(not(feature = "demo"))]
pub fn default_backend() -> Box<dyn NetworkBackend> {
    Box::new(NetworkManagerBackend)
}
