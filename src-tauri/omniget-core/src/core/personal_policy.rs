//! Permanent restrictions for this personal build. Stored settings and
//! environment variables cannot opt back into the removed remote services.

use crate::models::settings::AppSettings;
use crate::platforms::Platform;

pub const DISABLED_PLUGIN_REASON: &str =
    "External native plugins and the plugin marketplace are disabled in this personal build.";
pub const DISABLED_SERVICE_REASON: &str = "This service is disabled in this personal build.";

pub fn require_native_plugins() -> Result<(), String> {
    Err(DISABLED_PLUGIN_REASON.to_string())
}

pub fn require_p2p() -> Result<(), String> {
    Err(DISABLED_SERVICE_REASON.to_string())
}

pub fn require_discord_rpc() -> Result<(), String> {
    Err(DISABLED_SERVICE_REASON.to_string())
}

pub fn normalize_settings(mut settings: AppSettings) -> AppSettings {
    settings.rpc.enabled = false;
    settings.omnidisc.enabled = false;
    settings.league.enabled = false;
    settings.download.music_hotkey_enabled = false;
    settings
}

pub fn platform_enabled(name: &str) -> bool {
    !matches!(
        name.to_ascii_lowercase().as_str(),
        "p2p" | "telegram" | "telegram_media" | "courses" | "hotmart" | "udemy"
    )
}

/// Match complete host/path components so unrelated downloads keep working.
pub fn is_author_endpoint(url: &url::Url) -> bool {
    let host = url
        .host_str()
        .unwrap_or("")
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let domain = |value: &str| host == value || host.ends_with(&format!(".{value}"));
    if domain("tonho.wtf") || domain("omniget.wtf") || domain("getomniget.com") {
        return true;
    }
    let path = url.path().to_ascii_lowercase();
    match host.as_str() {
        "github.com" | "raw.githubusercontent.com" => {
            path == "/tonhowtf" || path.starts_with("/tonhowtf/")
        }
        "api.github.com" => path.starts_with("/repos/tonhowtf/"),
        "cdn.jsdelivr.net" => path.starts_with("/gh/tonhowtf/"),
        _ => false,
    }
}

/// Use on URL-probing clients so blocked redirect targets are rejected before
/// reqwest connects to the next host, with the usual ten-redirect bound.
pub fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if !url_enabled(attempt.url().as_str()) {
            attempt.error(DISABLED_SERVICE_REASON)
        } else if attempt.previous().len() >= 10 {
            attempt.error("Too many redirects")
        } else {
            attempt.follow()
        }
    })
}

pub fn url_enabled(raw: &str) -> bool {
    let trimmed = raw.trim();
    if let Ok(url) = url::Url::parse(trimmed) {
        if matches!(url.scheme(), "p2p" | "tg" | "telegram") || is_author_endpoint(&url) {
            return false;
        }
    }
    match Platform::from_url(trimmed) {
        Some(Platform::Hotmart | Platform::Udemy | Platform::Telegram) => false,
        Some(Platform::Other(name)) => !matches!(
            name.as_str(),
            "kiwify"
                | "gumroad"
                | "teachable"
                | "kajabi"
                | "skool"
                | "greatcourses"
                | "thinkific"
                | "rocketseat"
        ),
        _ => true,
    }
}

pub fn require_url(raw: &str) -> Result<(), String> {
    if url_enabled(raw) {
        Ok(())
    } else {
        Err(DISABLED_SERVICE_REASON.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_services_and_author_endpoints_are_blocked() {
        for url in [
            "p2p:share-code",
            "P2P:share-code",
            "tg://resolve?domain=example",
            "https://t.me/example",
            "https://www.udemy.com/course/example",
            "https://hotmart.com/example",
            "https://chat.tonho.wtf/",
            "https://chat.tonho.wtf./",
            "https://www.getomniget.com./",
            "https://omniget.wtf/",
            "https://github.com/tonhowtf/omniget/releases",
            "https://raw.githubusercontent.com/tonhowtf/omniget-plugins/main/plugins.json",
            "https://api.github.com/repos/tonhowtf/omniget/releases/latest",
            "https://cdn.jsdelivr.net/gh/tonhowtf/omniget-plugins@main/plugins.json",
        ] {
            assert!(!url_enabled(url), "allowed removed service {url}");
        }
        assert!(require_native_plugins().is_err());
        assert!(require_p2p().is_err());
        assert!(require_discord_rpc().is_err());
    }

    #[tokio::test]
    async fn shared_ytdlp_rejects_removed_services_before_starting_a_process() {
        let error = crate::core::ytdlp::get_video_info(
            std::path::Path::new("missing-yt-dlp-must-never-run"),
            "https://chat.tonho.wtf/",
            &[],
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), DISABLED_SERVICE_REASON);
    }

    #[test]
    fn instagram_local_bridge_and_official_updates_remain_available() {
        for url in [
            "https://www.instagram.com/reel/example/",
            "http://127.0.0.1:8765/",
            "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest",
            "https://github.com/yt-dlp/yt-dlp/releases/latest",
            "https://github.com/tonhowtf-example/project",
            "https://omniget.wtf.example.org/video.mp4",
        ] {
            assert!(url_enabled(url), "blocked retained feature {url}");
        }
    }

    #[test]
    fn settings_cannot_reenable_removed_services_or_change_retained_data() {
        let mut settings = AppSettings::default();
        settings.rpc.enabled = true;
        settings.omnidisc.enabled = true;
        settings.league.enabled = true;
        settings.download.music_hotkey_enabled = true;
        settings.bridge.token = "keep-this-pairing-token".into();
        let mut expected = serde_json::to_value(&settings).unwrap();
        expected["rpc"]["enabled"] = false.into();
        expected["omnidisc"]["enabled"] = false.into();
        expected["league"]["enabled"] = false.into();
        expected["download"]["music_hotkey_enabled"] = false.into();
        assert_eq!(
            serde_json::to_value(normalize_settings(settings)).unwrap(),
            expected
        );
    }
}
