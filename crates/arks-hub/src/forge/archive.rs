//! Source archives for releases (`tarball` / `zipball`): POSIX ustar + gzip, and
//! zip with stored (uncompressed) entries. Arks-owned writers; primary formats:
//! <https://pubs.opengroup.org/onlinepubs/9699919799/utilities/pax.html#tag_20_92_13_06>
//! (ustar) and the PKWARE APPNOTE (local file header / central directory).

use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;

/// `(path, executable, bytes)` entries.
pub(crate) type Entry = (String, bool, Vec<u8>);

fn octal(field: &mut [u8], v: u64) {
    let s = format!("{:0width$o}\0", v, width = field.len() - 1);
    field.copy_from_slice(s.as_bytes());
}

pub(crate) fn tar_gz(entries: &[Entry]) -> Result<Vec<u8>, String> {
    let mut tar = Vec::new();
    for (path, exec, bytes) in entries {
        let mut h = [0u8; 512];
        let (prefix, name) = if path.len() > 100 {
            let cut = path[..path.len().min(155)]
                .rfind('/')
                .ok_or("path too long for ustar")?;
            (&path[..cut], &path[cut + 1..])
        } else {
            ("", path.as_str())
        };
        if name.len() > 100 {
            return Err(format!("path too long for ustar: {path}"));
        }
        h[..name.len()].copy_from_slice(name.as_bytes());
        octal(&mut h[100..108], if *exec { 0o755 } else { 0o644 });
        octal(&mut h[108..116], 0);
        octal(&mut h[116..124], 0);
        octal(&mut h[124..136], bytes.len() as u64);
        octal(&mut h[136..148], 0);
        h[148..156].copy_from_slice(b"        ");
        h[156] = b'0';
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        h[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
        let sum: u32 = h.iter().map(|b| u32::from(*b)).sum();
        let s = format!("{sum:06o}\0 ");
        h[148..156].copy_from_slice(s.as_bytes());
        tar.extend_from_slice(&h);
        tar.extend_from_slice(bytes);
        tar.resize(tar.len().div_ceil(512) * 512, 0);
    }
    tar.resize(tar.len() + 1024, 0);
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&tar).map_err(|e| e.to_string())?;
    gz.finish().map_err(|e| e.to_string())
}

pub(crate) fn zip(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (path, exec, bytes) in entries {
        let crc = crc32fast::hash(bytes);
        let offset = out.len() as u32;
        let size = bytes.len() as u32;
        let name = path.as_bytes();
        let le16 = |v: u16| v.to_le_bytes();
        let le32 = |v: u32| v.to_le_bytes();
        out.extend_from_slice(&le32(0x0403_4b50));
        out.extend_from_slice(&le16(10));
        out.extend_from_slice(&le16(0x0800));
        out.extend_from_slice(&le16(0));
        out.extend_from_slice(&le32(0));
        out.extend_from_slice(&le32(crc));
        out.extend_from_slice(&le32(size));
        out.extend_from_slice(&le32(size));
        out.extend_from_slice(&le16(name.len() as u16));
        out.extend_from_slice(&le16(0));
        out.extend_from_slice(name);
        out.extend_from_slice(bytes);
        central.extend_from_slice(&le32(0x0201_4b50));
        central.extend_from_slice(&le16(0x031e));
        central.extend_from_slice(&le16(10));
        central.extend_from_slice(&le16(0x0800));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le32(0));
        central.extend_from_slice(&le32(crc));
        central.extend_from_slice(&le32(size));
        central.extend_from_slice(&le32(size));
        central.extend_from_slice(&le16(name.len() as u16));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        let mode: u32 = if *exec { 0o100_755 } else { 0o100_644 };
        central.extend_from_slice(&le32(mode << 16));
        central.extend_from_slice(&le32(offset));
        central.extend_from_slice(name);
    }
    let cd_offset = out.len() as u32;
    let n = entries.len() as u16;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::io::Read;

    #[test]
    fn tar_header_checksum_and_zip_signature() {
        let e = vec![("p/a.txt".to_string(), false, b"hi\n".to_vec())];
        let gz = tar_gz(&e).unwrap();
        let mut tar = Vec::new();
        GzDecoder::new(&gz[..]).read_to_end(&mut tar).unwrap();
        assert_eq!(&tar[..7], b"p/a.txt");
        assert_eq!(&tar[257..263], b"ustar\0");
        let stored: u32 =
            u32::from_str_radix(std::str::from_utf8(&tar[148..154]).unwrap(), 8).unwrap();
        let mut h = tar[..512].to_vec();
        h[148..156].copy_from_slice(b"        ");
        assert_eq!(stored, h.iter().map(|b| u32::from(*b)).sum::<u32>());
        assert_eq!(&tar[512..515], b"hi\n");
        let z = zip(&e);
        assert_eq!(&z[..4], b"PK\x03\x04");
        assert_eq!(&z[z.len() - 22..z.len() - 18], b"PK\x05\x06");
    }
}
