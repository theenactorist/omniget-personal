//! Comprimir vídeo para um alvo exato de tamanho (o limite de anexo do
//! Discord, do e-mail, do WhatsApp) com duas passagens do FFmpeg já gerido.
//!
//! O padrão é o mesmo do ffmpeg4discord: mede a duração, divide o orçamento
//! de bits pelo tempo, tira o áudio da conta e roda `-pass 1` / `-pass 2`.
//! Uma passagem só erra o alvo para cima ou para baixo; duas acertam.

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

const WHATSAPP_TARGET_BYTES: u64 = 15_000_000;
const WHATSAPP_LIMIT_BYTES: u64 = 16_000_000;
const WHATSAPP_AUDIO_KBPS: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SharingPreset {
    #[serde(rename = "whatsapp")]
    WhatsApp,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompressOptions {
    pub inputs: Vec<String>,
    /// Alvo em MB (base 1024, que é o que o Discord conta).
    pub target_mb: f64,
    /// "h264" | "h265" | "vp9"
    #[serde(default = "default_codec")]
    pub codec: String,
    /// 0 = escolher sozinho (128k estéreo, 96k mono); o áudio sai do orçamento.
    #[serde(default)]
    pub audio_kbps: u32,
    /// 0 = manter a resolução original.
    #[serde(default)]
    pub max_height: u32,
    /// 0 = manter o frame rate original.
    #[serde(default)]
    pub fps: u32,
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default)]
    pub output_dir: String,
    #[serde(default)]
    pub suffix: String,
    /// O preset de compartilhamento impõe formato, tamanho e qualidade no servidor.
    #[serde(default)]
    pub sharing_preset: Option<SharingPreset>,
}

impl CompressOptions {
    fn is_whatsapp(&self) -> bool {
        self.sharing_preset == Some(SharingPreset::WhatsApp)
    }

    fn target_bytes(&self) -> u64 {
        if self.is_whatsapp() {
            WHATSAPP_TARGET_BYTES
        } else {
            (self.target_mb.max(0.1) * 1024.0 * 1024.0) as u64
        }
    }

    fn container(&self) -> &'static str {
        if self.is_whatsapp() {
            "mp4"
        } else {
            container(&self.codec)
        }
    }
}

fn default_codec() -> String {
    "h264".into()
}

fn default_preset() -> String {
    "medium".into()
}

#[derive(Debug, Clone, Serialize)]
pub struct CompressItem {
    pub input: String,
    pub output: Option<String>,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub target_bytes: u64,
    pub video_kbps: u32,
    pub audio_kbps: u32,
    pub duration_s: f64,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompressResult {
    pub items: Vec<CompressItem>,
}

/// Divide o orçamento de bits entre vídeo e áudio. Devolve `(vídeo, áudio)`
/// em kbps, ou `None` quando nem cortando o áudio o alvo cabe.
///
/// `headroom` desconta o overhead do container (moov box, índices): mirar
/// 100% do alvo estoura por alguns por cento em arquivo curto.
pub fn plan_bitrates(
    duration_s: f64,
    target_bytes: u64,
    audio_kbps: u32,
    has_audio: bool,
    headroom: f64,
) -> Option<(u32, u32)> {
    if !duration_s.is_finite() || duration_s <= 0.0 || target_bytes == 0 {
        return None;
    }
    let total_kbps = (target_bytes as f64 * 8.0 * headroom) / (duration_s * 1000.0);
    if !has_audio {
        let v = total_kbps.floor() as u32;
        return (v >= 60).then_some((v, 0));
    }
    // Vai cedendo o áudio antes de desistir: 96k já é aceitável para fala.
    for a in [audio_kbps, 96, 64, 48, 32] {
        if a == 0 {
            continue;
        }
        let v = total_kbps - a as f64;
        if v >= 60.0 {
            return Some((v.floor() as u32, a));
        }
    }
    None
}

/// Mantém os 64 kbps de áudio do preset mesmo quando o vídeo é longo demais.
fn plan_whatsapp_bitrates(duration_s: f64, has_audio: bool) -> Option<(u32, u32)> {
    plan_bitrates(
        duration_s,
        WHATSAPP_TARGET_BYTES,
        WHATSAPP_AUDIO_KBPS,
        has_audio,
        0.97,
    )
    .filter(|(_, audio)| !has_audio || *audio == WHATSAPP_AUDIO_KBPS)
    // Main level 3.1 permite no máximo 14 Mbps. Clipes curtos não precisam
    // gastar o orçamento inteiro para manter a compatibilidade do nível.
    .map(|(video, audio)| (video.min(14_000), audio))
}

fn null_sink() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}

