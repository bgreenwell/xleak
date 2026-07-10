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

    // XLSX/ZIP magic number (PK..)
    if n >= 4 && buffer[..4] == [0x50, 0x4B, 0x03, 0x04] {
        return Ok(FileType::Xlsx);
    }

    // XLS (OLE2/CFB) magic number (D0 CF 11 E0 A1 B1 1A E1)
    if n >= 8 && buffer == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
        return Ok(FileType::Xls);
    }

    // CSV is plain text with no dedicated magic number, so fall back to a
    // heuristic: if the bytes we read contain no NULs and are valid UTF-8,
    // treat the file as CSV-like.
    let prefix = &buffer[..n];
    if prefix.iter().all(|&b| b != 0x00) && std::str::from_utf8(prefix).is_ok() {
        return Ok(FileType::Csv);
    }

    Ok(FileType::Unknown)
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("xleak_{name}_{nonce}"))
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

    #[test]
    fn test_detects_short_csv_without_extension() {
        let path = temp_path("short_csv");
        fs::write(&path, b"a,b\n").unwrap();

        let detected = detect_file_type(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(detected, FileType::Csv);
    }

    #[test]
    fn test_detects_utf8_bom_csv_without_extension() {
        let path = temp_path("bom_csv");
        fs::write(&path, b"\xEF\xBB\xBFna,age\nAndre,30\n").unwrap();

        let detected = detect_file_type(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(detected, FileType::Csv);
    }
}
