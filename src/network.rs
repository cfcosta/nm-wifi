#[cfg(any(test, not(feature = "demo")))]
use std::collections::HashMap;
use std::error::Error;
#[cfg(not(feature = "demo"))]
use std::io;
#[cfg(not(feature = "demo"))]
use std::time::Duration;

#[cfg(any(test, not(feature = "demo")))]
use dbus::arg::{PropMap, RefArg, Variant};
#[cfg(not(feature = "demo"))]
use networkmanager::{
    NetworkManager,
    devices::{Any, Device, Wireless},
};
#[cfg(not(feature = "demo"))]
use tokio::time::sleep;

use crate::types::{WifiNetwork, WifiSecurity};

#[cfg(not(feature = "demo"))]
fn contextual_error(context: &str, error: impl std::fmt::Display) -> Box<dyn Error> {
    io::Error::other(format!("{context}: {error}")).into()
}

#[cfg(feature = "demo")]
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

#[cfg(feature = "demo")]
fn demo_connect(request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
    let (network, password) = match request {
        ConnectionRequest::Open { network } => (network, None),
        ConnectionRequest::Secured { network, password } => (network, Some(password)),
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

pub enum ConnectionRequest<'a> {
    Open {
        network: &'a WifiNetwork,
    },
    Secured {
        network: &'a WifiNetwork,
        password: &'a str,
    },
}

#[cfg(not(feature = "demo"))]
impl ConnectionRequest<'_> {
    fn network(&self) -> &WifiNetwork {
        match self {
            Self::Open { network } | Self::Secured { network, .. } => network,
        }
    }
}

#[cfg(not(feature = "demo"))]
const AP_FLAGS_PRIVACY: u32 = 0x1;
#[cfg(not(feature = "demo"))]
const AP_SEC_KEY_MGMT_PSK: u32 = 0x100;
#[cfg(not(feature = "demo"))]
const AP_SEC_KEY_MGMT_8021X: u32 = 0x200;
#[cfg(not(feature = "demo"))]
const AP_SEC_KEY_MGMT_SAE: u32 = 0x400;
#[cfg(not(feature = "demo"))]
const AP_SEC_KEY_MGMT_OWE: u32 = 0x800;

#[cfg(not(feature = "demo"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecurityKind {
    Open,
    WpaPsk,
    WpaSae,
    Unsupported,
}

#[cfg(not(feature = "demo"))]
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

#[cfg(not(feature = "demo"))]
fn classify_security(network: &WifiNetwork, password: Option<&str>) -> SecurityKind {
    match (network.security, password) {
        (WifiSecurity::Open, _) => SecurityKind::Open,
        (WifiSecurity::WpaPsk, Some(_)) => SecurityKind::WpaPsk,
        (WifiSecurity::WpaSae, Some(_)) => SecurityKind::WpaSae,
        _ => SecurityKind::Unsupported,
    }
}

#[cfg(not(feature = "demo"))]
fn should_disconnect_device(active_ssid: Option<&str>, target_ssid: &str) -> bool {
    active_ssid == Some(target_ssid)
}

#[cfg(not(feature = "demo"))]
fn get_connected_ssid_via_nm() -> Result<Option<String>, Box<dyn Error>> {
    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|error| contextual_error("Failed to connect to D-Bus", error))?;
    let nm = NetworkManager::new(&dbus);
    let devices = nm
        .get_devices()
        .map_err(|error| contextual_error("Failed to list NetworkManager devices", error))?;

    for device in devices {
        if let Device::WiFi(wifi_device) = device {
            let access_point = match wifi_device.active_access_point() {
                Ok(access_point) => access_point,
                Err(_) => continue,
            };
            let ssid = access_point
                .ssid()
                .map_err(|error| contextual_error("Failed to read active WiFi SSID", error))?;
            if !ssid.is_empty() {
                return Ok(Some(ssid));
            }
        }
    }

    Ok(None)
}

#[cfg(feature = "demo")]
pub fn get_connected_ssid() -> Result<Option<String>, Box<dyn Error>> {
    Ok(demo_networks()
        .into_iter()
        .find(|network| network.connected)
        .map(|network| network.ssid))
}

#[cfg(not(feature = "demo"))]
pub fn get_connected_ssid() -> Result<Option<String>, Box<dyn Error>> {
    get_connected_ssid_via_nm()
}

