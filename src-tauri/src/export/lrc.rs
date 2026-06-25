use super::{ExportError, ExportFormat, ExportResult, ExportStatus, build_sidecar_path};
use crate::lyricsfile::ParsedLyricsfile;
use crate::persistent_entities::PersistentTrack;
use std::fs::{remove_file, write};

pub fn generate_lrc_content(parsed: &ParsedLyricsfile) -> Option<String> {
    if parsed.is_instrumental {
        return Some(crate::lyricsfile::INSTRUMENTAL_LRC.to_string());
    }

    parsed.synced_lyrics.clone().filter(|s| !s.trim().is_empty())
}

pub fn export_lrc(track: &PersistentTrack, parsed: &ParsedLyricsfile) -> Result<ExportResult, ExportError> {
    let content = match generate_lrc_content(parsed) {
        Some(content) => content,
        None => {
            return Ok(ExportResult {
                format: ExportFormat::Lrc,
                path: None,
                status: ExportStatus::Skipped("no synced lyrics available".to_string()),
            });
        }
    };

    let lrc_path = build_sidecar_path(&track.file_path, "lrc")?;

    for conflicting_extension in ["txt", "elrc"] {
        if let Ok(conflicting_path) = build_sidecar_path(&track.file_path, conflicting_extension) {
            let _ = remove_file(conflicting_path);
        }
    }

    write(&lrc_path, content).map_err(|e| ExportError::WriteError(e.to_string()))?;

    Ok(ExportResult {
        format: ExportFormat::Lrc,
        path: Some(lrc_path),
        status: ExportStatus::Success,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lrc_content() {
        let parsed = ParsedLyricsfile {
            plain_lyrics: None,
            synced_lyrics: Some("[00:12.00]Line 1".to_string()),
            word_synced_lyrics: None,
            is_instrumental: false,
        };

        let content = generate_lrc_content(&parsed);
        assert_eq!(content, Some("[00:12.00]Line 1".to_string()));

        let instrumental = ParsedLyricsfile {
            plain_lyrics: None,
            synced_lyrics: None,
            word_synced_lyrics: None,
            is_instrumental: true,
        };
        assert_eq!(
            generate_lrc_content(&instrumental),
            Some("[au: instrumental]".to_string())
        );
    }
}
