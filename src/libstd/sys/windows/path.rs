use crate::ffi::OsStr;
use crate::mem;
use crate::path::Prefix;

pub const MAIN_SEP_STR: &str = "\\";
pub const MAIN_SEP: char = '\\';

fn os_str_as_u8_slice(s: &OsStr) -> &[u8] {
    unsafe { mem::transmute(s) }
}
unsafe fn u8_slice_as_os_str(s: &[u8]) -> &OsStr {
    mem::transmute(s)
}

#[inline]
pub fn is_sep_byte(b: u8) -> bool {
    b == b'/' || b == b'\\'
}

#[inline]
pub fn is_verbatim_sep(b: u8) -> bool {
    b == b'\\'
}

// In most DOS systems, it is not possible to have more than 26 drive letters.
// See <https://en.wikipedia.org/wiki/Drive_letter_assignment#Common_assignments>.
pub fn is_valid_drive_letter(disk: u8) -> bool {
    disk.is_ascii_alphabetic()
}

pub fn parse_prefix(path: &OsStr) -> Option<Prefix<'_>> {
    use Prefix::{DeviceNS, Disk, Verbatim, VerbatimDisk, VerbatimUNC, UNC};
    unsafe {
        // The unsafety here stems from converting between &OsStr and &[u8]
        // and back. This is safe to do because (1) we only look at ASCII
        // contents of the encoding and (2) new &OsStr values are produced
        // only from ASCII-bounded slices of existing &OsStr values.
        let path = os_str_as_u8_slice(path);

        // \\
        if let Some(path) = path.strip_prefix(br"\\") {
            // \\?\
            if let Some(path) = path.strip_prefix(br"?\") {
                // \\?\UNC\server\share
                if let Some(path) = path.strip_prefix(br"UNC\") {
                    let (server, share) = match parse_two_comps(path, is_verbatim_sep) {
                        Some((server, share)) => {
                            (u8_slice_as_os_str(server), u8_slice_as_os_str(share))
                        }
                        None => (u8_slice_as_os_str(path), u8_slice_as_os_str(&[])),
                    };
                    return Some(VerbatimUNC(server, share));
                } else {
                    // \\?\path
                    match path {
                        // \\?\C:\path
                        [c, b':', b'\\', ..] if is_valid_drive_letter(*c) => {
                            return Some(VerbatimDisk(c.to_ascii_uppercase()));
                        }
                        // \\?\cat_pics
                        _ => {
                            let idx = path.iter().position(|&b| b == b'\\').unwrap_or(path.len());
                            let slice = &path[..idx];
                            return Some(Verbatim(u8_slice_as_os_str(slice)));
                        }
                    }
                }
            } else if let Some(path) = path.strip_prefix(b".\\") {
                // \\.\COM42
                let idx = path.iter().position(|&b| b == b'\\').unwrap_or(path.len());
                let slice = &path[..idx];
                return Some(DeviceNS(u8_slice_as_os_str(slice)));
            }
            match parse_two_comps(path, is_sep_byte) {
                Some((server, share)) if !server.is_empty() && !share.is_empty() => {
                    // \\server\share
                    return Some(UNC(u8_slice_as_os_str(server), u8_slice_as_os_str(share)));
                }
                _ => {}
            }
        } else if let [c, b':', ..] = path {
            // C:
            if is_valid_drive_letter(*c) {
                return Some(Disk(c.to_ascii_uppercase()));
            }
        }
        return None;
    }

    fn parse_two_comps(mut path: &[u8], f: fn(u8) -> bool) -> Option<(&[u8], &[u8])> {
        let first = &path[..path.iter().position(|x| f(*x))?];
        path = &path[(first.len() + 1)..];
        let idx = path.iter().position(|x| f(*x));
        let second = &path[..idx.unwrap_or(path.len())];
        Some((first, second))
    }
}