fn container(codec: &str) -> &'static str {
    if codec == "vp9" {
        "webm"
    } else {
        "mp4"
    }
}

fn video_encoder(codec: &str) -> &'static str {
    match codec {
        "h265" | "hevc" => "libx265",
        "vp9" => "libvpx-vp9",
        _ => "libx264",
    }
}

fn audio_encoder(codec: &str) -> &'static str {
    if codec == "vp9" {
        "libopus"
    } else {
        "aac"
    }
}

fn scale_filter(max_height: u32) -> Option<String> {
    (max_height > 0).then(|| {
        format!(
            "scale=-2:'min({h},ih)':flags=lanczos",
            h = max_height - max_height % 2
        )
    })
}

fn whatsapp_scale_filter() -> &'static str {
    // DAR considera pixels não quadrados, e é avaliado após a autorrotação.
    // reset_sar ajusta as dimensões antes de normalizar os pixels para 1:1.
    // Cada eixo fica no máximo do tamanho de entrada, mesmo em clipes pequenos.
    "scale=w='min(iw,if(gte(dar,1),854,480))':h='min(ih,if(gte(dar,1),480,854))':force_original_aspect_ratio=decrease:force_divisible_by=2:flags=lanczos:reset_sar=1,setsar=1"
}

/// Argumentos comuns às duas passagens (entrada, filtros, bitrate de vídeo).
fn common_args(opts: &CompressOptions, video_kbps: u32) -> Vec<String> {
    if opts.is_whatsapp() {
        return vec![
            "-map".into(),
            "0:v:0".into(),
            "-map".into(),
            "0:a:0?".into(),
            "-map_metadata".into(),
            "-1".into(),
            "-map_metadata:s".into(),
            "-1".into(),
            "-map_chapters".into(),
            "-1".into(),
            "-c:v".into(),
            "libx264".into(),
            "-b:v".into(),
            format!("{}k", video_kbps),
            "-maxrate".into(),
            "14000k".into(),
            "-bufsize".into(),
            "14000k".into(),
            "-preset".into(),
            "slow".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            "-profile:v".into(),
            "main".into(),
            "-level:v".into(),
            "3.1".into(),
            "-tag:v".into(),
            "avc1".into(),
            "-vf".into(),
            whatsapp_scale_filter().into(),
            "-r".into(),
            "24".into(),
            "-fps_mode".into(),
            "cfr".into(),
        ];
    }
    let mut a: Vec<String> = vec![
        "-c:v".into(),
        video_encoder(&opts.codec).into(),
        "-b:v".into(),
        format!("{}k", video_kbps),
    ];
    if opts.codec != "vp9" {
        a.push("-preset".into());
        a.push(opts.preset.clone());
    }
    if let Some(f) = scale_filter(opts.max_height) {
        a.push("-vf".into());
        a.push(f);
    }
    if opts.fps > 0 {
        a.push("-r".into());
        a.push(opts.fps.to_string());
    }
    a
}

fn output_args(opts: &CompressOptions, has_audio: bool, audio_kbps: u32) -> Vec<String> {
    let mut tail = Vec::new();
    if has_audio && audio_kbps > 0 {
        tail.extend([
            "-c:a".into(),
            if opts.is_whatsapp() {
                "aac".into()
            } else {
                audio_encoder(&opts.codec).into()
            },
            "-b:a".into(),
            format!("{}k", audio_kbps),
        ]);
        if opts.is_whatsapp() {
            tail.extend([
                "-profile:a".into(),
                "aac_low".into(),
                "-ac".into(),
                "1".into(),
                "-ar".into(),
                "44100".into(),
            ]);
        }
    } else {
        tail.push("-an".into());
    }
    if opts.container() == "mp4" {
        tail.extend(["-movflags".into(), "+faststart".into()]);
    }
    tail
}

