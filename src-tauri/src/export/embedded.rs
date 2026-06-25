use super::{ExportError, ExportFormat, ExportResult, ExportStatus};
use crate::lyricsfile::ParsedLyricsfile;
use crate::parser::lrc::parse_lrc;
use crate::persistent_entities::PersistentTrack;
use anyhow::{Context, Result};
use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::file::AudioFile;
use lofty::flac::FlacFile;
use lofty::id3::v2::{
    BinaryFrame, CommentFrame, Frame, FrameId, Id3v2Tag, SyncTextContentType,
    SynchronizedTextFrame, TimestampFormat, UnsynchronizedTextFrame,
};
use std::io::Seek;
use std::path::PathBuf;

pub fn export_embedded(track: &PersistentTrack, parsed: &ParsedLyricsfile) -> Result<ExportResult, ExportError> {
    let plain_lyrics = parsed.plain_lyrics.clone().unwrap_or_default();
    let synced_lyrics = if parsed.is_instrumental {
        crate::lyricsfile::INSTRUMENTAL_LRC.to_string()
    } else {
        parsed.synced_lyrics.clone().unwrap_or_default()
    };

    embed_lyrics(&track.file_path, &plain_lyrics, &synced_lyrics)
        .map_err(|e| ExportError::EmbedError(e.to_string()))?;

    Ok(ExportResult {
        format: ExportFormat::Embedded,
        path: Some(PathBuf::from(&track.file_path)),
        status: ExportStatus::Success,
    })
}

/// Embed lyrics into audio file metadata (MP3/FLAC).
pub fn embed_lyrics(track_path: &str, plain_lyrics: &str, synced_lyrics: &str) -> Result<()> {
    let path_lower = track_path.to_lowercase();

    if path_lower.ends_with(".mp3") {
        embed_lyrics_mp3(track_path, plain_lyrics, synced_lyrics)
    } else if path_lower.ends_with(".flac") {
        embed_lyrics_flac(track_path, plain_lyrics, synced_lyrics)
    } else {
        Ok(())
    }
}

fn embed_lyrics_flac(track_path: &str, plain_lyrics: &str, synced_lyrics: &str) -> Result<()> {
    use lofty::config::ParseOptions;
    use std::fs::OpenOptions;

    let mut file_content = OpenOptions::new()
        .read(true)
        .write(true)
        .open(track_path)
        .context("Failed to open FLAC file")?;

    let mut flac_file = FlacFile::read_from(&mut file_content, ParseOptions::new())
        .context("Failed to parse FLAC file")?;

    if let Some(vorbis_comments) = flac_file.vorbis_comments_mut() {
        if !plain_lyrics.is_empty() {
            vorbis_comments.insert("UNSYNCEDLYRICS".to_string(), plain_lyrics.to_string());
        } else {
            let _ = vorbis_comments.remove("UNSYNCEDLYRICS");
        }

        if !synced_lyrics.is_empty() {
            vorbis_comments.insert("LYRICS".to_string(), synced_lyrics.to_string());
        } else {
            let _ = vorbis_comments.remove("LYRICS");
        }

        file_content
            .seek(std::io::SeekFrom::Start(0))
            .context("Failed to seek in FLAC file")?;
        flac_file
            .save_to(&mut file_content, WriteOptions::default())
            .context("Failed to save FLAC file")?;
    }

    Ok(())
}

fn embed_lyrics_mp3(track_path: &str, plain_lyrics: &str, synced_lyrics: &str) -> Result<()> {
    use lofty::file::TaggedFileExt;
    use lofty::id3::v2::Id3v2Tag;
    use lofty::probe::Probe;

    let file_probe = Probe::open(track_path).context("Failed to open MP3 file")?;
    let mut file = file_probe
        .guess_file_type()
        .context("Failed to guess file type")?
        .read()
        .context("Failed to read MP3 file")?;
    let mut primary_tag = file
        .remove(file.primary_tag_type())
        .context("Failed to find ID3v2 tag")?;
    let mut id3v2: Id3v2Tag = primary_tag.into();

    let removed_comments: Vec<_> = id3v2.remove(&FrameId::new("COMM")?).collect();
    for frame in removed_comments {
        if let Frame::Comment(comment) = frame {
            id3v2.insert(Frame::Comment(CommentFrame::new(
                comment.encoding,
                [b'X', b'X', b'X'],
                comment.description,
                comment.content,
            )));
        }
    }

    insert_uslt_frame(&mut id3v2, plain_lyrics).context("Failed to insert USLT frame")?;
    insert_sylt_frame(&mut id3v2, synced_lyrics).context("Failed to insert SYLT frame")?;

    primary_tag = id3v2.into();
    file.insert_tag(primary_tag);
    file.save_to_path(track_path, WriteOptions::default())
        .context("Failed to save MP3 file")?;

    Ok(())
}

fn insert_uslt_frame(id3v2: &mut Id3v2Tag, plain_lyrics: &str) -> Result<()> {
    if !plain_lyrics.is_empty() {
        let uslt_frame = UnsynchronizedTextFrame::new(
            TextEncoding::UTF8,
            [b'X', b'X', b'X'],
            "".to_string(),
            plain_lyrics.to_string(),
        );
        id3v2.insert(Frame::UnsynchronizedText(uslt_frame));
    } else {
        let _ = id3v2.remove(&FrameId::new("USLT")?);
    }

    Ok(())
}

fn insert_sylt_frame(id3v2: &mut Id3v2Tag, synced_lyrics: &str) -> Result<()> {
    if !synced_lyrics.is_empty() {
        let synced_lyrics_vec = synced_lyrics_to_sylt_vec(synced_lyrics)?;

        let sylt_frame = SynchronizedTextFrame::new(
            TextEncoding::UTF8,
            [b'X', b'X', b'X'],
            TimestampFormat::MS,
            SyncTextContentType::Lyrics,
            None,
            synced_lyrics_vec,
        );

        let sylt_frame_byte = sylt_frame.as_bytes(WriteOptions::default())?;
        let sylt_frame_id = FrameId::new("SYLT")?;
        id3v2.insert(Frame::Binary(BinaryFrame::new(
            sylt_frame_id,
            sylt_frame_byte,
        )));
    } else {
        let _ = id3v2.remove(&FrameId::new("SYLT")?);
    }

    Ok(())
}

fn synced_lyrics_to_sylt_vec(synced_lyrics: &str) -> Result<Vec<(u32, String)>> {
    let parsed = parse_lrc(synced_lyrics);

    let converted_lyrics: Vec<(u32, String)> = parsed
        .timed_lines
        .iter()
        .map(|timed_line| (timed_line.timestamp_ms as u32, timed_line.text.clone()))
        .collect();

    Ok(converted_lyrics)
}
