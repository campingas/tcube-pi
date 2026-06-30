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
        contract_note: "Serves the static admin UI and compatible auth, setup, content, media, and status APIs behind the selected Caddy HTTPS boundary.",
    }
}
