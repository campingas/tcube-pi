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
    pub usb_connected: bool,
    pub version_file: PathBuf,
    pub update_dir: PathBuf,
    pub update_repo: String,
}
