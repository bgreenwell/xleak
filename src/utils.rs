use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum FileType {
    Xlsx,
    Xls,
    Csv,
    Unknown,
}

pub fn detect_file_type<P: AsRef<Path>>(path: P) -> io::Result<FileType> {
    let path = path.as_ref();

    // Prefer the file extension for text formats: CSV/TSV have no magic bytes,
    // so extension is the most reliable signal.
    if let Some(ext) = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        match ext.as_str() {
            "csv" | "tsv" | "tab" => return Ok(FileType::Csv),
            "xlsx" | "xlsm" | "xlsb" | "ods" => return Ok(FileType::Xlsx),
            "xls" => return Ok(FileType::Xls),
            _ => {}
        }
    }

    let mut file = File::open(path)?;
    let mut buffer = [0u8; 8];

    // Read the first 8 bytes (covers both the ZIP and OLE2 magic-number lengths).
    let n = file.read(&mut buffer)?;

    if n == 0 {
        return Ok(FileType::Unknown);
    }

    // XLSX/ZIP magic number (PK..)
    if n >= 4 && buffer[..4] == [0x50, 0x4B, 0x03, 0x04] {
        return Ok(FileType::Xlsx);
    }

    // XLS (OLE2/CFB) magic number (D0 CF 11 E0 A1 B1 1A E1)
    if n == 8 && buffer == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
        return Ok(FileType::Xls);
    }

    // CSV is plain text with no dedicated magic number, so fall back to a
    // heuristic: NUL-free valid UTF-8 is treated as CSV-like (#56: requiring
    // ASCII rejected BOMs and accented text; requiring 8 bytes rejected tiny
    // files). The window may end mid-character, so a trailing incomplete
    // sequence is fine.
    let sample = &buffer[..n];
    let text_like = match std::str::from_utf8(sample) {
        Ok(s) => !s.contains('\0'),
        Err(e) => {
            e.error_len().is_none()
                && e.valid_up_to() > 0
                && !sample[..e.valid_up_to()].contains(&0)
        }
    };
    if text_like {
        return Ok(FileType::Csv);
    }

    Ok(FileType::Unknown)
}

/// Strip control characters (Unicode `Cc`: C0, DEL, C1) except tab and
/// newline, so untrusted spreadsheet content can't inject terminal escape
/// sequences into non-interactive output (#59). Borrows when already clean.
pub fn sanitize_terminal_text(s: &str) -> std::borrow::Cow<'_, str> {
    let is_bad = |c: char| c.is_control() && c != '\t' && c != '\n';
    if s.chars().any(is_bad) {
        std::borrow::Cow::Owned(s.chars().filter(|&c| !is_bad(c)).collect())
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}

/// Convert a zero-based column index into spreadsheet-style letters
/// (0 -> "A", 25 -> "Z", 26 -> "AA", ...).
pub fn column_index_to_letters(col: usize) -> String {
    let mut result = String::new();
    let mut n = col + 1;
    while n > 0 {
        n -= 1;
        result.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_terminal_text() {
        // Escape sequences are stripped, including C1 controls.
        assert_eq!(sanitize_terminal_text("a\x1b[31mred"), "a[31mred");
        assert_eq!(sanitize_terminal_text("bel\x07l"), "bell");
        assert_eq!(sanitize_terminal_text("c1\u{9b}31mx"), "c131mx");
        // Tab and newline survive; clean strings borrow unchanged.
        assert_eq!(sanitize_terminal_text("a\tb\nc"), "a\tb\nc");
        assert!(matches!(
            sanitize_terminal_text("plain"),
            std::borrow::Cow::Borrowed("plain")
        ));
    }

    #[test]
    fn test_column_index_to_letters() {
        assert_eq!(column_index_to_letters(0), "A");
        assert_eq!(column_index_to_letters(25), "Z");
        assert_eq!(column_index_to_letters(26), "AA");
        assert_eq!(column_index_to_letters(27), "AB");
        assert_eq!(column_index_to_letters(51), "AZ");
        assert_eq!(column_index_to_letters(52), "BA");
        assert_eq!(column_index_to_letters(701), "ZZ");
        assert_eq!(column_index_to_letters(702), "AAA");
    }

    fn detect_tmp(name: &str, bytes: &[u8]) -> FileType {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, bytes).unwrap();
        let result = detect_file_type(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        result
    }

    #[test]
    fn test_detect_utf8_csv_without_extension() {
        // #56: BOMs, accented text, and short files are CSV-like, not Unknown.
        assert_eq!(
            detect_tmp("xleak_sniff_bom", b"\xEF\xBB\xBFa,b\n1,2\n"),
            FileType::Csv
        );
        assert_eq!(
            detect_tmp("xleak_sniff_accent", "h\u{e9}llo,x\n".as_bytes()),
            FileType::Csv
        );
        assert_eq!(detect_tmp("xleak_sniff_short", b"a,b"), FileType::Csv);
        // 8-byte window ending mid-multibyte-character is still text.
        assert_eq!(detect_tmp("xleak_sniff_cut", b"abcdefg\xC3"), FileType::Csv);
    }

    #[test]
    fn test_detect_rejects_binary_and_empty() {
        assert_eq!(detect_tmp("xleak_sniff_bin", &[0xFF; 8]), FileType::Unknown);
        assert_eq!(
            detect_tmp("xleak_sniff_nul", b"a,b\x00c"),
            FileType::Unknown
        );
        assert_eq!(detect_tmp("xleak_sniff_empty", b""), FileType::Unknown);
    }

    #[test]
    fn test_detect_magic_numbers() {
        assert_eq!(
            detect_tmp("xleak_sniff_zip", b"\x50\x4B\x03\x04rest"),
            FileType::Xlsx
        );
        assert_eq!(
            detect_tmp(
                "xleak_sniff_ole",
                &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
            ),
            FileType::Xls
        );
    }
}
