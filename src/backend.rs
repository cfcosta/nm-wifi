use std::{
    error::Error,
    future::Future,
    io,
    pin::Pin,
    sync::mpsc::{self, Receiver, TryRecvError},
};

use crate::{
    app::runtime::{
        RuntimeBackendDriver,
        RuntimeEvent,
        RuntimeRequest,
        ScanSnapshot,
    },
    network::ConnectionRequest,
    wifi::WifiNetwork,
};

pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

pub trait NetworkBackend {
    fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>>;
    fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>>;
    fn scan_networks(
        &self,
    ) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>>;
    fn connect(
        &self,
        request: ConnectionRequest<'_>,
    ) -> Result<(), Box<dyn Error>>;
    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>>;
}

fn runtime_channel_closed_error() -> Box<dyn Error> {
    io::Error::other("runtime backend event channel closed").into()
}

fn poll_pending_event(
    pending_event: &mut Option<Receiver<RuntimeEvent>>,
) -> Result<Option<RuntimeEvent>, Box<dyn Error>> {
    match pending_event.as_mut() {
        Some(receiver) => match receiver.try_recv() {
            Ok(event) => {
                *pending_event = None;
                Ok(Some(event))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                *pending_event = None;
                Err(runtime_channel_closed_error())
            }
        },
        None => Ok(None),
    }
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

    fn scan_networks(
        &self,
    ) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>> {
        Box::pin(crate::network::demo::scan_wifi_networks())
    }

    fn connect(
        &self,
        request: ConnectionRequest<'_>,
    ) -> Result<(), Box<dyn Error>> {
        crate::network::demo::connect_to_network(request)
    }

    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
        crate::network::demo::disconnect_from_network(network)
    }
}

#[cfg(feature = "demo")]
#[derive(Default)]
struct DemoRuntimeDriver {
    pending_event: Option<Receiver<RuntimeEvent>>,
}

#[cfg(feature = "demo")]
impl RuntimeBackendDriver for DemoRuntimeDriver {
    fn begin(&mut self, request: RuntimeRequest) {
        let (sender, receiver) = mpsc::channel();
        let event = match request {
            RuntimeRequest::Scan => {
                RuntimeEvent::ScanFinished(Ok(ScanSnapshot {
                    networks: crate::network::demo::demo_networks(),
                    adapter_name: crate::network::demo::get_wifi_adapter_name()
                        .ok()
                        .flatten(),
                }))
            }
            RuntimeRequest::Connect {
                network,
                passphrase,
            } => {
                let result = match passphrase.as_deref() {
                    Some(passphrase) => {
                        crate::network::demo::connect_to_network(
                            ConnectionRequest::Secured {
                                network: &network,
                                passphrase,
                            },
                        )
                    }
                    None => crate::network::demo::connect_to_network(
                        ConnectionRequest::Open { network: &network },
                    ),
                };
                RuntimeEvent::ConnectFinished(
                    result.map_err(|error| error.to_string()),
                )
            }
            RuntimeRequest::Disconnect { network } => {
                RuntimeEvent::DisconnectFinished(
                    crate::network::demo::disconnect_from_network(&network)
                        .map_err(|error| error.to_string()),
                )
            }
        };
        let _ = sender.send(event);
        self.pending_event = Some(receiver);
    }

    fn poll_event(&mut self) -> Result<Option<RuntimeEvent>, Box<dyn Error>> {
        poll_pending_event(&mut self.pending_event)
    }
}

#[cfg(feature = "demo")]
pub(crate) fn default_runtime_driver() -> Box<dyn RuntimeBackendDriver> {
    Box::new(DemoRuntimeDriver::default())
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

    fn scan_networks(
        &self,
    ) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>> {
        Box::pin(crate::network::networkmanager::scan_wifi_networks())
    }

    fn connect(
        &self,
        request: ConnectionRequest<'_>,
    ) -> Result<(), Box<dyn Error>> {
        crate::network::networkmanager::connect_to_network(request)
    }

    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
        crate::network::networkmanager::disconnect_from_network(network)
    }
}

#[cfg(not(feature = "demo"))]
#[derive(Default)]
struct NetworkManagerRuntimeDriver {
    pending_event: Option<Receiver<RuntimeEvent>>,
}

#[cfg(not(feature = "demo"))]
impl RuntimeBackendDriver for NetworkManagerRuntimeDriver {
    fn begin(&mut self, request: RuntimeRequest) {
        let (sender, receiver) = mpsc::channel();

        match request {
            RuntimeRequest::Scan => {
                tokio::spawn(async move {
                    let event = match tokio::task::spawn_blocking(|| {
                        let networks = crate::network::networkmanager::scan_wifi_networks_blocking();
                        let adapter_name = crate::network::networkmanager::get_wifi_adapter_name()
                            .ok()
                            .flatten();

                        match networks {
                            Ok(networks) => RuntimeEvent::ScanFinished(Ok(ScanSnapshot {
                                networks,
                                adapter_name,
                            })),
                            Err(error) => {
                                RuntimeEvent::ScanFinished(Err(error.to_string()))
                            }
                        }
                    })
                    .await
                    {
                        Ok(event) => event,
                        Err(error) => RuntimeEvent::ScanFinished(Err(format!(
                            "runtime scan task failed: {error}"
                        ))),
                    };

                    let _ = sender.send(event);
                });
            }
            RuntimeRequest::Connect {
                network,
                passphrase,
            } => {
                tokio::spawn(async move {
                    let event = match tokio::task::spawn_blocking(move || {
                        let result = match passphrase.as_deref() {
                            Some(passphrase) => crate::network::networkmanager::connect_to_network(
                                ConnectionRequest::Secured {
                                    network: &network,
                                    passphrase,
                                },
                            ),
                            None => crate::network::networkmanager::connect_to_network(
                                ConnectionRequest::Open { network: &network },
                            ),
                        };

                        RuntimeEvent::ConnectFinished(result.map_err(|error| error.to_string()))
                    })
                    .await
                    {
                        Ok(event) => event,
                        Err(error) => RuntimeEvent::ConnectFinished(Err(format!(
                            "runtime connect task failed: {error}"
                        ))),
                    };

                    let _ = sender.send(event);
                });
            }
            RuntimeRequest::Disconnect { network } => {
                tokio::spawn(async move {
                    let event = match tokio::task::spawn_blocking(move || {
                        RuntimeEvent::DisconnectFinished(
                            crate::network::networkmanager::disconnect_from_network(&network)
                                .map_err(|error| error.to_string()),
                        )
                    })
                    .await
                    {
                        Ok(event) => event,
                        Err(error) => RuntimeEvent::DisconnectFinished(Err(format!(
                            "runtime disconnect task failed: {error}"
                        ))),
                    };

                    let _ = sender.send(event);
                });
            }
        }

        self.pending_event = Some(receiver);
    }

    fn poll_event(&mut self) -> Result<Option<RuntimeEvent>, Box<dyn Error>> {
        poll_pending_event(&mut self.pending_event)
    }
}

#[cfg(not(feature = "demo"))]
pub(crate) fn default_runtime_driver() -> Box<dyn RuntimeBackendDriver> {
    Box::new(NetworkManagerRuntimeDriver::default())
}

#[cfg(feature = "demo")]
pub fn default_backend() -> Box<dyn NetworkBackend> {
    Box::new(DemoNetworkBackend)
}

#[cfg(not(feature = "demo"))]
pub fn default_backend() -> Box<dyn NetworkBackend> {
    Box::new(NetworkManagerBackend)
}
