#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiSecurity {
    Open,
    WpaPsk,
    WpaSae,
    Enterprise,
    Unsupported,
}

impl WifiSecurity {
    pub fn is_secured(self) -> bool {
        !matches!(self, Self::Open)
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::WpaPsk => "WPA/WPA2 Personal",
            Self::WpaSae => "WPA3 Personal",
            Self::Enterprise => "Enterprise (802.1X)",
            Self::Unsupported => "Unsupported secured network",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal_strength: u8,
    pub security: WifiSecurity,
    pub frequency: u32,
    pub connected: bool,
}

impl WifiNetwork {
    pub fn is_secured(&self) -> bool {
        self.security.is_secured()
    }
}
