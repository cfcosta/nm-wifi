use std::{collections::HashMap, error::Error, io, time::Duration};

use dbus::arg::PropMap;
use networkmanager::{
    NetworkManager,
    devices::{Any, Device, Wireless},
};
use tokio::time::sleep;

use crate::{
    network::{
        ConnectionRequest,
        open_network_connection_settings,
        secured_network_connection_settings,
    },
    wifi::{WifiNetwork, WifiSecurity},
};

pub(crate) const AP_FLAGS_PRIVACY: u32 = 0x1;
pub(crate) const AP_SEC_KEY_MGMT_PSK: u32 = 0x100;
pub(crate) const AP_SEC_KEY_MGMT_8021X: u32 = 0x200;
pub(crate) const AP_SEC_KEY_MGMT_SAE: u32 = 0x400;
const AP_SEC_KEY_MGMT_OWE: u32 = 0x800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecurityKind {
    Open,
    WpaPsk,
    WpaSae,
    Unsupported,
}

fn contextual_error(context: &str, error: impl std::fmt::Display) -> Box<dyn Error> {
    io::Error::other(format!("{context}: {error}")).into()
}

pub(crate) fn classify_access_point_security(
    flags: u32,
    wpa_flags: u32,
    rsn_flags: u32,
) -> WifiSecurity {
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

pub(crate) fn classify_security(network: &WifiNetwork, password: Option<&str>) -> SecurityKind {
    match (network.security, password) {
        (WifiSecurity::Open, _) => SecurityKind::Open,
        (WifiSecurity::WpaPsk, Some(_)) => SecurityKind::WpaPsk,
        (WifiSecurity::WpaSae, Some(_)) => SecurityKind::WpaSae,
        _ => SecurityKind::Unsupported,
    }
}

pub(crate) fn should_disconnect_device(active_ssid: Option<&str>, target_ssid: &str) -> bool {
    active_ssid == Some(target_ssid)
}

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

pub fn get_connected_ssid() -> Result<Option<String>, Box<dyn Error>> {
    get_connected_ssid_via_nm()
}

pub(crate) fn choose_wifi_adapter_name(
    connected: Option<String>,
    available: Vec<String>,
) -> Option<String> {
    connected.or_else(|| available.into_iter().next())
}

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

pub fn get_wifi_adapter_name() -> Result<Option<String>, Box<dyn Error>> {
    get_wifi_adapter_name_via_nm()
}

pub(crate) fn scan_wait_duration(last_scan_delta_ms: i64) -> Duration {
    if (0..15_000).contains(&last_scan_delta_ms) {
        Duration::from_millis(0)
    } else {
        Duration::from_millis(750)
    }
}

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

                    networks.push(WifiNetwork {
                        ssid,
                        signal_strength,
                        security,
                        frequency,
                        connected,
                    });
                }
            }

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

pub fn connect_to_network(request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
    let network = match &request {
        ConnectionRequest::Open { network } | ConnectionRequest::Secured { network, .. } => *network,
    };

    match request {
        ConnectionRequest::Open { .. } => {
            if network.security != WifiSecurity::Open {
                return Err("Password required for secured network".into());
            }
            connect_via_networkmanager(open_network_connection_settings(&network.ssid))
        }
        ConnectionRequest::Secured { passphrase, .. } => {
            match classify_security(network, Some(passphrase)) {
                SecurityKind::WpaPsk => connect_via_networkmanager(
                    secured_network_connection_settings(&network.ssid, passphrase, "wpa-psk"),
                ),
                SecurityKind::WpaSae => connect_via_networkmanager(
                    secured_network_connection_settings(&network.ssid, passphrase, "sae"),
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

pub fn disconnect_from_network(network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
    if disconnect_via_networkmanager(network)? {
        Ok(())
    } else {
        Err("NetworkManager could not find a matching active WiFi device to disconnect".into())
    }
}