#[cfg(not(feature = "demo"))]
fn choose_wifi_adapter_name(connected: Option<String>, available: Vec<String>) -> Option<String> {
    connected.or_else(|| available.into_iter().next())
}

#[cfg(not(feature = "demo"))]
fn get_wifi_adapter_name_via_nm() -> Result<Option<String>, Box<dyn Error>> {
    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|error| contextual_error("Failed to connect to D-Bus", error))?;
    let nm = NetworkManager::new(&dbus);
    let devices = nm
        .get_devices()
        .map_err(|error| contextual_error("Failed to list NetworkManager devices", error))?;
    let mut connected = None;
    let mut available = Vec::new();

    for device in devices {
        if let Device::WiFi(wifi_device) = device {
            let iface = wifi_device
                .interface()
                .map_err(|error| contextual_error("Failed to read WiFi interface name", error))?;
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

    Ok(choose_wifi_adapter_name(connected, available))
}

#[cfg(feature = "demo")]
pub fn get_wifi_adapter_name() -> Result<Option<String>, Box<dyn Error>> {
    Ok(Some("demo-wlan0".to_string()))
}

#[cfg(not(feature = "demo"))]
pub fn get_wifi_adapter_name() -> Result<Option<String>, Box<dyn Error>> {
    get_wifi_adapter_name_via_nm()
}

#[cfg(not(feature = "demo"))]
fn scan_wait_duration(last_scan_delta_ms: i64) -> Duration {
    if (0..15_000).contains(&last_scan_delta_ms) {
        Duration::from_millis(0)
    } else {
        Duration::from_millis(750)
    }
}

#[cfg(feature = "demo")]
pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, Box<dyn Error>> {
    Ok(demo_networks())
}

#[cfg(not(feature = "demo"))]
pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, Box<dyn Error>> {
    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|error| contextual_error("Failed to connect to D-Bus", error))?;
    let nm = NetworkManager::new(&dbus);

    let connected_ssid = get_connected_ssid()?;

    let devices = nm
        .get_devices()
        .map_err(|error| contextual_error("Failed to list NetworkManager devices", error))?;

    for device in devices {
        if let Device::WiFi(wifi_device) = device {
            let last_scan_before_request = wifi_device.last_scan().unwrap_or(0);

            wifi_device
                .request_scan(HashMap::new())
                .map_err(|error| contextual_error("Failed to request WiFi scan", error))?;

            let last_scan_after_request = wifi_device.last_scan().unwrap_or(last_scan_before_request);
            let wait_duration =
                scan_wait_duration(last_scan_after_request - last_scan_before_request);
            if !wait_duration.is_zero() {
                sleep(wait_duration).await;
            }

            let access_points = wifi_device
                .get_all_access_points()
                .map_err(|error| contextual_error("Failed to list WiFi access points", error))?;

            let mut networks = Vec::new();

            for ap in access_points {
                let ssid = ap
                    .ssid()
                    .map_err(|error| contextual_error("Failed to read access point SSID", error))?;
                if !ssid.is_empty() {
                    let flags = ap.flags().map_err(|error| {
                        contextual_error("Failed to read access point flags", error)
                    })?;
                    let wpa_flags = ap.wpa_flags().map_err(|error| {
                        contextual_error("Failed to read WPA capabilities", error)
                    })?;
                    let rsn_flags = ap.rsn_flags().map_err(|error| {
                        contextual_error("Failed to read RSN capabilities", error)
                    })?;

                    let security = classify_access_point_security(flags, wpa_flags, rsn_flags);

                    let signal_strength = ap
                        .strength()
                        .map_err(|error| contextual_error("Failed to read signal strength", error))?;

                    let frequency = ap
                        .frequency()
                        .map_err(|error| contextual_error("Failed to read WiFi frequency", error))?;

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

#[cfg(not(feature = "demo"))]
fn nm_wifi_proxy(
    dbus: &dbus::blocking::Connection,
) -> dbus::blocking::Proxy<'_, &dbus::blocking::Connection> {
    dbus.with_proxy(
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        Duration::from_secs(10),
    )
}

#[cfg(any(test, not(feature = "demo")))]
fn variant<T: RefArg + 'static>(value: T) -> Variant<Box<dyn RefArg>> {
    Variant(Box::new(value))
}

#[cfg(any(test, not(feature = "demo")))]
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

#[cfg(any(test, not(feature = "demo")))]
fn open_network_connection_settings(ssid: &str) -> HashMap<&'static str, PropMap> {
    base_connection_settings(ssid)
}

#[cfg(any(test, not(feature = "demo")))]
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

#[cfg(not(feature = "demo"))]
fn connect_via_networkmanager(
    settings: HashMap<&'static str, PropMap>,
) -> Result<(), Box<dyn Error>> {
    let adapter = get_wifi_adapter_name_via_nm()?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "No WiFi adapter was found in NetworkManager",
        )
    })?;

    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|error| contextual_error("Failed to connect to D-Bus", error))?;
    let proxy = nm_wifi_proxy(&dbus);

    let (device_path,): (dbus::Path<'static>,) = proxy
        .method_call(
            "org.freedesktop.NetworkManager",
            "GetDeviceByIpIface",
            (adapter.as_str(),),
        )
        .map_err(|error| contextual_error("Failed to find WiFi device in NetworkManager", error))?;

    let specific_object = dbus::Path::from("/");
    let _: (dbus::Path<'static>, dbus::Path<'static>) = proxy
        .method_call(
            "org.freedesktop.NetworkManager",
            "AddAndActivateConnection",
            (settings, device_path, specific_object),
        )
        .map_err(|error| {
            contextual_error(
                "NetworkManager failed to activate the WiFi connection",
                error,
            )
        })?;

    Ok(())
}

