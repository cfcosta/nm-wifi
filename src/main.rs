mod network;
mod theme;
mod types;
mod ui;

use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self,
        DisableMouseCapture,
        EnableMouseCapture,
        Event,
        KeyCode,
        KeyEventKind,
    },
    execute,
    terminal::{
        EnterAlternateScreen,
        LeaveAlternateScreen,
        disable_raw_mode,
        enable_raw_mode,
    },
};
use network::{
    connect_to_network,
    disconnect_from_network,
    get_wifi_adapter_info,
    scan_wifi_networks,
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use types::{App, AppState};
use ui::ui;

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if app.should_quit {
            break;
        }

        if app.state == AppState::Scanning {
            // Process events during scanning to allow UI updates and handle input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press
                {
                    match key.code {
                        KeyCode::Esc => {
                            app.quit();
                            continue;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            if !app.networks.is_empty() {
                                app.next();
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if !app.networks.is_empty() {
                                app.previous();
                            }
                        }
                        KeyCode::Enter | KeyCode::Char('c') => {
                            if !app.networks.is_empty() {
                                app.select_network();
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
                // Continue to redraw with any new events
                continue;
            }

            // Perform incremental scan
            let networks = scan_wifi_networks().await?;
            let previous_count = app.networks.len();
            app.networks = networks;
            app.network_count = app.networks.len();
            app.last_scan_time = Some(Instant::now());

            // Get adapter info on first scan
            if app.adapter_info.is_none() {
                app.adapter_info = get_wifi_adapter_info().await;
            }

            // Update selection when first networks appear or preserve selection
            if previous_count == 0 && !app.networks.is_empty() {
                if app.selected_network.is_some() {
                    app.update_selection_after_rescan();
                } else {
                    app.list_state.select(Some(0));
                }
            }

            // Check if we should finish scanning (after reasonable time or enough networks)
            if !app.networks.is_empty() {
                app.status_message = format!(
                    "Found {} network(s). Ready to connect!",
                    app.networks.len()
                );
                app.state = AppState::NetworkList;
            } else {
                app.status_message =
                    "Scanning for WiFi networks...".to_string();
            }

            continue;
        }

        if app.state == AppState::Connecting {
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && key.code == KeyCode::Esc
            {
                app.quit();
                continue;
            }

            let password = if app.selected_network.as_ref().unwrap().secured {
                Some(app.password_input.as_str())
            } else {
                None
            };

            match connect_to_network(
                app.selected_network.as_ref().unwrap(),
                password,
            )
            .await
            {
                Ok(_) => {
                    app.connection_success = true;
                    app.connection_error = None;
                    app.status_message = "Connected successfully!".to_string();
                }
                Err(e) => {
                    app.connection_success = false;
                    app.connection_error = Some(e.to_string());
                    app.status_message = "Connection failed".to_string();
                }
            }
            app.state = AppState::ConnectionResult;
            continue;
        }

        if app.state == AppState::Disconnecting {
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && key.code == KeyCode::Esc
            {
                app.quit();
                continue;
            }

            match disconnect_from_network(
                app.selected_network.as_ref().unwrap(),
            )
            .await
            {
                Ok(_) => {
                    app.connection_success = true;
                    app.connection_error = None;
                    app.status_message =
                        "Disconnected successfully!".to_string();
                }
                Err(e) => {
                    app.connection_success = false;
                    app.connection_error = Some(e.to_string());
                    app.status_message = "Disconnection failed".to_string();
                }
            }
            app.state = AppState::ConnectionResult;
            continue;
        }

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match app.state {
                AppState::Scanning => {
                    // Handled above in the scanning loop
                }
                AppState::NetworkList => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Enter | KeyCode::Char('c') => app.select_network(),
                    KeyCode::Char('d') => {
                        if let Some(network) = app
                            .networks
                            .get(app.selected_index)
                            .filter(|n| n.connected)
                            .cloned()
                        {
                            app.is_disconnect_operation = true;
                            app.state = AppState::Disconnecting;
                            app.connection_start_time = Some(Instant::now());
                            app.status_message = format!(
                                "Disconnecting from {}...",
                                network.ssid
                            );

                            app.selected_network = Some(network);
                        }
                    }
                    KeyCode::Char('r') => {
                        app.state = AppState::Scanning;
                        app.status_message =
                            "Scanning for networks...".to_string();
                        app.networks.clear();
                    }
                    KeyCode::Char('h') => {
                        app.state = AppState::Help;
                    }
                    KeyCode::Char('i') => {
                        if !app.networks.is_empty() {
                            app.state = AppState::NetworkDetails;
                        }
                    }
                    _ => {}
                },
                AppState::Help => match key.code {
                    KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('q') => {
                        app.state = AppState::NetworkList;
                    }
                    _ => {}
                },
                AppState::NetworkDetails => match key.code {
                    KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('q') => {
                        app.state = AppState::NetworkList;
                    }
                    _ => {}
                },
                AppState::PasswordInput => match key.code {
                    KeyCode::Esc => {
                        app.state = AppState::NetworkList;
                        app.password_input.clear();
                        app.password_visible = false;
                    }
                    KeyCode::Enter => app.confirm_password(),
                    KeyCode::Backspace => app.remove_char_from_password(),
                    KeyCode::Tab => {
                        app.password_visible = !app.password_visible;
                    }
                    KeyCode::Char(c) => app.add_char_to_password(c),
                    _ => {}
                },
                AppState::Connecting => {
                    // Handled above in the connecting loop
                }
                AppState::Disconnecting => {
                    // Handled above in the disconnecting loop
                }
                AppState::ConnectionResult => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Enter => {
                        // Always return to network list after connection result
                        app.back_to_network_list();
                        // Rescan to update connection status
                        app.state = AppState::Scanning;
                        app.status_message =
                            "Scanning for networks...".to_string();
                        app.networks.clear();
                    }
                    _ => {}
                },
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new();
    let res = run_app(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