async fn run_pass(
    ffmpeg: &Path,
    input: &Path,
    args: &[String],
    pass: u8,
    log: &Path,
    tail: &[String],
) -> anyhow::Result<()> {
    let mut cmd = crate::core::process::command(ffmpeg);
    cmd.args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(args)
        .args(["-pass", &pass.to_string()])
        .arg("-passlogfile")
        .arg(log)
        .args(tail);
    let out = cmd
        .output()
        .await
        .map_err(|e| anyhow!("ffmpeg nao iniciou: {}", e))?;
    if !out.status.success() {
        return Err(anyhow!(
            "ffmpeg passagem {}: {}",
            pass,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

fn clean_logs(log: &Path) {
    // Esta pasta privada foi criada exclusivamente para esta execução. Removê-la
    // inteira também cobre os arquivos .temp deixados por uma passagem que falha.
    if let Some(dir) = log.parent() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

struct PassLogs(PathBuf);

impl Drop for PassLogs {
    fn drop(&mut self) {
        clean_logs(&self.0);
    }
}

/// Reserva o nome atomicamente: o `-y` do FFmpeg só pode substituir o arquivo
/// vazio que esta execução criou, nunca uma saída anterior ou o original.
struct ReservedOutput {
    path: PathBuf,
    complete: bool,
}

impl Drop for ReservedOutput {
    fn drop(&mut self) {
        if !self.complete {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn reserve_whatsapp_output(out_dir: &Path, input: &Path) -> anyhow::Result<ReservedOutput> {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "video".into());
    for number in 1..=10_000 {
        let name = if number == 1 {
            format!("{}-whatsapp.mp4", stem)
        } else {
            format!("{}-whatsapp-{}.mp4", stem, number)
        };
        let path = out_dir.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => {
                return Ok(ReservedOutput {
                    path,
                    complete: false,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(anyhow!(
        "Could not reserve a new filename for the WhatsApp video."
    ))
}

fn check_whatsapp_size(bytes_after: u64) -> anyhow::Result<()> {
    if bytes_after == 0 || bytes_after >= WHATSAPP_LIMIT_BYTES {
        return Err(anyhow!(
            "The encoded video is {} bytes. This sharing preset requires a nonempty video below {} bytes. Choose a shorter clip.",
            bytes_after,
            WHATSAPP_LIMIT_BYTES
        ));
    }
    Ok(())
}

async fn compress_one(
    ffmpeg: &Path,
    opts: &CompressOptions,
    input: &str,
) -> anyhow::Result<CompressItem> {
    let inp = Path::new(input);
    let target_bytes = opts.target_bytes();
    let bytes_before = std::fs::metadata(inp).map(|m| m.len()).unwrap_or(0);

    let probe = crate::core::ffmpeg::probe(inp).await?;
    let has_audio = probe.streams.iter().any(|s| s.codec_type == "audio");
    let channels = probe
        .streams
        .iter()
        .find(|s| s.codec_type == "audio")
        .and_then(|s| s.channels)
        .unwrap_or(2);
    let wanted_audio = if opts.audio_kbps > 0 {
        opts.audio_kbps
    } else if channels <= 1 {
        96
    } else {
        128
    };
    let planned = if opts.is_whatsapp() {
        plan_whatsapp_bitrates(probe.duration_seconds, has_audio)
    } else {
        plan_bitrates(
            probe.duration_seconds,
            target_bytes,
            wanted_audio,
            has_audio,
            0.97,
        )
    };
    let (video_kbps, audio_kbps) = planned.ok_or_else(|| {
        if opts.is_whatsapp() {
            return anyhow!(
                "This {:.0}s video is too long to preserve in full{} below this sharing preset's 16 MB target. Choose a shorter clip.",
                probe.duration_seconds,
                if has_audio { " with sound" } else { "" }
            );
        }
        anyhow!(
            "alvo pequeno demais para {:.0}s de vídeo — aumente o tamanho ou corte o clipe",
            probe.duration_seconds
        )
    })?;

    let out_dir = if opts.output_dir.trim().is_empty() {
        inp.parent().map(|p| p.to_path_buf()).unwrap_or_default()
    } else {
        PathBuf::from(opts.output_dir.trim())
    };
    std::fs::create_dir_all(&out_dir)?;
    let mut reserved_output = if opts.is_whatsapp() {
        Some(reserve_whatsapp_output(&out_dir, inp)?)
    } else {
        None
    };
    let output = if let Some(reserved) = &reserved_output {
        reserved.path.clone()
    } else {
        let stem = inp
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "video".into());
        let suffix = if opts.suffix.is_empty() {
            "-small"
        } else {
            opts.suffix.as_str()
        };
        out_dir.join(format!("{}{}.{}", stem, suffix, opts.container()))
    };

    let log_dir = std::env::temp_dir().join(format!("omniget-2pass-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&log_dir)?;
    let logs = PassLogs(log_dir.join("pass"));

    let mut kbps = video_kbps;
    let mut bytes_after = 0u64;
    // Uma correção só: se a primeira tentativa passar do alvo, reencaixa o
    // bitrate na proporção do estouro e refaz a passagem 2 (o log da 1 vale).
    for attempt in 0..2u8 {
        let args = common_args(opts, kbps);
        let tail = output_args(opts, has_audio, audio_kbps);

        if attempt == 0 {
            let first: Vec<String> = vec![
                "-an".into(),
                "-f".into(),
                if opts.container() == "webm" {
                    "webm".into()
                } else {
                    "mp4".into()
                },
                null_sink().into(),
            ];
            run_pass(ffmpeg, inp, &args, 1, &logs.0, &first).await?;
        }
        let mut second = tail.clone();
        second.push(output.to_string_lossy().to_string());
        run_pass(ffmpeg, inp, &args, 2, &logs.0, &second).await?;

        bytes_after = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
        if bytes_after <= target_bytes || attempt == 1 {
            break;
        }
        let ratio = target_bytes as f64 / bytes_after as f64;
        let next = (kbps as f64 * ratio * 0.97).floor() as u32;
        if next < 60 || next >= kbps {
            break;
        }
        kbps = next;
    }
    if opts.is_whatsapp() {
        check_whatsapp_size(bytes_after)?;
    }
    if let Some(reserved) = &mut reserved_output {
        reserved.complete = true;
    }

    Ok(CompressItem {
        input: input.to_string(),
        output: Some(output.to_string_lossy().to_string()),
        bytes_before,
        bytes_after,
        target_bytes,
        video_kbps: kbps,
        audio_kbps: if has_audio { audio_kbps } else { 0 },
        duration_s: probe.duration_seconds,
        ok: true,
        error: None,
    })
}

pub async fn run(
    opts: CompressOptions,
    progress: super::ProgressFn,
) -> anyhow::Result<CompressResult> {
    let ffmpeg = crate::core::dependencies::ensure_ffmpeg().await?;
    let total = opts.inputs.len() as u64;
    let mut items = Vec::new();
    for (i, input) in opts.inputs.iter().enumerate() {
        super::report(
            &progress,
            "video-compress",
            "progress",
            i as u64,
            Some(total),
            Some(input.clone()),
        );
        match compress_one(&ffmpeg, &opts, input).await {
            Ok(item) => items.push(item),
            Err(e) => {
                tracing::warn!("[video-compress] {}: {}", input, e);
                items.push(CompressItem {
                    input: input.clone(),
                    output: None,
                    bytes_before: std::fs::metadata(input).map(|m| m.len()).unwrap_or(0),
                    bytes_after: 0,
                    target_bytes: opts.target_bytes(),
                    video_kbps: 0,
                    audio_kbps: 0,
                    duration_s: 0.0,
                    ok: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    super::report(
        &progress,
        "video-compress",
        "done",
        total,
        Some(total),
        None,
    );
    Ok(CompressResult { items })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn whatsapp_options() -> CompressOptions {
        // Deliberately conflicting manual values must have no effect on the preset.
        serde_json::from_value(serde_json::json!({
            "inputs": [], "target_mb": 1000.0, "codec": "vp9",
            "audio_kbps": 192, "max_height": 2160, "fps": 60,
            "preset": "ultrafast", "suffix": "/../../original",
            "sharing_preset": "whatsapp"
        }))
        .unwrap()
    }

    fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
        args.windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].as_str())
    }

    fn test_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("omniget-compress-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sharing_preset_is_optional_and_rejects_unknown_values() {
        for value in [
            serde_json::json!({"inputs": [], "target_mb": 8.0}),
            serde_json::json!({"inputs": [], "target_mb": 8.0, "sharing_preset": null}),
        ] {
            let opts: CompressOptions = serde_json::from_value(value).unwrap();
            assert!(!opts.is_whatsapp());
            assert_eq!(opts.target_bytes(), 8 * 1024 * 1024);
        }
        assert!(whatsapp_options().is_whatsapp());
        for preset in ["whats_app", "telegram", ""] {
            assert!(
                serde_json::from_value::<CompressOptions>(serde_json::json!({
                    "inputs": [], "target_mb": 8.0, "sharing_preset": preset
                }))
                .is_err()
            );
        }
    }

    #[test]
    fn whatsapp_budget_uses_decimal_bytes_and_fixed_audio() {
        let opts = whatsapp_options();
        assert_eq!(opts.target_bytes(), 15_000_000);
        let duration = 60.0;
        let (video, audio) = plan_whatsapp_bitrates(duration, true).unwrap();
        assert_eq!(audio, 64);
        assert_eq!(video, 1876);
        let estimated_bytes = (video + audio) as f64 * 1000.0 * duration / 8.0;
        assert!(estimated_bytes <= 15_000_000.0 * 0.97);
        assert!(estimated_bytes < WHATSAPP_LIMIT_BYTES as f64);
        assert_eq!(plan_whatsapp_bitrates(duration, false), Some((1940, 0)));
        assert_eq!(plan_whatsapp_bitrates(1.0, true), Some((14_000, 64)));
    }

    #[test]
    fn whatsapp_refuses_impossible_duration_instead_of_degrading_audio() {
        assert!(plan_whatsapp_bitrates(1200.0, true).is_none());
        assert!(plan_whatsapp_bitrates(1200.0, false).is_some());
        for duration in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(plan_whatsapp_bitrates(duration, true).is_none());
        }
    }

    #[test]
    fn whatsapp_arguments_enforce_format_and_keep_the_whole_clip() {
        let opts = whatsapp_options();
        let args = common_args(&opts, 1000);
        for (name, value) in [
            ("-c:v", "libx264"),
            ("-b:v", "1000k"),
            ("-preset", "slow"),
            ("-pix_fmt", "yuv420p"),
            ("-profile:v", "main"),
            ("-level:v", "3.1"),
            ("-tag:v", "avc1"),
            ("-r", "24"),
            ("-fps_mode", "cfr"),
            ("-map_metadata", "-1"),
            ("-map_metadata:s", "-1"),
            ("-map_chapters", "-1"),
        ] {
            assert_eq!(arg_value(&args, name), Some(value), "{}", name);
        }
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:v:0"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:a:0?"]));
        let tail = output_args(&opts, true, 64);
        for (name, value) in [
            ("-c:a", "aac"),
            ("-profile:a", "aac_low"),
            ("-b:a", "64k"),
            ("-ac", "1"),
            ("-ar", "44100"),
            ("-movflags", "+faststart"),
        ] {
            assert_eq!(arg_value(&tail, name), Some(value), "{}", name);
        }
        assert_eq!(opts.container(), "mp4");
        for flag in ["-shortest", "-t", "-to", "-ss", "-fs"] {
            assert!(!args.iter().chain(tail.iter()).any(|arg| arg == flag));
        }
        let silent = output_args(&opts, false, 0);
        assert!(silent.iter().any(|arg| arg == "-an"));
        assert!(arg_value(&silent, "-c:a").is_none());
    }

    #[test]
    fn generic_arguments_keep_manual_options() {
        let mut opts = whatsapp_options();
        opts.sharing_preset = None;
        let args = common_args(&opts, 1000);
        assert_eq!(arg_value(&args, "-c:v"), Some("libvpx-vp9"));
        assert_eq!(arg_value(&args, "-r"), Some("60"));
        assert!(arg_value(&args, "-profile:v").is_none());
        assert_eq!(opts.container(), "webm");
        let tail = output_args(&opts, true, 192);
        assert_eq!(arg_value(&tail, "-c:a"), Some("libopus"));
        assert_eq!(arg_value(&tail, "-b:a"), Some("192k"));
        assert!(arg_value(&tail, "-ac").is_none());
    }

    #[test]
    fn whatsapp_requires_nonempty_output_strictly_below_the_limit() {
        assert!(check_whatsapp_size(0).is_err());
        assert!(check_whatsapp_size(15_000_000).is_ok());
        assert!(check_whatsapp_size(15_999_999).is_ok());
        assert!(check_whatsapp_size(16_000_000).is_err());
        assert!(check_whatsapp_size(16_000_001).is_err());
    }

    #[test]
    fn whatsapp_output_reservation_preserves_existing_files_and_cleans_failures() {
        let dir = test_dir();
        let input = dir.join("reel.mp4");
        let existing = dir.join("reel-whatsapp.mp4");
        std::fs::write(&input, b"original").unwrap();
        std::fs::write(&existing, b"previous output").unwrap();
        let first = reserve_whatsapp_output(&dir, &input).unwrap();
        assert_eq!(first.path, dir.join("reel-whatsapp-2.mp4"));
        let mut second = reserve_whatsapp_output(&dir, &input).unwrap();
        assert_eq!(second.path, dir.join("reel-whatsapp-3.mp4"));
        second.complete = true;
        let first_path = first.path.clone();
        let second_path = second.path.clone();
        drop(first);
        drop(second);
        assert!(!first_path.exists());
        assert!(second_path.exists());
        assert_eq!(std::fs::read(input).unwrap(), b"original");
        assert_eq!(std::fs::read(existing).unwrap(), b"previous output");
        assert_eq!(second_path.parent(), Some(dir.as_path()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pass_logs_are_cleaned_when_a_pass_fails() {
        let dir = test_dir();
        let log = dir.join("pass");
        for suffix in [
            "-0.log",
            "-0.log.mbtree",
            "-0.log.temp",
            "-0.log.mbtree.temp",
        ] {
            std::fs::write(format!("{}{}", log.display(), suffix), b"pass data").unwrap();
        }
        drop(PassLogs(log));
        assert!(!dir.exists());
    }

    #[test]
    fn budget_splits_audio_out() {
        // 10 MB para 60 s: ~1356 kbps no total, menos 128 de áudio.
        let (v, a) = plan_bitrates(60.0, 10 * 1024 * 1024, 128, true, 0.97).unwrap();
        assert_eq!(a, 128);
        assert!((1200..1260).contains(&v), "bitrate fora da faixa: {}", v);
    }

    #[test]
    fn budget_gives_everything_to_video_when_muted() {
        let (v, a) = plan_bitrates(60.0, 10 * 1024 * 1024, 128, false, 0.97).unwrap();
        assert_eq!(a, 0);
        assert!(v > 1300);
    }

    #[test]
    fn budget_degrades_audio_before_failing() {
        // 8 MB para 10 min: com 128k de áudio não sobra vídeo; tem que ceder.
        let (v, a) = plan_bitrates(600.0, 8 * 1024 * 1024, 128, true, 0.97).unwrap();
        assert_eq!(a, 48);
        assert!(v >= 60, "vídeo ficou abaixo do piso: {}", v);
    }

    #[test]
    fn budget_refuses_the_impossible() {
        assert!(plan_bitrates(7200.0, 1024 * 1024, 128, true, 0.97).is_none());
        // 8 MB para 30 min não cabe nem cortando o áudio ao mínimo.
        assert!(plan_bitrates(1800.0, 8 * 1024 * 1024, 128, true, 0.97).is_none());
        assert!(plan_bitrates(0.0, 10 * 1024 * 1024, 128, true, 0.97).is_none());
    }

    #[test]
    fn scale_keeps_even_dimensions() {
        assert_eq!(scale_filter(0), None);
        assert!(scale_filter(721).unwrap().contains("min(720,ih)"));
    }

    /// Exercise the actual FFmpeg expression, including anamorphic, odd and small sources.
    /// `cargo test -p omniget-core --lib live_whatsapp_scale -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn live_whatsapp_scale_preserves_orientation_without_upscale() {
        let ffmpeg = crate::core::dependencies::ensure_ffmpeg().await.unwrap();
        let dir = test_dir();
        for (width, height, sar, expected_width, expected_height) in [
            (1080, 1920, "1/1", 480, 854),
            (1920, 1080, "1/1", 854, 480),
            (320, 180, "1/1", 320, 180),
            (180, 320, "1/1", 180, 320),
            (321, 181, "1/1", 320, 180),
            (181, 321, "1/1", 180, 320),
            (1000, 1000, "1/1", 480, 480),
            (720, 480, "8/9", 640, 480),
            (720, 480, "2/1", 720, 240),
            (1920, 1080, "1/2", 480, 540),
            (1080, 1920, "2/1", 540, 480),
        ] {
            let output = dir.join(format!("{}x{}.png", width, height));
            let generated = crate::core::process::command(&ffmpeg)
                .args([
                    "-y",
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                ])
                .arg(format!(
                    "testsrc=size={}x{}:rate=1:duration=1,setsar={}",
                    width, height, sar
                ))
                .args(["-vf", whatsapp_scale_filter(), "-frames:v", "1"])
                .arg(&output)
                .output()
                .await
                .unwrap();
            assert!(
                generated.status.success(),
                "{}x{}: {}",
                width,
                height,
                String::from_utf8_lossy(&generated.stderr)
            );
            let inspected = crate::core::process::command("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-select_streams",
                    "v:0",
                    "-show_entries",
                    "stream=width,height,sample_aspect_ratio",
                    "-of",
                    "json",
                ])
                .arg(&output)
                .output()
                .await
                .unwrap();
            assert!(inspected.status.success());
            let probe: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
            let video = &probe["streams"][0];
            let actual = (
                video["width"].as_u64().unwrap() as u32,
                video["height"].as_u64().unwrap() as u32,
            );
            assert_eq!(actual, (expected_width, expected_height));
            assert!(actual.0 <= width && actual.1 <= height);
            assert_eq!((actual.0 % 2, actual.1 % 2), (0, 0));
            assert_eq!(video["sample_aspect_ratio"].as_str(), Some("1:1"));
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Ponta a ponta com o FFmpeg gerido: gera um clipe, comprime para 1 MB
    /// e confere que o alvo foi respeitado sem perder áudio nem duração.
    /// `cargo test -p omniget-core --lib -- --ignored live_compress`
    #[tokio::test]
    #[ignore]
    async fn live_compress_hits_the_target() {
        let ffmpeg = crate::core::dependencies::ensure_ffmpeg().await.unwrap();
        let dir = std::env::temp_dir().join("omniget-vcompress-live");
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("fonte.mp4");
        let gen = crate::core::process::command(&ffmpeg)
            .args([
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=1280x720:rate=30:duration=20",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=20",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-crf",
                "18",
                "-c:a",
                "aac",
            ])
            .arg(&src)
            .output()
            .await
            .unwrap();
        assert!(gen.status.success(), "não consegui gerar o clipe de teste");
        let original = std::fs::metadata(&src).unwrap().len();

        let opts = CompressOptions {
            inputs: vec![src.to_string_lossy().to_string()],
            target_mb: 1.0,
            codec: "h264".into(),
            audio_kbps: 0,
            max_height: 0,
            fps: 0,
            preset: "veryfast".into(),
            output_dir: dir.to_string_lossy().to_string(),
            suffix: String::new(),
            sharing_preset: None,
        };
        let res = run(opts, crate::core::tools::noop_progress())
            .await
            .unwrap();
        let item = &res.items[0];
        assert!(item.ok, "falhou: {:?}", item.error);
        assert!(
            item.bytes_after <= item.target_bytes,
            "{} bytes passaram do alvo de {}",
            item.bytes_after,
            item.target_bytes
        );
        assert!(item.bytes_after < original, "não comprimiu nada");

        let out = item.output.as_ref().unwrap();
        let probe = crate::core::ffmpeg::probe(std::path::Path::new(out))
            .await
            .unwrap();
        assert!(
            (probe.duration_seconds - 20.0).abs() < 1.0,
            "duração mudou: {}",
            probe.duration_seconds
        );
        assert!(
            probe.streams.iter().any(|s| s.codec_type == "audio"),
            "perdeu o áudio"
        );
        eprintln!(
            "{} → {} bytes (alvo {}), vídeo {} kbps",
            original, item.bytes_after, item.target_bytes, item.video_kbps
        );
        // O log de duas passagens não pode ficar para trás.
        assert!(
            !std::fs::read_dir(&dir)
                .unwrap()
                .flatten()
                .any(|e| e.file_name().to_string_lossy().contains("pass")),
            "sobrou log de passagem na pasta de saída"
        );
    }

    #[test]
    fn containers_follow_the_codec() {
        assert_eq!(container("vp9"), "webm");
        assert_eq!(container("h265"), "mp4");
        assert_eq!(video_encoder("h265"), "libx265");
        assert_eq!(audio_encoder("vp9"), "libopus");
    }
}