#[cfg(feature = "demo")]
pub fn connect_to_network(request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
    demo_connect(request)
}

#[cfg(not(feature = "demo"))]
pub fn connect_to_network(request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
    let network = request.network();

    match request {
        ConnectionRequest::Open { .. } => {
            if network.security != WifiSecurity::Open {
                return Err("Password required for secured network".into());
            }
            connect_via_networkmanager(open_network_connection_settings(&network.ssid))
        }
        ConnectionRequest::Secured { password, .. } => {
            match classify_security(network, Some(password)) {
                SecurityKind::WpaPsk => connect_via_networkmanager(
                    secured_network_connection_settings(&network.ssid, password, "wpa-psk"),
                ),
                SecurityKind::WpaSae => connect_via_networkmanager(
                    secured_network_connection_settings(&network.ssid, password, "sae"),
                ),
                SecurityKind::Open => {
                    Err("Open networks should not be activated with a password request".into())
                }
                SecurityKind::Unsupported => Err(format!(
                    "Unsupported network security for NetworkManager activation: {}",
                    network.security.display_name()
                )
                .into()),
            }
        }
    }
}

#[cfg(not(feature = "demo"))]
fn disconnect_via_networkmanager(network: &WifiNetwork) -> Result<bool, Box<dyn Error>> {
    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|error| contextual_error("Failed to connect to D-Bus", error))?;
    let nm = NetworkManager::new(&dbus);

    for device in nm
        .get_devices()
        .map_err(|error| contextual_error("Failed to list NetworkManager devices", error))?
    {
        if let Device::WiFi(wifi_device) = device {
            let active_ssid = wifi_device
                .active_access_point()
                .ok()
                .and_then(|ap| ap.ssid().ok());

            if should_disconnect_device(active_ssid.as_deref(), &network.ssid) {
                wifi_device.disconnect().map_err(|error| {
                    contextual_error("Failed to disconnect device via NetworkManager", error)
                })?;
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(feature = "demo")]
pub fn disconnect_from_network(network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
    if network.connected {
        Ok(())
    } else {
        Err("Demo mode: selected network is not connected".into())
    }
}

#[cfg(not(feature = "demo"))]
pub fn disconnect_from_network(network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
    if disconnect_via_networkmanager(network)? {
        Ok(())
    } else {
        Err("NetworkManager could not find a matching active WiFi device to disconnect".into())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "demo"))]
    use std::time::Duration;

    #[cfg(not(feature = "demo"))]
    use super::{
        AP_FLAGS_PRIVACY,
        AP_SEC_KEY_MGMT_8021X,
        AP_SEC_KEY_MGMT_PSK,
        AP_SEC_KEY_MGMT_SAE,
        SecurityKind,
        choose_wifi_adapter_name,
        classify_access_point_security,
        classify_security,
        scan_wait_duration,
        should_disconnect_device,
    };
    #[cfg(feature = "demo")]
    use super::{ConnectionRequest, connect_to_network, demo_networks, scan_wifi_networks};
    use super::{open_network_connection_settings, secured_network_connection_settings};
    #[cfg(not(feature = "demo"))]
    use crate::types::WifiNetwork;
    use crate::types::WifiSecurity;

    #[cfg(not(feature = "demo"))]
    #[test]
    fn adapter_selection_prefers_connected_wifi_interfaces() {
        assert_eq!(
            choose_wifi_adapter_name(
                Some("wlp2s0".to_string()),
                vec!["wlan1".to_string(), "wlp2s0".to_string()]
            ),
            Some("wlp2s0".to_string())
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn adapter_selection_falls_back_to_first_available_wifi_interface() {
        assert_eq!(
            choose_wifi_adapter_name(None, vec!["wlan1".to_string(), "wlp2s0".to_string()]),
            Some("wlan1".to_string())
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn disconnect_matching_requires_the_selected_ssid() {
        assert!(should_disconnect_device(Some("home"), "home"));
        assert!(!should_disconnect_device(Some("guest"), "home"));
        assert!(!should_disconnect_device(None, "home"));
    }

    #[cfg(not(feature = "demo"))]
    fn network(security: WifiSecurity) -> WifiNetwork {
        WifiNetwork {
            ssid: "test".to_string(),
            signal_strength: 60,
            security,
            frequency: 2412,
            connected: false,
        }
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn open_networks_are_classified_as_open() {
        assert_eq!(
            classify_security(&network(WifiSecurity::Open), None),
            SecurityKind::Open
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn access_points_with_psk_flags_are_classified_as_wpa_personal() {
        assert_eq!(
            classify_access_point_security(0, 0, AP_SEC_KEY_MGMT_PSK),
            WifiSecurity::WpaPsk
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn access_points_with_sae_flags_are_classified_as_wpa3_personal() {
        assert_eq!(
            classify_access_point_security(0, 0, AP_SEC_KEY_MGMT_SAE),
            WifiSecurity::WpaSae
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn enterprise_access_points_are_not_treated_as_personal_networks() {
        assert_eq!(
            classify_access_point_security(0, 0, AP_SEC_KEY_MGMT_8021X),
            WifiSecurity::Enterprise
        );
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn psk_networks_are_classified_when_password_is_present() {
        assert_eq!(
            classify_security(&network(WifiSecurity::WpaPsk), Some("hunter2")),
            SecurityKind::WpaPsk
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn sae_networks_are_classified_when_password_is_present() {
        assert_eq!(
            classify_security(&network(WifiSecurity::WpaSae), Some("hunter2")),
            SecurityKind::WpaSae
        );
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn unsupported_connect_cases_are_detected() {
        assert_eq!(
            classify_security(&network(WifiSecurity::Unsupported), None),
            SecurityKind::Unsupported
        );
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn recent_scans_do_not_force_an_extra_wait() {
        assert_eq!(scan_wait_duration(5_000), Duration::from_millis(0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn stale_scans_wait_longer_than_the_old_fixed_delay() {
        assert_eq!(scan_wait_duration(20_000), Duration::from_millis(750));
        assert_eq!(scan_wait_duration(-1), Duration::from_millis(750));
    }

    #[cfg(feature = "demo")]
    #[tokio::test]
    async fn demo_scan_returns_mock_networks() {
        let networks = scan_wifi_networks().await.expect("demo scan works");
        assert!(networks.iter().any(|network| network.ssid == "CatCat"));
        assert!(
            networks
                .iter()
                .any(|network| network.security == WifiSecurity::WpaSae)
        );
    }

    #[cfg(feature = "demo")]
    #[test]
    fn demo_connect_accepts_matching_passwords() {
        let network = demo_networks()
            .into_iter()
            .find(|network| network.ssid == "CatCat")
            .expect("demo network exists");

        let result = connect_to_network(ConnectionRequest::Secured {
            network: &network,
            password: "AcerolaAcai",
        });

        assert!(result.is_ok());
    }

    #[cfg(feature = "demo")]
    #[test]
    fn demo_connect_rejects_invalid_passwords() {
        let network = demo_networks()
            .into_iter()
            .find(|network| network.ssid == "CatCat")
            .expect("demo network exists");

        let result = connect_to_network(ConnectionRequest::Secured {
            network: &network,
            password: "wrong-password",
        });

        assert_eq!(
            result.expect_err("demo connect should fail").to_string(),
            "Demo mode: invalid password"
        );
    }
}
