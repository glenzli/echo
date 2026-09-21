//! File discovery and desktop intake share this extension registry.
//! An extension is a candidate for probing, never proof that its codec is decodable.

/// Audio files and containers whose audio streams Echo can inspect.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "wave", "bwf", "rf64", "w64", "flac", "mp3", "m4a", "m4b", "aac", "aiff", "aif", "aifc",
    "caf", "ogg", "oga", "opus", "wma", "amr", "ape", "wv", "mp4", "mov", "m4v", "webm", "mka",
    "mkv", "3gp", "3g2",
];

/// File dialog glob patterns, in the same order as directory discovery.
#[must_use]
pub fn audio_file_patterns() -> String {
    AUDIO_EXTENSIONS
        .iter()
        .map(|extension| format!("*.{extension}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Upload hints for the same intake family; the runtime still validates actual bytes.
pub(crate) fn audio_content_type(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav" | "wave" | "bwf" | "rf64") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("m4a" | "m4b" | "mp4" | "m4v") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("aiff" | "aif" | "aifc") => "audio/aiff",
        Some("caf") => "audio/x-caf",
        Some("ogg" | "oga" | "opus") => "audio/ogg",
        Some("wma") => "audio/x-ms-wma",
        Some("amr") => "audio/amr",
        Some("webm") => "audio/webm",
        Some("mka" | "mkv") => "audio/x-matroska",
        Some("mov") => "video/quicktime",
        Some("3gp") => "audio/3gpp",
        Some("3g2") => "audio/3gpp2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests;
