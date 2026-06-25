use crate::lyricsfile::ParsedLyricsfile;
use crate::persistent_entities::PersistentTrack;
use serde::Serialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod elrc;
pub mod embedded;
pub mod lrc;
pub mod txt;

/// Errors that can occur during export operations
#[derive(Error, Debug)]
pub enum ExportError {
    #[error("Failed to build export path: {0}")]
    PathBuildError(String),

    #[error("Failed to write file: {0}")]
    WriteError(String),

    #[error("Failed to embed lyrics: {0}")]
    EmbedError(String),
}

/// Export format types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Plain text format (.txt)
    Txt,
    /// Standard LRC format (.lrc)
    Lrc,
    /// Enhanced LRC format with word-level timing (.elrc)
    Elrc,
    /// Embedded in audio file metadata
    Embedded,
}

/// Status of an export operation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "message")]
pub enum ExportStatus {
    /// Export was successful
    Success,
    /// Export was skipped (e.g., no lyrics available for this format)
    Skipped(String),
    /// Export failed with an error
    Error(String),
}

/// Result of an export operation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub format: ExportFormat,
    pub path: Option<PathBuf>,
    pub status: ExportStatus,
}

/// Build the file path for a lyrics sidecar file.
pub fn build_sidecar_path(track_path: &str, extension: &str) -> Result<PathBuf, ExportError> {
    let path = Path::new(track_path);
    let parent_path = path
        .parent()
        .ok_or_else(|| ExportError::PathBuildError("Track has no parent directory".to_string()))?;
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ExportError::PathBuildError("Invalid track filename".to_string()))?;

    Ok(parent_path.join(format!("{}.{}", file_stem, extension)))
}

fn handler_for(format: ExportFormat) -> fn(&PersistentTrack, &ParsedLyricsfile) -> Result<ExportResult, ExportError> {
    match format {
        ExportFormat::Txt => txt::export_txt,
        ExportFormat::Lrc => lrc::export_lrc,
        ExportFormat::Elrc => elrc::export_elrc,
        ExportFormat::Embedded => embedded::export_embedded,
    }
}

/// Export lyrics for a single track in the specified format.
pub fn export_track_format(
    track: &PersistentTrack,
    parsed: &ParsedLyricsfile,
    format: ExportFormat,
) -> Result<ExportResult, ExportError> {
    let handler = handler_for(format);
    handler(track, parsed)
}

/// Export lyrics for a track in multiple formats.
pub fn export_track(
    track: &PersistentTrack,
    parsed: &ParsedLyricsfile,
    formats: &[ExportFormat],
) -> Vec<ExportResult> {
    let mut results = Vec::with_capacity(formats.len());

    for format in formats {
        match export_track_format(track, parsed, *format) {
            Ok(result) => results.push(result),
            Err(e) => results.push(ExportResult {
                format: *format,
                path: None,
                status: ExportStatus::Error(e.to_string()),
            }),
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_sidecar_path() {
        let track_path = "/music/artist/album/song.mp3";
        let txt_path = build_sidecar_path(track_path, "txt").unwrap();
        assert_eq!(txt_path.to_str().unwrap(), "/music/artist/album/song.txt");

        let lrc_path = build_sidecar_path(track_path, "lrc").unwrap();
        assert_eq!(lrc_path.to_str().unwrap(), "/music/artist/album/song.lrc");

        let elrc_path = build_sidecar_path(track_path, "elrc").unwrap();
        assert_eq!(elrc_path.to_str().unwrap(), "/music/artist/album/song.elrc");
    }
}
