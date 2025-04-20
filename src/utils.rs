use base64::{Engine as _, engine::general_purpose::STANDARD as base64_standard};
use crate::{AppError, Result};
use crate::api::Song;

pub const RESET: &str = "\x1b[0m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const CYAN: &str = "\x1b[36m";
pub const YELLOW: &str = "\x1b[33m";

#[macro_export]
macro_rules! log_info { ($($arg:tt)*) => { println!("\n{}[信息]{} {}", $crate::utils::CYAN, $crate::utils::RESET, format!($($arg)*)) } }
#[macro_export]
macro_rules! log_success { ($($arg:tt)*) => { println!("\n{}[成功]{} {}", $crate::utils::GREEN, $crate::utils::RESET, format!($($arg)*)) } }
#[macro_export]
macro_rules! log_error { ($($arg:tt)*) => { eprintln!("\n{}[错误]{} {}", $crate::utils::RED, $crate::utils::RESET, format!($($arg)*)) } }
#[macro_export]
macro_rules! log_warn { ($($arg:tt)*) => { eprintln!("\n{}[警告]{} {}", $crate::utils::YELLOW, $crate::utils::RESET, format!($($arg)*)) } }

pub fn resolve_resp_json(callback_sign: &str, val: &str) -> Result<String> {
    if !val.starts_with(callback_sign) || !val.ends_with(')') {
        return Err(AppError::ApiError(format!(
            "JSONP 格式无效。 预期为 '{} (...)' ，但实际为: '{}'",
            callback_sign,
            val.chars().take(50).collect::<String>()
        )));
    }

    val.find('(')
       .zip(val.rfind(')'))
       .filter(|(start, end)| start < end)
       .map(|(start, end)| val[start + 1..end].to_string())
       .ok_or_else(|| AppError::ApiError(format!(
           "无法从 JSONP 中提取 JSON： '{}'",
           val.chars().take(50).collect::<String>()
       )))
}

pub fn create_safe_filename(song: &Song) -> String {
    let sanitize = |s: &str| -> String {
        s.chars()
         .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
         .collect::<String>()
         .split_whitespace()
         .filter(|part| !part.is_empty())
         .collect::<Vec<_>>()
         .join("_")
    };

    let safe_song_name = sanitize(&song.name);
    let safe_artist_name = song.singer.iter()
                               .map(|s| sanitize(&s.name))
                               .filter(|s| !s.is_empty())
                               .collect::<Vec<_>>()
                               .join("_");

    let final_artist = if safe_artist_name.is_empty() { "未知艺人" } else { &safe_artist_name };
    let final_song = if safe_song_name.is_empty() { "未知歌曲" } else { &safe_song_name };

    format!("{} - {}", final_artist, final_song)
}

pub fn decode_base64(encoded: &str) -> Result<String> {
    let bytes = base64_standard.decode(encoded)?;
    String::from_utf8(bytes).map_err(AppError::Utf8)
}