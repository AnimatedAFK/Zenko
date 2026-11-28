//! Minimal zip reader (stored + deflate), enough for the whisper.cpp release zip (zipfile.ZipFile in zenko.py).

fn u16le(b: &[u8], i: usize) -> Option<usize> { b.get(i..i + 2).map(|s| u16::from_le_bytes([s[0], s[1]]) as usize) }
fn u32le(b: &[u8], i: usize) -> Option<usize> { b.get(i..i + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize) }

/// Every file entry as (name, data), read through the central directory. Folder entries are skipped.
pub fn entries(b: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let bad = || "File is not a zip file".to_string();
    if b.len() < 22 { return Err(bad()); }
    let lo = b.len().saturating_sub(22 + 65535);
    let eocd = (lo..=b.len() - 22).rev().find(|&i| &b[i..i + 4] == b"PK\x05\x06").ok_or_else(bad)?;
    let count = u16le(b, eocd + 10).ok_or_else(bad)?;
    let mut p = u32le(b, eocd + 16).ok_or_else(bad)?;
    let mut out = Vec::new();
    for _ in 0..count {
        if b.get(p..p + 4) != Some(b"PK\x01\x02") { return Err("Bad magic number for central directory".into()); }
        let method = u16le(b, p + 10).ok_or_else(bad)?;
        let csize = u32le(b, p + 20).ok_or_else(bad)?;
        let usize_ = u32le(b, p + 24).ok_or_else(bad)?;
        let (nl, el, cl) = (u16le(b, p + 28).ok_or_else(bad)?, u16le(b, p + 30).ok_or_else(bad)?, u16le(b, p + 32).ok_or_else(bad)?);
        let off = u32le(b, p + 42).ok_or_else(bad)?;
        let name = String::from_utf8_lossy(b.get(p + 46..p + 46 + nl).ok_or_else(bad)?).replace('\\', "/");
        p += 46 + nl + el + cl;
        if name.ends_with('/') { continue; }
        if b.get(off..off + 4) != Some(b"PK\x03\x04") { return Err("Bad magic number for file header".into()); }
        let start = off + 30 + u16le(b, off + 26).ok_or_else(bad)? + u16le(b, off + 28).ok_or_else(bad)?;
        let raw = b.get(start..start + csize).ok_or_else(bad)?;
        let data = match method {
            0 => raw.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(raw, usize_.max(1) * 2 + 1024)
                .map_err(|e| format!("Error -3 while decompressing data: {e:?}"))?,
            m => return Err(format!("compression type {m}")),
        };
        out.push((name, data));
    }
    Ok(out)
}
