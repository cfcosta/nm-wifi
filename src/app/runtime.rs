use std::{error::Error, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{Terminal, backend::Backend};

use super::{
    CONNECTION_COMPLETION_REQUIRES_NETWORK,
    DISCONNECTION_COMPLETION_REQUIRES_NETWORK,
    apply_scanned_networks,
    handle_keypress,
    handle_scanning_keypress,
    selected_network_for_operation,
};
use crate::{
    app_state::{App, AppState},
    ui::ui,
    wifi::{WifiNetwork, WifiSecurity},
};

const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub(crate) struct ScanSnapshot {
    pub(crate) networks: Vec<WifiNetwork>,
    pub(crate) adapter_name: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum RuntimeRequest {
    Scan,
    Connect {
        network: WifiNetwork,
        passphrase: Option<String>,
    },
    Disconnect {
        network: WifiNetwork,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum RuntimeEvent {
    ScanFinished(Result<ScanSnapshot, String>),
    ConnectFinished(Result<(), String>),
    DisconnectFinished(Result<(), String>),
}

pub(crate) trait RuntimeInput {
    fn next_key(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<KeyCode>, Box<dyn Error>>;
}

pub(crate) struct CrosstermInput;

impl RuntimeInput for CrosstermInput {
    fn next_key(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<KeyCode>, Box<dyn Error>> {
        if !event::poll(timeout)? {
            return Ok(None);
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                Ok(Some(key.code))
            }
            _ => Ok(None),
        }
    }
}

pub(crate) trait RuntimeBackendDriver {
    fn begin(&mut self, request: RuntimeRequest);

    fn poll_event(&mut self) -> Result<Option<RuntimeEvent>, Box<dyn Error>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InFlightRequest {
    Scan,
    Connect,
    Disconnect,
}

pub(crate) async fn run_app_with_runtime<B, I, D>(
    terminal: &mut Terminal<B>,
    input: &mut I,
    driver: &mut D,
    mut app: App,
) -> Result<App, Box<dyn Error>>
where
    B: Backend,
    I: RuntimeInput + ?Sized,
    D: RuntimeBackendDriver + ?Sized,
{
    let mut in_flight = None;

    loop {
        terminal.draw(|frame| ui(frame, &app))?;

        if app.should_quit {
            break;
        }

        if let Some(event) = driver.poll_event()? {
            apply_runtime_event(&mut app, event);
            in_flight = None;
            continue;
        }

        if let Some(request) = in_flight {
            handle_in_flight_request(input, &mut app, request)?;
            continue;
        }

        match app.state {
            AppState::Scanning => match input.next_key(INPUT_POLL_INTERVAL)? {
                Some(key) => handle_scanning_keypress(&mut app, key),
                None => {
                    driver.begin(RuntimeRequest::Scan);
                    in_flight = Some(InFlightRequest::Scan);
                }
            },
            AppState::Connecting => {
                if let Some(key) = input.next_key(INPUT_POLL_INTERVAL)? {
                    if key == KeyCode::Esc {
                        app.quit();
                    }
                } else {
                    driver.begin(connection_request(&app));
                    in_flight = Some(InFlightRequest::Connect);
                }
            }
            AppState::Disconnecting => {
                if let Some(key) = input.next_key(INPUT_POLL_INTERVAL)? {
                    if key == KeyCode::Esc {
                        app.quit();
                    }
                } else {
                    driver.begin(disconnection_request(&app));
                    in_flight = Some(InFlightRequest::Disconnect);
                }
            }
            _ => {
                if let Some(key) = input.next_key(INPUT_POLL_INTERVAL)? {
                    handle_keypress(&mut app, key);
                }
            }
        }
    }

    Ok(app)
}

fn handle_in_flight_request<I: RuntimeInput + ?Sized>(
    input: &mut I,
    app: &mut App,
    request: InFlightRequest,
) -> Result<(), Box<dyn Error>> {
    match request {
        InFlightRequest::Scan => {
            if let Some(key) = input.next_key(INPUT_POLL_INTERVAL)? {
                handle_scanning_keypress(app, key);
            }
        }
        InFlightRequest::Connect | InFlightRequest::Disconnect => {
            if let Some(key) = input.next_key(INPUT_POLL_INTERVAL)?
                && key == KeyCode::Esc
            {
                app.quit();
            }
        }
    }

    Ok(())
}

fn connection_request(app: &App) -> RuntimeRequest {
    let network = selected_network_for_operation(
        app,
        CONNECTION_COMPLETION_REQUIRES_NETWORK,
    )
    .clone();
    let passphrase = matches!(network.security, WifiSecurity::Open)
        .then(|| None)
        .unwrap_or_else(|| Some(app.password_input.clone()));

    RuntimeRequest::Connect {
        network,
        passphrase,
    }
}

fn disconnection_request(app: &App) -> RuntimeRequest {
    let network = selected_network_for_operation(
        app,
        DISCONNECTION_COMPLETION_REQUIRES_NETWORK,
    )
    .clone();
    RuntimeRequest::Disconnect { network }
}

fn apply_runtime_event(app: &mut App, event: RuntimeEvent) {
    match event {
        RuntimeEvent::ScanFinished(Ok(snapshot)) => apply_scanned_networks(
            app,
            snapshot.networks,
            snapshot.adapter_name,
        ),
        RuntimeEvent::ScanFinished(Err(error)) => app.handle_scan_error(error),
        RuntimeEvent::ConnectFinished(Ok(())) => {
            app.finish_operation(true, None)
        }
        RuntimeEvent::ConnectFinished(Err(error)) => {
            app.finish_operation(false, Some(error))
        }
        RuntimeEvent::DisconnectFinished(Ok(())) => {
            app.finish_operation(true, None)
        }
        RuntimeEvent::DisconnectFinished(Err(error)) => {
            app.finish_operation(false, Some(error))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, error::Error, time::Duration};

    use crossterm::event::KeyCode;
    use ratatui::{Terminal, backend::TestBackend};

    use super::{
        RuntimeBackendDriver,
        RuntimeEvent,
        RuntimeInput,
        RuntimeRequest,
        apply_runtime_event,
        run_app_with_runtime,
    };
    use crate::{
        app_state::{App, AppState},
        wifi::{WifiNetwork, WifiSecurity},
    };

    fn network(
        ssid: &str,
        security: WifiSecurity,
        connected: bool,
    ) -> WifiNetwork {
        WifiNetwork {
            ssid: ssid.to_string(),
            signal_strength: 78,
            security,
            frequency: 5180,
            connected,
        }
    }

    struct ScriptedInput {
        keys: VecDeque<Option<KeyCode>>,
    }

    impl ScriptedInput {
        fn new(keys: Vec<Option<KeyCode>>) -> Self {
            Self {
                keys: VecDeque::from(keys),
            }
        }
    }

    impl RuntimeInput for ScriptedInput {
        fn next_key(
            &mut self,
            _timeout: Duration,
        ) -> Result<Option<KeyCode>, Box<dyn Error>> {
            Ok(self.keys.pop_front().flatten())
        }
    }

    struct ScriptedDriver {
        begin_calls: Vec<&'static str>,
        events: VecDeque<Option<RuntimeEvent>>,
    }

    impl ScriptedDriver {
        fn new(events: Vec<Option<RuntimeEvent>>) -> Self {
            Self {
                begin_calls: Vec::new(),
                events: VecDeque::from(events),
            }
        }
    }

    impl RuntimeBackendDriver for ScriptedDriver {
        fn begin(&mut self, request: RuntimeRequest) {
            match request {
                RuntimeRequest::Scan => self.begin_calls.push("scan"),
                RuntimeRequest::Connect {
                    network,
                    passphrase,
                } => {
                    assert_eq!(network.ssid, "CatCat");
                    assert_eq!(passphrase.as_deref(), Some("AcerolaAcai"));
                    self.begin_calls.push("connect")
                }
                RuntimeRequest::Disconnect { network } => {
                    assert_eq!(network.ssid, "CatCat");
                    self.begin_calls.push("disconnect")
                }
            }
        }

        fn poll_event(
            &mut self,
        ) -> Result<Option<RuntimeEvent>, Box<dyn Error>> {
            Ok(self.events.pop_front().flatten())
        }
    }

    #[tokio::test]
    async fn pending_connect_can_be_aborted_with_esc() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        let mut input = ScriptedInput::new(vec![None, Some(KeyCode::Esc)]);
        let mut driver = ScriptedDriver::new(vec![None, None]);
        let mut app = App::new();
        app.state = AppState::Connecting;
        app.selected_network =
            Some(network("CatCat", WifiSecurity::WpaSae, false));
        app.password_input = "AcerolaAcai".to_string();

        let app =
            run_app_with_runtime(&mut terminal, &mut input, &mut driver, app)
                .await
                .expect("runtime loop succeeds");

        assert!(app.should_quit);
        assert!(matches!(app.state, AppState::Connecting));
        assert_eq!(driver.begin_calls, vec!["connect"]);
    }

    #[tokio::test]
    async fn pending_scan_can_be_aborted_with_esc() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        let mut input = ScriptedInput::new(vec![None, Some(KeyCode::Esc)]);
        let mut driver = ScriptedDriver::new(vec![None, None]);
        let app = App::new();

        let app =
            run_app_with_runtime(&mut terminal, &mut input, &mut driver, app)
                .await
                .expect("runtime loop succeeds");

        assert!(app.should_quit);
        assert!(matches!(app.state, AppState::Scanning));
        assert_eq!(driver.begin_calls, vec!["scan"]);
    }

    #[tokio::test]
    async fn pending_disconnect_completion_preserves_error_mapping() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        let mut input =
            ScriptedInput::new(vec![None, None, Some(KeyCode::Esc)]);
        let mut driver = ScriptedDriver::new(vec![
            None,
            None,
            Some(RuntimeEvent::DisconnectFinished(Err(
                "disconnect failed".to_string()
            ))),
            None,
        ]);
        let mut app = App::new();
        app.state = AppState::Disconnecting;
        app.selected_network =
            Some(network("CatCat", WifiSecurity::WpaSae, true));
        app.is_disconnect_operation = true;

        let app =
            run_app_with_runtime(&mut terminal, &mut input, &mut driver, app)
                .await
                .expect("runtime loop succeeds");

        assert!(matches!(app.state, AppState::ConnectionResult));
        assert!(!app.connection_success);
        assert_eq!(app.connection_error.as_deref(), Some("disconnect failed"));
        assert_eq!(driver.begin_calls, vec!["disconnect"]);
    }

    #[test]
    fn runtime_events_apply_scan_and_connect_results() {
        let mut app = App::new();
        apply_runtime_event(
            &mut app,
            RuntimeEvent::ScanFinished(Ok(super::ScanSnapshot {
                networks: vec![network("CatCat", WifiSecurity::WpaSae, true)],
                adapter_name: Some("demo-wlan0".to_string()),
            })),
        );

        assert!(matches!(app.state, AppState::NetworkList));
        assert_eq!(app.network_count, 1);
        assert_eq!(app.adapter_name.as_deref(), Some("demo-wlan0"));

        app.selected_network =
            Some(network("CatCat", WifiSecurity::WpaSae, true));
        apply_runtime_event(&mut app, RuntimeEvent::ConnectFinished(Ok(())));

        assert!(matches!(app.state, AppState::ConnectionResult));
        assert!(app.connection_success);
    }
}
