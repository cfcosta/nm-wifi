use std::{cell::RefCell, error::Error, rc::Rc};

use nm_wifi::{
    app::{
        complete_connection_with_backend,
        complete_disconnection_with_backend,
        refresh_networks_with_backend,
    },
    app_state::{App, AppState},
    backend::{BackendFuture, NetworkBackend},
    network::ConnectionRequest,
    wifi::{WifiNetwork, WifiSecurity},
};

#[derive(Clone, Default)]
struct FakeBackendState {
    scan_networks: Vec<WifiNetwork>,
    adapter_name: Option<String>,
    adapter_name_error: Option<String>,
    connect_calls: Vec<String>,
    disconnect_calls: Vec<String>,
    connect_error: Option<String>,
    disconnect_error: Option<String>,
    scan_error: Option<String>,
}

struct FakeBackend {
    state: Rc<RefCell<FakeBackendState>>,
}

impl FakeBackend {
    fn new(state: FakeBackendState) -> Self {
        Self {
            state: Rc::new(RefCell::new(state)),
        }
    }
}

fn boxed_error(message: String) -> Box<dyn Error> {
    Box::new(std::io::Error::other(message))
}

impl NetworkBackend for FakeBackend {
    fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>> {
        Ok(None)
    }

    fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>> {
        let state = self.state.borrow();
        match &state.adapter_name_error {
            Some(message) => Err(boxed_error(message.clone())),
            None => Ok(state.adapter_name.clone()),
        }
    }

    fn scan_networks(
        &self,
    ) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>> {
        let result = {
            let state = self.state.borrow();
            match &state.scan_error {
                Some(message) => Err(boxed_error(message.clone())),
                None => Ok(state.scan_networks.clone()),
            }
        };
        Box::pin(async move { result })
    }

    fn connect(
        &self,
        request: ConnectionRequest<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let mut state = self.state.borrow_mut();
        let ssid = match request {
            ConnectionRequest::Open { network }
            | ConnectionRequest::Secured { network, .. } => {
                network.ssid.clone()
            }
        };
        state.connect_calls.push(ssid);
        match &state.connect_error {
            Some(message) => Err(boxed_error(message.clone())),
            None => Ok(()),
        }
    }

    fn disconnect(&self, network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
        let mut state = self.state.borrow_mut();
        state.disconnect_calls.push(network.ssid.clone());
        match &state.disconnect_error {
            Some(message) => Err(boxed_error(message.clone())),
            None => Ok(()),
        }
    }
}

fn network(ssid: &str, security: WifiSecurity, connected: bool) -> WifiNetwork {
    WifiNetwork {
        ssid: ssid.to_string(),
        signal_strength: 77,
        security,
        frequency: 5180,
        connected,
    }
}

#[tokio::test]
async fn fake_backend_scan_updates_app_state_and_adapter_name() {
    let backend = FakeBackend::new(FakeBackendState {
        scan_networks: vec![
            network("CatCat", WifiSecurity::WpaSae, true),
            network("Coffee Corner", WifiSecurity::Open, false),
        ],
        adapter_name: Some("fake-wlan0".to_string()),
        ..FakeBackendState::default()
    });
    let mut app = App::new();

    refresh_networks_with_backend(&backend, &mut app)
        .await
        .expect("scan succeeds");

    assert!(matches!(app.state, AppState::NetworkList));
    assert_eq!(app.network_count, 2);
    assert_eq!(app.adapter_name.as_deref(), Some("fake-wlan0"));
    assert_eq!(app.selected_index, 0);
}

#[tokio::test]
async fn fake_backend_scan_errors_leave_retry_message() {
    let backend = FakeBackend::new(FakeBackendState {
        scan_error: Some("backend unavailable".to_string()),
        ..FakeBackendState::default()
    });
    let mut app = App::new();

    refresh_networks_with_backend(&backend, &mut app)
        .await
        .expect("scan helper handles backend errors internally");

    assert!(matches!(app.state, AppState::NetworkList));
    assert!(
        app.status_message
            .contains("Scan failed: backend unavailable")
    );
}

#[tokio::test]
async fn refresh_ignores_adapter_name_lookup_failure() {
    let backend = FakeBackend::new(FakeBackendState {
        scan_networks: vec![network(
            "Coffee Corner",
            WifiSecurity::Open,
            false,
        )],
        adapter_name_error: Some("adapter lookup failed".to_string()),
        ..FakeBackendState::default()
    });
    let mut app = App::new();

    refresh_networks_with_backend(&backend, &mut app)
        .await
        .expect("refresh succeeds even when adapter lookup fails");

    assert!(matches!(app.state, AppState::NetworkList));
    assert_eq!(app.network_count, 1);
    assert_eq!(app.selected_index, 0);
    assert!(app.adapter_name.is_none());
}

#[test]
fn fake_backend_connect_completes_result_state_and_records_calls() {
    let backend_state = FakeBackendState::default();
    let backend = FakeBackend::new(backend_state);
    let mut app = App::new();
    app.selected_network = Some(network("CatCat", WifiSecurity::WpaSae, false));
    app.password_input = "AcerolaAcai".to_string();
    app.state = AppState::Connecting;

    complete_connection_with_backend(&backend, &mut app)
        .expect("connect succeeds");

    assert!(matches!(app.state, AppState::ConnectionResult));
    assert!(app.connection_success);
    assert_eq!(
        backend.state.borrow().connect_calls,
        vec!["CatCat".to_string()]
    );
}

#[test]
fn connect_failure_maps_into_connection_result() {
    let backend = FakeBackend::new(FakeBackendState {
        connect_error: Some("connect failed".to_string()),
        ..FakeBackendState::default()
    });
    let mut app = App::new();
    app.selected_network = Some(network("CatCat", WifiSecurity::WpaSae, false));
    app.password_input = "AcerolaAcai".to_string();
    app.state = AppState::Connecting;

    complete_connection_with_backend(&backend, &mut app)
        .expect("connect helper maps backend failure into app state");

    assert!(matches!(app.state, AppState::ConnectionResult));
    assert!(!app.connection_success);
    assert_eq!(app.connection_error.as_deref(), Some("connect failed"));
    assert_eq!(
        backend.state.borrow().connect_calls,
        vec!["CatCat".to_string()]
    );
}

#[test]
fn fake_backend_disconnect_surfaces_backend_failures() {
    let backend = FakeBackend::new(FakeBackendState {
        disconnect_error: Some("disconnect failed".to_string()),
        ..FakeBackendState::default()
    });
    let mut app = App::new();
    app.selected_network = Some(network("CatCat", WifiSecurity::WpaSae, true));
    app.state = AppState::Disconnecting;
    app.is_disconnect_operation = true;

    complete_disconnection_with_backend(&backend, &mut app)
        .expect("disconnect helper maps backend failure into app state");

    assert!(matches!(app.state, AppState::ConnectionResult));
    assert!(!app.connection_success);
    assert_eq!(app.connection_error.as_deref(), Some("disconnect failed"));
    assert_eq!(
        backend.state.borrow().disconnect_calls,
        vec!["CatCat".to_string()]
    );
}
