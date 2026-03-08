use std::{collections::HashMap, error::Error, time::Duration};

use dbus::arg::{PropMap, RefArg, Variant};
use networkmanager::{
    NetworkManager,
    devices::{Any, Device, Wireless},
};
use tokio::time::sleep;

use crate::types::WifiNetwork;

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

pub async fn get_connected_ssid() -> Option<String> {
    get_connected_ssid_via_nm()
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

pub async fn get_wifi_adapter_info() -> Option<String> {
    get_wifi_adapter_info_via_nm()
}

fn scan_wait_duration(last_scan_delta_ms: i64) -> Duration {
    if (0..15_000).contains(&last_scan_delta_ms) {
        Duration::from_millis(0)
    } else {
        Duration::from_millis(750)
    }
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
            let last_scan_before_request = wifi_device.last_scan().unwrap_or(0);

            wifi_device
                .request_scan(HashMap::new())
                .map_err(|_| "Failed to request scan".to_string())?;

            let last_scan_after_request = wifi_device.last_scan().unwrap_or(last_scan_before_request);
            let wait_duration =
                scan_wait_duration(last_scan_after_request - last_scan_before_request);
            if !wait_duration.is_zero() {
                sleep(wait_duration).await;
            }

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

pub async fn connect_to_network(
    network: &WifiNetwork,
    password: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    match classify_security(network, password) {
        SecurityKind::Open => {
            if connect_open_network_via_networkmanager(network)? {
                Ok(())
            } else {
                Err("NetworkManager failed to activate open network".into())
            }
        }
        SecurityKind::WpaPsk => {
            let password = password.ok_or("Password required for secured network")?;
            if connect_psk_network_via_networkmanager(network, password)? {
                Ok(())
            } else {
                Err("NetworkManager failed to activate WPA-PSK network".into())
            }
        }
        SecurityKind::Unsupported => {
            Err("Unsupported network security for NetworkManager activation".into())
        }
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
    if disconnect_via_networkmanager(network)? {
        Ok(())
    } else {
        Err("NetworkManager could not find a matching active WiFi device to disconnect".into())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        SecurityKind,
        choose_wifi_adapter,
        classify_security,
        open_network_connection_settings,
        scan_wait_duration,
        secured_network_connection_settings,
        should_disconnect_device,
    };
    use crate::types::WifiNetwork;

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
    fn open_networks_are_classified_as_open() {
        let network = WifiNetwork {
            ssid: "cafe".to_string(),
            signal_strength: 60,
            secured: false,
            frequency: 2412,
            connected: false,
        };

        assert_eq!(classify_security(&network, None), SecurityKind::Open);
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
    fn psk_networks_are_classified_when_password_is_present() {
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
    }

    #[test]
    fn unsupported_connect_cases_are_detected() {
        let network = WifiNetwork {
            ssid: "corp".to_string(),
            signal_strength: 70,
            secured: true,
            frequency: 5200,
            connected: false,
        };

        assert_eq!(classify_security(&network, None), SecurityKind::Unsupported);
    }

    #[test]
    fn recent_scans_do_not_force_an_extra_wait() {
        assert_eq!(scan_wait_duration(5_000), Duration::from_millis(0));
    }

    #[test]
    fn stale_scans_wait_longer_than_the_old_fixed_delay() {
        assert_eq!(scan_wait_duration(20_000), Duration::from_millis(750));
        assert_eq!(scan_wait_duration(-1), Duration::from_millis(750));
    }
}
