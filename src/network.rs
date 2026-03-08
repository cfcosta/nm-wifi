use std::{collections::HashMap, error::Error, process::Command, time::Duration};

use networkmanager::{
    NetworkManager,
    devices::{Device, Wireless},
};
use tokio::time::sleep;

use crate::types::WifiNetwork;

pub async fn get_connected_ssid() -> Option<String> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
        .output()
        .ok()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if let Some(stripped) = line.strip_prefix("yes:") {
                return Some(stripped.to_string());
            }
        }
    }
    None
}

fn parse_connected_wifi_device(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let mut parts = line.splitn(3, ':');
        let device = parts.next()?;
        let device_type = parts.next()?;
        let state = parts.next()?;

        (device_type == "wifi" && state == "connected").then(|| device.to_string())
    })
}

fn parse_any_wifi_device(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let mut parts = line.splitn(3, ':');
        let device = parts.next()?;
        let device_type = parts.next()?;

        (device_type == "wifi").then(|| device.to_string())
    })
}

pub async fn get_wifi_adapter_info() -> Option<String> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "DEVICE,TYPE,STATE", "dev"])
        .output()
        .ok()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        return parse_connected_wifi_device(&output_str)
            .or_else(|| parse_any_wifi_device(&output_str));
    }
    None
}

pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, Box<dyn Error>> {
    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|_| "Failed to connect to D-Bus".to_string())?;
    let nm = NetworkManager::new(&dbus);

    // Get currently connected SSID
    let connected_ssid = get_connected_ssid().await;

    let devices = nm
        .get_devices()
        .map_err(|_| "Failed to get devices".to_string())?;

    for device in devices {
        if let Device::WiFi(wifi_device) = device {
            wifi_device
                .request_scan(HashMap::new())
                .map_err(|_| "Failed to request scan".to_string())?;

            // Brief wait for scan to start
            sleep(Duration::from_millis(200)).await;

            let access_points = wifi_device
                .get_all_access_points()
                .map_err(|_| "Failed to get access points".to_string())?;

            let mut networks = Vec::new();

            for ap in access_points {
                let ssid = ap.ssid().map_err(|_| "Failed to get SSID".to_string())?;
                if !ssid.is_empty() {
                    let flags = ap.flags().map_err(|_| "Failed to get flags".to_string())?;
                    let wpa_flags = ap
                        .wpa_flags()
                        .map_err(|_| "Failed to get WPA flags".to_string())?;
                    let rsn_flags = ap
                        .rsn_flags()
                        .map_err(|_| "Failed to get RSN flags".to_string())?;

                    let secured = rsn_flags != 0 || wpa_flags != 0 || (flags & 0x1) != 0;

                    let signal_strength = ap
                        .strength()
                        .map_err(|_| "Failed to get signal strength".to_string())?;

                    let frequency = ap
                        .frequency()
                        .map_err(|_| "Failed to get frequency".to_string())?;

                    let connected = connected_ssid.as_ref() == Some(&ssid);

                    let network = WifiNetwork {
                        ssid,
                        signal_strength,
                        secured,
                        frequency,
                        connected,
                    };
                    networks.push(network);
                }
            }

            // Deduplicate networks by SSID, keeping the one with highest frequency
            let mut unique_networks: HashMap<String, WifiNetwork> = HashMap::new();
            for network in networks {
                match unique_networks.get(&network.ssid) {
                    Some(existing) => {
                        if network.frequency > existing.frequency {
                            unique_networks.insert(network.ssid.clone(), network);
                        }
                    }
                    None => {
                        unique_networks.insert(network.ssid.clone(), network);
                    }
                }
            }

            let mut deduplicated_networks: Vec<WifiNetwork> = unique_networks.into_values().collect();

            // Sort by connection status first, then by signal strength
            deduplicated_networks.sort_by(|a, b| match (a.connected, b.connected) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.signal_strength.cmp(&a.signal_strength),
            });

            return Ok(deduplicated_networks);
        }
    }

    Ok(Vec::new())
}

fn connect_command_args(
    network: &WifiNetwork,
    password: Option<&str>,
) -> Result<Vec<String>, Box<dyn Error>> {
    if network.secured && password.is_none() {
        return Err("Password required for secured network".into());
    }

    let mut args = vec![
        "device".to_string(),
        "wifi".to_string(),
        "connect".to_string(),
        network.ssid.clone(),
    ];

    if let Some(password) = password {
        args.push("password".to_string());
        args.push(password.to_string());
    }

    Ok(args)
}

pub async fn connect_to_network(
    network: &WifiNetwork,
    password: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let args = connect_command_args(network, password)?;
    let output = Command::new("nmcli")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("nmcli failed: {}", error_msg).into())
    }
}

pub async fn disconnect_from_network(_network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
    let adapter = get_wifi_adapter_info()
        .await
        .ok_or_else(|| "Failed to find connected WiFi adapter".to_string())?;

    let output = Command::new("nmcli")
        .args(["device", "disconnect", &adapter])
        .output()
        .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("nmcli disconnect failed: {}", error_msg).into())
    }
}

#[cfg(test)]
mod tests {
    use super::{connect_command_args, parse_any_wifi_device, parse_connected_wifi_device};
    use crate::types::WifiNetwork;

    #[test]
    fn adapter_parser_prefers_connected_wifi_devices() {
        let output = "eth0:ethernet:connected\nwlp2s0:wifi:connected\nwlan1:wifi:disconnected";
        assert_eq!(
            parse_connected_wifi_device(output),
            Some("wlp2s0".to_string())
        );
    }

    #[test]
    fn adapter_parser_can_fall_back_to_any_wifi_device() {
        let output = "eth0:ethernet:connected\nwlan1:wifi:disconnected";
        assert_eq!(parse_any_wifi_device(output), Some("wlan1".to_string()));
    }

    #[test]
    fn secured_networks_use_nmcli_device_wifi_connect_with_password() {
        let network = WifiNetwork {
            ssid: "home".to_string(),
            signal_strength: 80,
            secured: true,
            frequency: 5180,
            connected: false,
        };

        assert_eq!(
            connect_command_args(&network, Some("hunter2")).unwrap(),
            vec!["device", "wifi", "connect", "home", "password", "hunter2"]
        );
    }
}
