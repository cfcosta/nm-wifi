use std::{
    error::Error,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{Terminal, backend::Backend};

use crate::{
    app_state::{App, AppState, OperationKind},
    backend::{NetworkBackend, default_backend},
    network::ConnectionRequest,
    ui::ui,
    wifi::WifiNetwork,
};

#[cfg_attr(not(test), allow(dead_code))]
mod runtime;

pub struct CleanupGuard<F: FnOnce()> {
    cleanup: Option<F>,
}

impl<F: FnOnce()> CleanupGuard<F> {
    pub fn new(cleanup: F) -> Self {
        Self {
            cleanup: Some(cleanup),
        }
    }

    pub fn dismiss(mut self) {
        self.cleanup = None;
    }
}

impl<F: FnOnce()> Drop for CleanupGuard<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

pub fn begin_disconnect_for_selected_network(app: &mut App) {
    if let Some(network) = app
        .selected_network_in_list()
        .filter(|n| n.connected)
        .cloned()
    {
        app.begin_operation(network, OperationKind::Disconnect);
    }
}

const CONNECTION_COMPLETION_REQUIRES_NETWORK: &str =
    "connection completion requires a selected network";
const DISCONNECTION_COMPLETION_REQUIRES_NETWORK: &str =
    "disconnection completion requires a selected network";

fn selected_network_for_operation<'a>(
    app: &'a App,
    message: &'static str,
) -> &'a WifiNetwork {
    app.selected_network.as_ref().expect(message)
}

fn apply_scanned_networks(
    app: &mut App,
    networks: Vec<WifiNetwork>,
    adapter_name: Option<String>,
) {
    let previous_count = app.networks.len();
    app.networks = networks;
    app.network_count = app.networks.len();
    app.last_scan_time = Some(Instant::now());

    if app.adapter_name.is_none() {
        app.adapter_name = adapter_name;
    }

    if previous_count == 0 && !app.networks.is_empty() {
        if app.selected_network.is_some() {
            app.update_selection_after_rescan();
        } else {
            app.selected_index = 0;
        }
    }

    if !app.networks.is_empty() {
        app.status_message = format!(
            "Found {} network(s). Ready to connect!",
            app.networks.len()
        );
        app.state = AppState::NetworkList;
    } else {
        app.status_message = "Scanning for WiFi networks...".to_string();
    }
}

async fn refresh_networks(backend: &dyn NetworkBackend, app: &mut App) {
    let networks = match backend.scan_networks().await {
        Ok(networks) => networks,
        Err(error) => {
            app.handle_scan_error(error);
            return;
        }
    };
    let adapter_name = if app.adapter_name.is_none() {
        backend.adapter_name().ok().flatten()
    } else {
        None
    };

    apply_scanned_networks(app, networks, adapter_name);
}

pub async fn refresh_networks_with_backend(
    backend: &dyn NetworkBackend,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    refresh_networks(backend, app).await;
    Ok(())
}

fn complete_connection(backend: &dyn NetworkBackend, app: &mut App) {
    let network = selected_network_for_operation(
        app,
        CONNECTION_COMPLETION_REQUIRES_NETWORK,
    );
    let request = if network.security.is_secured() {
        ConnectionRequest::Secured {
            network,
            passphrase: app.password_input.as_str(),
        }
    } else {
        ConnectionRequest::Open { network }
    };

    match backend.connect(request) {
        Ok(_) => app.finish_operation(true, None),
        Err(error) => app.finish_operation(false, Some(error.to_string())),
    }
}

pub fn complete_connection_with_backend(
    backend: &dyn NetworkBackend,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    complete_connection(backend, app);
    Ok(())
}

fn complete_disconnection(backend: &dyn NetworkBackend, app: &mut App) {
    let network = selected_network_for_operation(
        app,
        DISCONNECTION_COMPLETION_REQUIRES_NETWORK,
    );

    match backend.disconnect(network) {
        Ok(_) => app.finish_operation(true, None),
        Err(error) => app.finish_operation(false, Some(error.to_string())),
    }
}

pub fn complete_disconnection_with_backend(
    backend: &dyn NetworkBackend,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    complete_disconnection(backend, app);
    Ok(())
}

fn handle_scanning_keypress(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.quit(),
        KeyCode::Char('j') | KeyCode::Down if !app.networks.is_empty() => {
            app.next()
        }
        KeyCode::Char('k') | KeyCode::Up if !app.networks.is_empty() => {
            app.previous()
        }
        KeyCode::Enter | KeyCode::Char('c') if !app.networks.is_empty() => {
            app.activate_selected_network()
        }
        _ => {}
    }
}

async fn handle_scanning_state(
    backend: &dyn NetworkBackend,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            handle_scanning_keypress(app, key.code);
        }
        return Ok(());
    }

    refresh_networks(backend, app).await;
    Ok(())
}

async fn handle_connection_state(
    backend: &dyn NetworkBackend,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    if event::poll(Duration::from_millis(100))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
        && key.code == KeyCode::Esc
    {
        app.quit();
        return Ok(());
    }

    complete_connection(backend, app);
    Ok(())
}

async fn handle_disconnection_state(
    backend: &dyn NetworkBackend,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    if event::poll(Duration::from_millis(100))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
        && key.code == KeyCode::Esc
    {
        app.quit();
        return Ok(());
    }

    complete_disconnection(backend, app);
    Ok(())
}

