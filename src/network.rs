use std::{collections::HashMap, error::Error, process::Command, time::Duration};

use dbus::arg::{PropMap, RefArg, Variant};
use networkmanager::{
    NetworkManager,
    devices::{Any, Device, Wireless},
};
use tokio::time::sleep;

use crate::types::WifiNetwork;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionBackend {
    NetworkManager,
    NmcliFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecurityKind {
    Open,
    WpaPsk,
    Unsupported,
}

fn classify_security(network: &WifiNetwork, password: Option<&str>) -> SecurityKind {
    match (network.secured, password) {
        (false, _) => SecurityKind::Open,
        (true, Some(_)) => SecurityKind::WpaPsk,
        (true, None) => SecurityKind::Unsupported,
    }
}

fn connect_backend_for(network: &WifiNetwork, password: Option<&str>) -> ConnectionBackend {
    match classify_security(network, password) {
        SecurityKind::Open | SecurityKind::WpaPsk => ConnectionBackend::NetworkManager,
        SecurityKind::Unsupported => ConnectionBackend::NmcliFallback,
    }
}

fn disconnect_backend_for() -> ConnectionBackend {
    ConnectionBackend::NetworkManager
}

fn should_disconnect_device(active_ssid: Option<&str>, target_ssid: &str) -> bool {
    active_ssid == Some(target_ssid)
}

fn get_connected_ssid_via_nm() -> Option<String> {
    let dbus = dbus::blocking::Connection::new_system().ok()?;
    let nm = NetworkManager::new(&dbus);

    for device in nm.get_devices().ok()? {
        if let Device::WiFi(wifi_device) = device
            && let Ok(access_point) = wifi_device.active_access_point()
            && let Ok(ssid) = access_point.ssid()
            && !ssid.is_empty()
        {
            return Some(ssid);
        }
    }

    None
}

fn get_connected_ssid_via_nmcli() -> Option<String> {
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

pub async fn get_connected_ssid() -> Option<String> {
    get_connected_ssid_via_nm().or_else(get_connected_ssid_via_nmcli)
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

fn choose_wifi_adapter(connected: Option<String>, available: Vec<String>) -> Option<String> {
    connected.or_else(|| available.into_iter().next())
}

fn get_wifi_adapter_info_via_nm() -> Option<String> {
    let dbus = dbus::blocking::Connection::new_system().ok()?;
    let nm = NetworkManager::new(&dbus);
    let mut connected = None;
    let mut available = Vec::new();

    for device in nm.get_devices().ok()? {
        if let Device::WiFi(wifi_device) = device {
            let iface = wifi_device.ip_interface().ok()?;
            let is_connected = wifi_device
                .active_access_point()
                .ok()
                .and_then(|ap| ap.ssid().ok())
                .is_some_and(|ssid| !ssid.is_empty());

            if is_connected {
                connected = Some(iface.clone());
            }
            available.push(iface);
        }
    }

    choose_wifi_adapter(connected, available)
}

fn get_wifi_adapter_info_via_nmcli() -> Option<String> {
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

pub async fn get_wifi_adapter_info() -> Option<String> {
    get_wifi_adapter_info_via_nm().or_else(get_wifi_adapter_info_via_nmcli)
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

fn nm_wifi_proxy(
    dbus: &dbus::blocking::Connection,
) -> dbus::blocking::Proxy<'_, &dbus::blocking::Connection> {
    dbus.with_proxy(
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        Duration::from_secs(10),
    )
}

fn variant<T: RefArg + 'static>(value: T) -> Variant<Box<dyn RefArg>> {
    Variant(Box::new(value))
}

fn base_connection_settings(ssid: &str) -> HashMap<&'static str, PropMap> {
    let mut connection = PropMap::new();
    connection.insert("type".to_string(), variant("802-11-wireless".to_string()));
    connection.insert("id".to_string(), variant(format!("nm-wifi-{ssid}")));

    let mut wireless = PropMap::new();
    wireless.insert("ssid".to_string(), variant(ssid.as_bytes().to_vec()));
    wireless.insert("mode".to_string(), variant("infrastructure".to_string()));

    let mut ipv4 = PropMap::new();
    ipv4.insert("method".to_string(), variant("auto".to_string()));

    let mut ipv6 = PropMap::new();
    ipv6.insert("method".to_string(), variant("auto".to_string()));

    let mut settings = HashMap::new();
    settings.insert("connection", connection);
    settings.insert("802-11-wireless", wireless);
    settings.insert("ipv4", ipv4);
    settings.insert("ipv6", ipv6);
    settings
}

fn open_network_connection_settings(ssid: &str) -> HashMap<&'static str, PropMap> {
    base_connection_settings(ssid)
}

fn secured_network_connection_settings(ssid: &str, password: &str) -> HashMap<&'static str, PropMap> {
    let mut settings = base_connection_settings(ssid);

    let mut wireless_security = PropMap::new();
    wireless_security.insert("key-mgmt".to_string(), variant("wpa-psk".to_string()));
    wireless_security.insert("psk".to_string(), variant(password.to_string()));

    if let Some(wireless) = settings.get_mut("802-11-wireless") {
        wireless.insert(
            "security".to_string(),
            variant("802-11-wireless-security".to_string()),
        );
    }

    settings.insert("802-11-wireless-security", wireless_security);
    settings
}

