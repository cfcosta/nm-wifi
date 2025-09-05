use std::{
    collections::HashMap,
    error::Error,
    process::Command,
    time::Duration,
};

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

pub async fn get_wifi_adapter_info() -> Option<String> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "DEVICE,TYPE,STATE", "dev"])
        .output()
        .ok()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 && parts[1] == "wifi" && parts[2] == "connected"
            {
                return Some(parts[0].to_string());
            }
        }
        // If no connected adapter, find any wifi adapter
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && parts[1] == "wifi" {
                return Some(parts[0].to_string());
            }
        }
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
                let ssid =
                    ap.ssid().map_err(|_| "Failed to get SSID".to_string())?;
                if !ssid.is_empty() {
                    let flags = ap
                        .flags()
                        .map_err(|_| "Failed to get flags".to_string())?;
                    let wpa_flags = ap
                        .wpa_flags()
                        .map_err(|_| "Failed to get WPA flags".to_string())?;
                    let rsn_flags = ap
                        .rsn_flags()
                        .map_err(|_| "Failed to get RSN flags".to_string())?;

                    let secured =
                        rsn_flags != 0 || wpa_flags != 0 || (flags & 0x1) != 0;

                    let signal_strength = ap.strength().map_err(|_| {
                        "Failed to get signal strength".to_string()
                    })?;

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
            let mut unique_networks: HashMap<String, WifiNetwork> =
                HashMap::new();
            for network in networks {
                match unique_networks.get(&network.ssid) {
                    Some(existing) => {
                        if network.frequency > existing.frequency {
                            unique_networks
                                .insert(network.ssid.clone(), network);
                        }
                    }
                    None => {
                        unique_networks.insert(network.ssid.clone(), network);
                    }
                }
            }

            let mut deduplicated_networks: Vec<WifiNetwork> =
                unique_networks.into_values().collect();

            // Sort by connection status first, then by signal strength
            deduplicated_networks.sort_by(|a, b| {
                match (a.connected, b.connected) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.signal_strength.cmp(&a.signal_strength),
                }
            });

            return Ok(deduplicated_networks);
        }
    }

    Ok(Vec::new())
}

pub async fn connect_to_network(
    network: &WifiNetwork,
    password: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if network.secured && password.is_none() {
        return Err("Password required for secured network".into());
    }

    // Use `nmcli` command line tool for connection
    let mut cmd = Command::new("nmcli");

    if network.secured {
        // For secured networks, use the connection add approach
        cmd.args([
            "connection",
            "add",
            "type",
            "wifi",
            "con-name",
            &network.ssid,
            "ssid",
            &network.ssid,
            "wifi-sec.key-mgmt",
            "wpa-psk",
            "wifi-sec.psk",
            password.unwrap(),
        ]);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute nmcli add: {}", e))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);

            // If connection already exists, try to modify it
            if error_msg.contains("already exists") {
                let mut modify_cmd = Command::new("nmcli");
                modify_cmd.args([
                    "connection",
                    "modify",
                    &network.ssid,
                    "wifi-sec.psk",
                    password.unwrap(),
                ]);

                let modify_output = modify_cmd.output().map_err(|e| {
                    format!("Failed to execute nmcli modify: {}", e)
                })?;

                if !modify_output.status.success() {
                    let modify_error =
                        String::from_utf8_lossy(&modify_output.stderr);
                    return Err(format!(
                        "nmcli modify failed: {}",
                        modify_error
                    )
                    .into());
                }
            } else {
                return Err(format!("nmcli add failed: {}", error_msg).into());
            }
        }

        // Now activate the connection
        let mut activate_cmd = Command::new("nmcli");
        activate_cmd.args(["connection", "up", &network.ssid]);

        let activate_output = activate_cmd
            .output()
            .map_err(|e| format!("Failed to execute nmcli up: {}", e))?;

        if activate_output.status.success() {
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&activate_output.stderr);
            Err(format!("nmcli activation failed: {}", error_msg).into())
        }
    } else {
        // For open networks, use the simple connect command
        cmd.args(["device", "wifi", "connect", &network.ssid]);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(format!("nmcli failed: {}", error_msg).into())
        }
    }
}

pub async fn disconnect_from_network(
    network: &WifiNetwork,
) -> Result<(), Box<dyn Error>> {
    let output = Command::new("nmcli")
        .args(["connection", "down", &network.ssid])
        .output()
        .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("nmcli disconnect failed: {}", error_msg).into())
    }
}
