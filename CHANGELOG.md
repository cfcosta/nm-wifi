# Changelog

All notable changes to `nm-wifi` are documented here, based on the project's `jj` history.

## v0.2.0

This release focused on making `nm-wifi` more dependable in everyday use, while also making the project easier to package, test, and demo.

### Highlights

- **More reliable Wi-Fi operations through NetworkManager**
  - Scanning, connecting, and disconnecting were tightened up around NetworkManager.
  - Secured Wi-Fi setup now lets NetworkManager negotiate connection settings more cleanly.
  - Disconnect behavior was fixed so the app targets the active Wi-Fi device instead of relying on profile-name assumptions.
  - Missing or inconsistent access-point data is handled more gracefully.

- **Better resilience during failures**
  - The TUI now stays alive if a scan fails instead of collapsing the session.
  - Terminal cleanup on startup failures was improved, reducing the chance of leaving the terminal in a broken state.
  - Rescans now clear stale metadata correctly, which helps avoid confusing state after repeated refreshes.

- **UI polish and clearer feedback**
  - Network list hints were aligned with actual connect and disconnect behavior.
  - Misleading cancel hints during network operations were removed.
  - Duplicate modal headers were removed and modal copy was simplified.
  - SSIDs now respect terminal display width more accurately.
  - 6 GHz networks are labeled correctly as **6G**.
  - Secure-network detection was improved, so networks are classified based on real key-management data.

- **New demo mode and screenshot generation**
  - A mocked backend was added so you can explore the full interface without touching your live NetworkManager setup.
  - The project can now generate README screenshots directly from the demo UI.

### Behind the scenes

- The app was refactored around a shared backend trait, which improves internal structure and testability.
- Selection handling and app-state boundaries were cleaned up to reduce edge-case UI bugs.
- Packaging and release workflow work was improved, including cleaner Nix packaging and a manual crates.io release workflow.

## v0.1.0

The first release established `nm-wifi` as a keyboard-driven terminal app for scanning nearby Wi-Fi networks and managing connections on Linux.

### Highlights

- **Initial Wi-Fi TUI**
  - Added the core terminal interface for browsing available networks.
  - Introduced a more polished UI with theming, borders, and improved presentation.

- **Connect and disconnect support**
  - Added support for connecting to networks from inside the TUI.
  - Added visibility for the currently connected network and support for disconnecting.
  - Improved WPA handling and password-entry flow.

- **Smoother in-app workflow**
  - The app no longer exits immediately after connect or disconnect actions.
  - Added more keybindings and better dialogs.
  - Fixed selection behavior after connect and disconnect operations.
  - Improved scan behavior so the scanning state is shown more appropriately.

- **Documentation and onboarding**
  - Added the initial README and later expanded it with more complete documentation.

### Behind the scenes

- The codebase was reorganized into clearer modules.
- Dependency updates and small runtime timing tweaks improved responsiveness while waiting for connection operations.