fn connect_via_networkmanager(
    settings: HashMap<&'static str, PropMap>,
) -> Result<bool, Box<dyn Error>> {
    let adapter = match get_wifi_adapter_info_via_nm() {
        Some(adapter) => adapter,
        None => return Ok(false),
    };

    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|_| "Failed to connect to D-Bus".to_string())?;
    let proxy = nm_wifi_proxy(&dbus);

    let (device_path,): (dbus::Path<'static>,) = proxy
        .method_call(
            "org.freedesktop.NetworkManager",
            "GetDeviceByIpIface",
            (adapter.as_str(),),
        )
        .map_err(|_| "Failed to find WiFi device in NetworkManager".to_string())?;

    let specific_object = dbus::Path::from("/");
    let result: Result<(dbus::Path<'static>, dbus::Path<'static>), dbus::Error> = proxy.method_call(
        "org.freedesktop.NetworkManager",
        "AddAndActivateConnection",
        (settings, device_path, specific_object),
    );

    match result {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

fn connect_open_network_via_networkmanager(network: &WifiNetwork) -> Result<bool, Box<dyn Error>> {
    connect_via_networkmanager(open_network_connection_settings(&network.ssid))
}

fn connect_psk_network_via_networkmanager(
    network: &WifiNetwork,
    password: &str,
) -> Result<bool, Box<dyn Error>> {
    connect_via_networkmanager(secured_network_connection_settings(&network.ssid, password))
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
    if matches!(
        connect_backend_for(network, password),
        ConnectionBackend::NetworkManager
    ) {
        match classify_security(network, password) {
            SecurityKind::Open if connect_open_network_via_networkmanager(network)? => {
                return Ok(());
            }
            SecurityKind::WpaPsk => {
                if let Some(password) = password
                    && connect_psk_network_via_networkmanager(network, password)?
                {
                    return Ok(());
                }
            }
            SecurityKind::Unsupported => {}
            SecurityKind::Open => {}
        }
    }

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

fn disconnect_via_nmcli(adapter: &str) -> Result<(), Box<dyn Error>> {
    let output = Command::new("nmcli")
        .args(["device", "disconnect", adapter])
        .output()
        .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("nmcli disconnect failed: {}", error_msg).into())
    }
}

fn disconnect_via_networkmanager(network: &WifiNetwork) -> Result<bool, Box<dyn Error>> {
    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|_| "Failed to connect to D-Bus".to_string())?;
    let nm = NetworkManager::new(&dbus);

    for device in nm
        .get_devices()
        .map_err(|_| "Failed to get devices".to_string())?
    {
        if let Device::WiFi(wifi_device) = device {
            let active_ssid = wifi_device
                .active_access_point()
                .ok()
                .and_then(|ap| ap.ssid().ok());

            if should_disconnect_device(active_ssid.as_deref(), &network.ssid) {
                wifi_device
                    .disconnect()
                    .map_err(|_| "Failed to disconnect device via NetworkManager".to_string())?;
                return Ok(true);
            }
        }
    }

    Ok(false)
}