fn handle_keypress(app: &mut App, key: KeyCode) {
    match app.state {
        AppState::NetworkList => match key {
            KeyCode::Char('q') | KeyCode::Esc => app.quit(),
            KeyCode::Char('j') | KeyCode::Down => app.next(),
            KeyCode::Char('k') | KeyCode::Up => app.previous(),
            KeyCode::Enter | KeyCode::Char('c') => {
                app.activate_selected_network()
            }
            KeyCode::Char('d') => begin_disconnect_for_selected_network(app),
            KeyCode::Char('r') => app.start_scan(),
            KeyCode::Char('h') => app.state = AppState::Help,
            KeyCode::Char('i') if !app.networks.is_empty() => {
                app.state = AppState::NetworkDetails;
            }
            _ => {}
        },
        AppState::Help => match key {
            KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('q') => {
                app.state = AppState::NetworkList;
            }
            _ => {}
        },
        AppState::NetworkDetails => match key {
            KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('q') => {
                app.state = AppState::NetworkList;
            }
            _ => {}
        },
        AppState::PasswordInput => match key {
            KeyCode::Esc => {
                app.state = AppState::NetworkList;
                app.password_input.clear();
                app.password_visible = false;
            }
            KeyCode::Enter => app.confirm_password(),
            KeyCode::Backspace => app.remove_char_from_password(),
            KeyCode::Tab => app.password_visible = !app.password_visible,
            KeyCode::Char(c) => app.add_char_to_password(c),
            _ => {}
        },
        AppState::ConnectionResult => match key {
            KeyCode::Char('q') | KeyCode::Esc => app.quit(),
            KeyCode::Enter => {
                app.back_to_network_list();
                app.start_scan();
            }
            _ => {}
        },
        AppState::Scanning | AppState::Connecting | AppState::Disconnecting => {
        }
    }
}

pub async fn run_app_with_backend<B: Backend>(
    terminal: &mut Terminal<B>,
    backend: &dyn NetworkBackend,
    mut app: App,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|frame| ui(frame, &app))?;

        if app.should_quit {
            break;
        }

        match app.state {
            AppState::Scanning => {
                handle_scanning_state(backend, &mut app).await?;
                continue;
            }
            AppState::Connecting => {
                handle_connection_state(backend, &mut app).await?;
                continue;
            }
            AppState::Disconnecting => {
                handle_disconnection_state(backend, &mut app).await?;
                continue;
            }
            _ => {}
        }

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            handle_keypress(&mut app, key.code);
        }
    }

    Ok(())
}

pub async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: App,
) -> Result<(), Box<dyn Error>> {
    let backend = default_backend();
    run_app_with_backend(terminal, backend.as_ref(), app).await
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, error::Error, rc::Rc};

    use super::{
        CleanupGuard,
        begin_disconnect_for_selected_network,
        complete_connection,
        complete_disconnection,
    };
    use crate::{
        app_state::{App, AppState},
        backend::{BackendFuture, NetworkBackend},
        network::ConnectionRequest,
        wifi::{WifiNetwork, WifiSecurity},
    };

    struct NoopBackend;

    impl NetworkBackend for NoopBackend {
        fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>> {
            Ok(None)
        }

        fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>> {
            Ok(None)
        }

        fn scan_networks(
            &self,
        ) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>>
        {
            Box::pin(async { Ok(Vec::new()) })
        }

        fn connect(
            &self,
            _request: ConnectionRequest<'_>,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn disconnect(
            &self,
            _network: &WifiNetwork,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    fn network(ssid: &str, connected: bool) -> WifiNetwork {
        WifiNetwork {
            ssid: ssid.to_string(),
            signal_strength: 80,
            security: WifiSecurity::WpaPsk,
            frequency: 5180,
            connected,
        }
    }

    #[test]
    fn cleanup_guard_runs_cleanup_on_drop() {
        let cleaned = Rc::new(RefCell::new(false));
        let cleaned_for_drop = Rc::clone(&cleaned);

        {
            let _guard = CleanupGuard::new(move || {
                *cleaned_for_drop.borrow_mut() = true;
            });
        }

        assert!(*cleaned.borrow());
    }

    #[test]
    fn disconnect_shortcut_uses_current_selected_connected_network() {
        let mut app = App::new();
        app.state = AppState::NetworkList;
        app.networks = vec![network("guest", false), network("home", true)];
        app.selected_index = 1;

        begin_disconnect_for_selected_network(&mut app);

        assert!(matches!(app.state, AppState::Disconnecting));
        assert!(app.is_disconnect_operation);
        assert!(app.connection_start_time.is_some());
        assert_eq!(
            app.selected_network
                .as_ref()
                .map(|network| network.ssid.as_str()),
            Some("home")
        );
        assert_eq!(app.status_message, "Disconnecting from home...");
    }

    #[test]
    fn disconnect_shortcut_ignores_unconnected_selected_network() {
        let mut app = App::new();
        app.state = AppState::NetworkList;
        app.networks = vec![network("guest", false), network("home", true)];
        app.selected_index = 0;

        begin_disconnect_for_selected_network(&mut app);

        assert!(matches!(app.state, AppState::NetworkList));
        assert!(!app.is_disconnect_operation);
        assert!(app.connection_start_time.is_none());
        assert!(app.selected_network.is_none());
    }

    #[test]
    #[should_panic(
        expected = "connection completion requires a selected network"
    )]
    fn connection_completion_requires_selected_network() {
        let backend = NoopBackend;
        let mut app = App::new();

        complete_connection(&backend, &mut app);
    }

    #[test]
    #[should_panic(
        expected = "disconnection completion requires a selected network"
    )]
    fn disconnection_completion_requires_selected_network() {
        let backend = NoopBackend;
        let mut app = App::new();

        complete_disconnection(&backend, &mut app);
    }
}
