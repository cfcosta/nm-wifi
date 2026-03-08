#[cfg(any(test, not(feature = "demo")))]
use std::collections::HashMap;
use std::error::Error;

#[cfg(any(test, not(feature = "demo")))]
use dbus::arg::{PropMap, RefArg, Variant};

use crate::wifi::WifiNetwork;

#[cfg(feature = "demo")]
pub(crate) mod demo;
#[cfg(not(feature = "demo"))]
pub(crate) mod networkmanager;

pub enum ConnectionRequest<'a> {
    Open {
        network: &'a WifiNetwork,
    },
    Secured {
        network: &'a WifiNetwork,
        password: &'a str,
    },
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

#[cfg(feature = "demo")]
pub use demo::demo_networks;

#[cfg(feature = "demo")]
pub fn get_connected_ssid() -> Result<Option<String>, Box<dyn Error>> {
    demo::get_connected_ssid()
}

#[cfg(not(feature = "demo"))]
pub fn get_connected_ssid() -> Result<Option<String>, Box<dyn Error>> {
    networkmanager::get_connected_ssid()
}

#[cfg(feature = "demo")]
pub fn get_wifi_adapter_name() -> Result<Option<String>, Box<dyn Error>> {
    demo::get_wifi_adapter_name()
}

#[cfg(not(feature = "demo"))]
pub fn get_wifi_adapter_name() -> Result<Option<String>, Box<dyn Error>> {
    networkmanager::get_wifi_adapter_name()
}

#[cfg(feature = "demo")]
pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, Box<dyn Error>> {
    demo::scan_wifi_networks().await
}

#[cfg(not(feature = "demo"))]
pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, Box<dyn Error>> {
    networkmanager::scan_wifi_networks().await
}

#[cfg(feature = "demo")]
pub fn connect_to_network(request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
    demo::connect_to_network(request)
}

#[cfg(not(feature = "demo"))]
pub fn connect_to_network(request: ConnectionRequest<'_>) -> Result<(), Box<dyn Error>> {
    networkmanager::connect_to_network(request)
}

#[cfg(feature = "demo")]
pub fn disconnect_from_network(network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
    demo::disconnect_from_network(network)
}

#[cfg(not(feature = "demo"))]
pub fn disconnect_from_network(network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
    networkmanager::disconnect_from_network(network)
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "demo"))]
    use std::time::Duration;

    #[cfg(feature = "demo")]
    use super::ConnectionRequest;
    #[cfg(feature = "demo")]
    use super::demo::{connect_to_network, demo_networks, scan_wifi_networks};
    #[cfg(not(feature = "demo"))]
    use super::networkmanager::{
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
    use super::{open_network_connection_settings, secured_network_connection_settings};
    #[cfg(not(feature = "demo"))]
    use crate::wifi::WifiNetwork;
    use crate::wifi::WifiSecurity;

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