pub async fn disconnect_from_network(network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
    match disconnect_backend_for() {
        ConnectionBackend::NetworkManager => {
            if disconnect_via_networkmanager(network)? {
                return Ok(());
            }

            let adapter = get_wifi_adapter_info()
                .await
                .ok_or_else(|| "Failed to find connected WiFi adapter".to_string())?;
            disconnect_via_nmcli(&adapter)
        }
        ConnectionBackend::NmcliFallback => {
            let adapter = get_wifi_adapter_info()
                .await
                .ok_or_else(|| "Failed to find connected WiFi adapter".to_string())?;
            disconnect_via_nmcli(&adapter)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConnectionBackend,
        SecurityKind,
        choose_wifi_adapter,
        classify_security,
        connect_backend_for,
        connect_command_args,
        disconnect_backend_for,
        open_network_connection_settings,
        parse_any_wifi_device,
        parse_connected_wifi_device,
        secured_network_connection_settings,
        should_disconnect_device,
    };
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
    fn adapter_selection_prefers_connected_wifi_interfaces() {
        assert_eq!(
            choose_wifi_adapter(
                Some("wlp2s0".to_string()),
                vec!["wlan1".to_string(), "wlp2s0".to_string()]
            ),
            Some("wlp2s0".to_string())
        );
    }

    #[test]
    fn adapter_selection_falls_back_to_first_available_wifi_interface() {
        assert_eq!(
            choose_wifi_adapter(None, vec!["wlan1".to_string(), "wlp2s0".to_string()]),
            Some("wlan1".to_string())
        );
    }

    #[test]
    fn disconnect_matching_requires_the_selected_ssid() {
        assert!(should_disconnect_device(Some("home"), "home"));
        assert!(!should_disconnect_device(Some("guest"), "home"));
        assert!(!should_disconnect_device(None, "home"));
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

    #[test]
    fn open_networks_prefer_networkmanager() {
        let network = WifiNetwork {
            ssid: "cafe".to_string(),
            signal_strength: 60,
            secured: false,
            frequency: 2412,
            connected: false,
        };

        assert_eq!(classify_security(&network, None), SecurityKind::Open);
        assert_eq!(
            connect_backend_for(&network, None),
            ConnectionBackend::NetworkManager
        );
    }

    #[test]
    fn open_network_settings_include_wireless_and_ip_defaults() {
        let settings = open_network_connection_settings("cafe");

        assert!(settings.contains_key("connection"));
        assert!(settings.contains_key("802-11-wireless"));
        assert!(settings.contains_key("ipv4"));
        assert!(settings.contains_key("ipv6"));
    }

    #[test]
    fn psk_network_settings_include_wireless_security() {
        let settings = secured_network_connection_settings("home", "hunter2");

        assert!(settings.contains_key("802-11-wireless-security"));
        assert_eq!(
            settings
                .get("802-11-wireless")
                .and_then(|wireless| wireless.get("security"))
                .and_then(|value| value.0.as_str()),
            Some("802-11-wireless-security")
        );
    }

    #[test]
    fn psk_networks_prefer_networkmanager_when_password_is_present() {
        let network = WifiNetwork {
            ssid: "home".to_string(),
            signal_strength: 80,
            secured: true,
            frequency: 5180,
            connected: false,
        };

        assert_eq!(
            classify_security(&network, Some("hunter2")),
            SecurityKind::WpaPsk
        );
        assert_eq!(
            connect_backend_for(&network, Some("hunter2")),
            ConnectionBackend::NetworkManager
        );
    }

    #[test]
    fn unsupported_connect_cases_fall_back_explicitly() {
        let network = WifiNetwork {
            ssid: "corp".to_string(),
            signal_strength: 70,
            secured: true,
            frequency: 5200,
            connected: false,
        };

        assert_eq!(classify_security(&network, None), SecurityKind::Unsupported);
        assert_eq!(
            connect_backend_for(&network, None),
            ConnectionBackend::NmcliFallback
        );
    }

    #[test]
    fn disconnects_prefer_networkmanager() {
        assert_eq!(disconnect_backend_for(), ConnectionBackend::NetworkManager);
    }
}
