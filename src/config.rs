use std::path::PathBuf;

#[derive(Debug)]
pub struct AdminConfig {
    pub bind: String,
    pub database: PathBuf,
    pub ui_dist: PathBuf,
    pub media_root: PathBuf,
    pub content_root: PathBuf,
    pub hostname: String,
    pub usb_address: String,
}

#[derive(Debug)]
pub struct DeviceApiConfig {
    pub bind: String,
    pub database: PathBuf,
}
