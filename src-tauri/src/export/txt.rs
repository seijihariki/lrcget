use super::{ExportError, ExportFormat, ExportResult, ExportStatus, build_sidecar_path};
use crate::lyricsfile::ParsedLyricsfile;
use crate::persistent_entities::PersistentTrack;
use std::fs::{remove_file, write};

pub fn generate_txt_content(parsed: &ParsedLyricsfile) -> Option<String> {
    if parsed.is_instrumental {
        return None;
    }

    parsed.plain_lyrics.clone().filter(|s| !s.trim().is_empty())
}

pub fn export_txt(track: &PersistentTrack, parsed: &ParsedLyricsfile) -> Result<ExportResult, ExportError> {
    let content = match generate_txt_content(parsed) {
        Some(content) => content,
        None => {
            return Ok(ExportResult {
                format: ExportFormat::Txt,
                path: None,
                status: ExportStatus::Skipped("no plain lyrics available".to_string()),
            });
        }
    };

    let txt_path = build_sidecar_path(&track.file_path, "txt")?;

    // Keep only one sidecar representation for synced formats.
    for conflicting_extension in ["lrc", "elrc"] {
        if let Ok(conflicting_path) = build_sidecar_path(&track.file_path, conflicting_extension) {
            let _ = remove_file(conflicting_path);
        }
    }

    write(&txt_path, content).map_err(|e| ExportError::WriteError(e.to_string()))?;

    Ok(ExportResult {
        format: ExportFormat::Txt,
        path: Some(txt_path),
        status: ExportStatus::Success,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_txt_content() {
        let parsed = ParsedLyricsfile {
            plain_lyrics: Some("Line 1\nLine 2".to_string()),
            synced_lyrics: None,
            word_synced_lyrics: None,
            is_instrumental: false,
        };

        let content = generate_txt_content(&parsed);
        assert_eq!(content, Some("Line 1\nLine 2".to_string()));

        let instrumental = ParsedLyricsfile {
            plain_lyrics: None,
            synced_lyrics: None,
            word_synced_lyrics: None,
            is_instrumental: true,
        };
        assert_eq!(generate_txt_content(&instrumental), None);
    }
}
