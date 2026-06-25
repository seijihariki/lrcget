use super::{ExportError, ExportFormat, ExportResult, ExportStatus, build_sidecar_path};
use crate::lyricsfile::ParsedLyricsfile;
use crate::persistent_entities::PersistentTrack;
use std::fs::{remove_file, write};

pub fn generate_elrc_content(parsed: &ParsedLyricsfile) -> Option<String> {
    if parsed.is_instrumental {
        return Some(crate::lyricsfile::INSTRUMENTAL_LRC.to_string());
    }

    parsed
        .word_synced_lyrics
        .clone()
        .filter(|s| !s.trim().is_empty())
}

pub fn export_elrc(track: &PersistentTrack, parsed: &ParsedLyricsfile) -> Result<ExportResult, ExportError> {
    let content = match generate_elrc_content(parsed) {
        Some(content) => content,
        None => {
            return Ok(ExportResult {
                format: ExportFormat::Elrc,
                path: None,
                status: ExportStatus::Skipped("no word-synced lyrics available".to_string()),
            });
        }
    };

    let elrc_path = build_sidecar_path(&track.file_path, "elrc")?;

    for conflicting_extension in ["txt", "lrc"] {
        if let Ok(conflicting_path) = build_sidecar_path(&track.file_path, conflicting_extension) {
            let _ = remove_file(conflicting_path);
        }
    }

    write(&elrc_path, content).map_err(|e| ExportError::WriteError(e.to_string()))?;

    Ok(ExportResult {
        format: ExportFormat::Elrc,
        path: Some(elrc_path),
        status: ExportStatus::Success,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_elrc_content() {
        let parsed = ParsedLyricsfile {
            plain_lyrics: None,
            synced_lyrics: Some("[00:12.00]Line 1".to_string()),
            word_synced_lyrics: Some("[00:12.000]<00:12.000>Line 1".to_string()),
            is_instrumental: false,
        };

        let content = generate_elrc_content(&parsed);
        assert_eq!(content, Some("[00:12.000]<00:12.000>Line 1".to_string()));

        let no_word_sync = ParsedLyricsfile {
            plain_lyrics: None,
            synced_lyrics: Some("[00:12.00]Line 1".to_string()),
            word_synced_lyrics: None,
            is_instrumental: false,
        };
        assert_eq!(generate_elrc_content(&no_word_sync), None);
    }
}
