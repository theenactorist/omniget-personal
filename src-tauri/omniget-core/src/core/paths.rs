/// Personal builds keep data separate from the upstream application.
pub fn app_data_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("OMNIGET_DATA_DIR") {
        return Some(std::path::PathBuf::from(dir));
    }
    dirs::data_dir().map(|base| base.join("local.omniget.personal"))
}
