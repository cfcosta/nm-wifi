use std::error::Error;

use crate::{
    network::ConnectionRequest,
    wifi::{WifiNetwork, WifiSecurity},
};

pub fn demo_networks() -> Vec<WifiNetwork> {
    vec![
        WifiNetwork {
            ssid: "CatCat".to_string(),
            signal_strength: 69,
            security: WifiSecurity::WpaSae,
            frequency: 5220,
            connected: true,
        },
        WifiNetwork {
            ssid: "VIVOFIBRA-5210-5G".to_string(),
            signal_strength: 72,
            security: WifiSecurity::WpaPsk,
            frequency: 5200,
            connected: false,
        },
        WifiNetwork {
            ssid: "Coffee Corner".to_string(),
            signal_strength: 54,
            security: WifiSecurity::Open,
            frequency: 2412,
            connected: false,
        },
        WifiNetwork {
            ssid: "Office Secure".to_string(),
            signal_strength: 63,
            security: WifiSecurity::Enterprise,
            frequency: 5745,
            connected: false,
        },
    ]
}

fn demo_connect(request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
    let (network, password) = match request {
        ConnectionRequest::Open { network } => (network, None),
        ConnectionRequest::Secured {
            network,
            passphrase,
        } => (network, Some(passphrase)),
    };

    match (network.ssid.as_str(), network.security, password) {
        ("Coffee Corner", WifiSecurity::Open, _) => Ok(()),
        ("VIVOFIBRA-5210-5G", WifiSecurity::WpaPsk, Some("hunter2")) => Ok(()),
        ("CatCat", WifiSecurity::WpaSae, Some("AcerolaAcai")) => Ok(()),
        (_, WifiSecurity::Enterprise, _) => {
            Err("Demo mode: enterprise networks are not supported".into())
        }
        (_, WifiSecurity::Open, _) => Ok(()),
        (_, _, Some(_)) => Err("Demo mode: invalid password".into()),
        _ => Err("Demo mode: password required for secured network".into()),
    }
}

pub fn get_connected_ssid() -> Result<Option<String>, Box<dyn Error>> {
    Ok(demo_networks()
        .into_iter()
        .find(|network| network.connected)
        .map(|network| network.ssid))
}

pub fn get_wifi_adapter_name() -> Result<Option<String>, Box<dyn Error>> {
    Ok(Some("demo-wlan0".to_string()))
}

pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, Box<dyn Error>> {
    Ok(demo_networks())
}

pub fn connect_to_network(request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
    demo_connect(request)
}

pub fn disconnect_from_network(network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
    if network.connected {
        Ok(())
    } else {
        Err("Demo mode: selected network is not connected".into())
    }
}
