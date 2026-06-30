use serde::Serialize;

use crate::config::AdminConfig;

#[derive(Debug, Serialize)]
pub(crate) struct StatusResponse {
    status: &'static str,
    service: &'static str,
    mode: &'static str,
    database_present: bool,
    ui_dist_present: bool,
    media_root: String,
    content_root: String,
    hostname: String,
    usb_address: String,
    usb_connected: bool,
    contract_note: &'static str,
}

pub(crate) fn pi_status(config: &AdminConfig) -> StatusResponse {
    StatusResponse {
        status: "ok",
        service: "tcube-pi-admin",
        mode: "pi_hosted_admin_spike",
        database_present: config.database.exists(),
        ui_dist_present: config.ui_dist.join("index.html").exists(),
        media_root: config.media_root.display().to_string(),
        content_root: config.content_root.display().to_string(),
        hostname: config.hostname.clone(),
        usb_address: config.usb_address.clone(),
        usb_connected: config.usb_connected,
        contract_note: "Serves the static admin UI and compatible auth, setup, content, media, and status APIs behind the selected Caddy HTTPS boundary.",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::AdminConfig;

    use super::pi_status;

    #[test]
    fn pi_status_uses_explicit_usb_connection_flag() {
        let connected = AdminConfig {
            bind: "127.0.0.1:0".to_string(),
            database: PathBuf::from("/tmp/tcube.sqlite3"),
            ui_dist: PathBuf::from("admin-ui"),
            media_root: PathBuf::from("data/audio"),
            content_root: PathBuf::from("content"),
            hostname: "tcube.local".to_string(),
            usb_address: "10.55.0.1".to_string(),
            usb_connected: true,
        };
        let disconnected = AdminConfig {
            bind: "127.0.0.1:0".to_string(),
            database: PathBuf::from("/tmp/tcube.sqlite3"),
            ui_dist: PathBuf::from("admin-ui"),
            media_root: PathBuf::from("data/audio"),
            content_root: PathBuf::from("content"),
            hostname: "tcube.local".to_string(),
            usb_address: "10.55.0.1".to_string(),
            usb_connected: false,
        };

        assert!(pi_status(&connected).usb_connected);
        assert!(!pi_status(&disconnected).usb_connected);
    }
}
