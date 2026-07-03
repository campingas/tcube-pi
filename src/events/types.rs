use serde::{Deserialize, Serialize};

fn default_dashboard_host() -> String {
    "tcube.local".to_string()
}

fn default_setup_help_text() -> String {
    "Open t cube dot local, or the IP address, to set me up.".to_string()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonBehavior {
    Language,
    Animals,
    Music,
    Soundbox,
    Disabled,
    SetupHelp,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
    pub id: String,
    pub text: String,
    pub audio_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModeContent {
    pub mode: String,
    pub responses: Vec<Response>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ButtonMapping {
    pub button_id: u8,
    pub behavior: ButtonBehavior,
    pub mode: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContentPack {
    #[serde(default)]
    pub setup_complete: bool,
    #[serde(default = "default_dashboard_host")]
    pub dashboard_host: String,
    #[serde(default)]
    pub dashboard_ip: Option<String>,
    #[serde(default = "default_setup_help_text")]
    pub setup_help_text: String,
    #[serde(default)]
    pub button_mappings: Vec<ButtonMapping>,
    pub modes: Vec<ModeContent>,
}

#[derive(Clone, Debug)]
pub struct ButtonEvent {
    pub occurred_at: String,
    pub button_id: u8,
    pub mode: String,
    pub response_id: String,
    pub response_text: String,
}

#[derive(Clone, Debug)]
pub struct ImpactEvent {
    pub occurred_at: String,
    pub source: String,
    pub latency_us: u128,
}

#[derive(Clone, Debug)]
pub struct Measurement {
    pub occurred_at: i64,
    pub button_id: u8,
    pub mode: String,
    pub response_id: String,
    pub response_text: String,
    pub latency_us: u128,
}
