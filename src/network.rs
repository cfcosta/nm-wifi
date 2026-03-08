use std::{collections::HashMap, error::Error, time::Duration};

use dbus::arg::{PropMap, RefArg, Variant};
use networkmanager::{
    NetworkManager,
    devices::{Any, Device, Wireless},
};
use tokio::time::sleep;

use crate::types::{WifiNetwork, WifiSecurity};

const AP_FLAGS_PRIVACY: u32 = 0x1;
const AP_SEC_KEY_MGMT_PSK: u32 = 0x100;
const AP_SEC_KEY_MGMT_8021X: u32 = 0x200;
const AP_SEC_KEY_MGMT_SAE: u32 = 0x400;
const AP_SEC_KEY_MGMT_OWE: u32 = 0x800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecurityKind {
    Open,
    WpaPsk,
    WpaSae,
    Unsupported,
}

fn classify_access_point_security(flags: u32, wpa_flags: u32, rsn_flags: u32) -> WifiSecurity {
    let key_mgmt_flags = wpa_flags | rsn_flags;

    if key_mgmt_flags & AP_SEC_KEY_MGMT_SAE != 0 {
        WifiSecurity::WpaSae
    } else if key_mgmt_flags & AP_SEC_KEY_MGMT_PSK != 0 {
        WifiSecurity::WpaPsk
    } else if key_mgmt_flags & AP_SEC_KEY_MGMT_8021X != 0 {
        WifiSecurity::Enterprise
    } else if key_mgmt_flags & AP_SEC_KEY_MGMT_OWE != 0 || flags & AP_FLAGS_PRIVACY != 0 {
        WifiSecurity::Unsupported
    } else {
        WifiSecurity::Open
    }
}

fn classify_security(network: &WifiNetwork, password: Option<&str>) -> SecurityKind {
    match (network.security, password) {
        (WifiSecurity::Open, _) => SecurityKind::Open,
        (WifiSecurity::WpaPsk, Some(_)) => SecurityKind::WpaPsk,
        (WifiSecurity::WpaSae, Some(_)) => SecurityKind::WpaSae,
        _ => SecurityKind::Unsupported,
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
            let iface = wifi_device.interface().ok()?;
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

                    let security = classify_access_point_security(flags, wpa_flags, rsn_flags);

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
                        security,
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

fn secured_network_connection_settings(
    ssid: &str,
    password: &str,
    key_mgmt: &str,
) -> HashMap<&'static str, PropMap> {
    let mut settings = base_connection_settings(ssid);

    let mut wireless_security = PropMap::new();
    wireless_security.insert("key-mgmt".to_string(), variant(key_mgmt.to_string()));
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
    connect_via_networkmanager(secured_network_connection_settings(
        &network.ssid,
        password,
        "wpa-psk",
    ))
}

fn connect_sae_network_via_networkmanager(
    network: &WifiNetwork,
    password: &str,
) -> Result<bool, Box<dyn Error>> {
    connect_via_networkmanager(secured_network_connection_settings(
        &network.ssid,
        password,
        "sae",
    ))
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
                Err("NetworkManager failed to activate WPA/WPA2 Personal network".into())
            }
        }
        SecurityKind::WpaSae => {
            let password = password.ok_or("Password required for secured network")?;
            if connect_sae_network_via_networkmanager(network, password)? {
                Ok(())
            } else {
                Err("NetworkManager failed to activate WPA3 Personal network".into())
            }
        }
        SecurityKind::Unsupported => Err(format!(
            "Unsupported network security for NetworkManager activation: {}",
            network.security.display_name()
        )
        .into()),
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
        AP_FLAGS_PRIVACY,
        AP_SEC_KEY_MGMT_8021X,
        AP_SEC_KEY_MGMT_PSK,
        AP_SEC_KEY_MGMT_SAE,
        SecurityKind,
        choose_wifi_adapter,
        classify_access_point_security,
        classify_security,
        open_network_connection_settings,
        scan_wait_duration,
        secured_network_connection_settings,
        should_disconnect_device,
    };
    use crate::types::{WifiNetwork, WifiSecurity};

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

    fn network(security: WifiSecurity) -> WifiNetwork {
        WifiNetwork {
            ssid: "test".to_string(),
            signal_strength: 60,
            security,
            frequency: 2412,
            connected: false,
        }
    }

    #[test]
    fn open_networks_are_classified_as_open() {
        assert_eq!(
            classify_security(&network(WifiSecurity::Open), None),
            SecurityKind::Open
        );
    }

    #[test]
    fn access_points_with_psk_flags_are_classified_as_wpa_personal() {
        assert_eq!(
            classify_access_point_security(0, 0, AP_SEC_KEY_MGMT_PSK),
            WifiSecurity::WpaPsk
        );
    }

    #[test]
    fn access_points_with_sae_flags_are_classified_as_wpa3_personal() {
        assert_eq!(
            classify_access_point_security(0, 0, AP_SEC_KEY_MGMT_SAE),
            WifiSecurity::WpaSae
        );
    }

    #[test]
    fn enterprise_access_points_are_not_treated_as_personal_networks() {
        assert_eq!(
            classify_access_point_security(0, 0, AP_SEC_KEY_MGMT_8021X),
            WifiSecurity::Enterprise
        );
    }

    #[test]
    fn privacy_without_supported_key_management_is_unsupported() {
        assert_eq!(
            classify_access_point_security(AP_FLAGS_PRIVACY, 0, 0),
            WifiSecurity::Unsupported
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
        let settings = secured_network_connection_settings("home", "hunter2", "wpa-psk");

        assert!(settings.contains_key("802-11-wireless-security"));
        assert_eq!(
            settings
                .get("802-11-wireless")
                .and_then(|wireless| wireless.get("security"))
                .and_then(|value| value.0.as_str()),
            Some("802-11-wireless-security")
        );
        assert_eq!(
            settings
                .get("802-11-wireless-security")
                .and_then(|security| security.get("key-mgmt"))
                .and_then(|value| value.0.as_str()),
            Some("wpa-psk")
        );
    }

    #[test]
    fn sae_network_settings_use_sae_key_management() {
        let settings = secured_network_connection_settings("home", "hunter2", "sae");

        assert_eq!(
            settings
                .get("802-11-wireless-security")
                .and_then(|security| security.get("key-mgmt"))
                .and_then(|value| value.0.as_str()),
            Some("sae")
        );
    }

    #[test]
    fn psk_networks_are_classified_when_password_is_present() {
        assert_eq!(
            classify_security(&network(WifiSecurity::WpaPsk), Some("hunter2")),
            SecurityKind::WpaPsk
        );
    }

    #[test]
    fn sae_networks_are_classified_when_password_is_present() {
        assert_eq!(
            classify_security(&network(WifiSecurity::WpaSae), Some("hunter2")),
            SecurityKind::WpaSae
        );
    }

    #[test]
    fn enterprise_networks_remain_unsupported_even_with_a_password() {
        assert_eq!(
            classify_security(
                &network(WifiSecurity::Enterprise),
                Some("correcthorsebatterystaple")
            ),
            SecurityKind::Unsupported
        );
    }

    #[test]
    fn unsupported_connect_cases_are_detected() {
        assert_eq!(
            classify_security(&network(WifiSecurity::Unsupported), None),
            SecurityKind::Unsupported
        );
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
