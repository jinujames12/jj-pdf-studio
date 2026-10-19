//! Writing the page pictures that PDF to Images (SPEC 5.11) renders in the
//! window: into a folder, or into one ZIP file.
//!
//! The pictures arrive one at a time so only one page is in memory. PNG and
//! JPEG are already compressed, so ZIP entries are stored, not deflated; that
//! keeps this free of a compression dependency and is as small as it gets.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::assemble::same_file;
use crate::error::{Error, Result};
use crate::split::Written;

// ZIP ----------------------------------------------------------------------

fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };
    !data.iter().fold(!0u32, |crc, &b| {
        TABLE[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8)
    })
}

/// DOS date and time words for "now" (UTC), which is what ZIP stores.
fn dos_now() -> (u16, u16) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rem) = ((secs / 86_400) as i64, (secs % 86_400) as u16);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u16;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u16;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    let year = (year.clamp(1980, 2107) - 1980) as u16;
    let date = (year << 9) | (month << 5) | day;
    let time = ((rem / 3600) << 11) | (((rem % 3600) / 60) << 5) | ((rem % 60) / 2);
    (date, time)
}

struct Entry {
    name: Vec<u8>,
    crc: u32,
    size: u32,
    offset: u32,
}

/// A streaming ZIP writer for stored (uncompressed) entries.
struct ZipWriter<W: Write> {
    out: W,
    entries: Vec<Entry>,
    offset: u64,
    stamp: (u16, u16),
}

impl<W: Write> ZipWriter<W> {
    fn new(out: W) -> Self {
        ZipWriter {
            out,
            entries: Vec::new(),
            offset: 0,
            stamp: dos_now(),
        }
    }

    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        self.out.write_all(bytes)?;
        self.offset += bytes.len() as u64;
        Ok(())
    }

    fn add(&mut self, name: &str, data: &[u8]) -> Result<()> {
        let too_big = Error::BadOption(
            "The ZIP file would be larger than 4 GB. Export fewer pages or use a lower resolution.",
        );
        let size = u32::try_from(data.len())
            .map_err(|_| Error::BadOption("That picture is too large to put in a ZIP file."))?;
        let offset = u32::try_from(self.offset).map_err(|_| too_big)?;
        if self.entries.len() >= 65_534 {
            return Err(Error::BadOption(
                "A ZIP file holds at most 65,000 pictures.",
            ));
        }
        let crc = crc32(data);
        let name = name.as_bytes().to_vec();
        let mut header = Vec::with_capacity(30 + name.len());
        header.extend(0x0403_4B50u32.to_le_bytes());
        header.extend(20u16.to_le_bytes()); // version needed
        header.extend(0x0800u16.to_le_bytes()); // names are UTF-8
        header.extend(0u16.to_le_bytes()); // stored
        header.extend(self.stamp.1.to_le_bytes());
        header.extend(self.stamp.0.to_le_bytes());
        header.extend(crc.to_le_bytes());
        header.extend(size.to_le_bytes());
        header.extend(size.to_le_bytes());
        header.extend((name.len() as u16).to_le_bytes());
        header.extend(0u16.to_le_bytes()); // no extra field
        header.extend(&name);
        self.put(&header)?;
        self.put(data)?;
        self.entries.push(Entry {
            name,
            crc,
            size,
            offset,
        });
        Ok(())
    }

    fn finish(mut self) -> Result<W> {
        let start = self.offset;
        let entries = std::mem::take(&mut self.entries);
        for e in &entries {
            let mut rec = Vec::with_capacity(46 + e.name.len());
            rec.extend(0x0201_4B50u32.to_le_bytes());
            rec.extend(20u16.to_le_bytes()); // made by
            rec.extend(20u16.to_le_bytes()); // version needed
            rec.extend(0x0800u16.to_le_bytes());
            rec.extend(0u16.to_le_bytes());
            rec.extend(self.stamp.1.to_le_bytes());
            rec.extend(self.stamp.0.to_le_bytes());
            rec.extend(e.crc.to_le_bytes());
            rec.extend(e.size.to_le_bytes());
            rec.extend(e.size.to_le_bytes());
            rec.extend((e.name.len() as u16).to_le_bytes());
            rec.extend(0u16.to_le_bytes()); // extra length
            rec.extend(0u16.to_le_bytes()); // comment length
            rec.extend(0u16.to_le_bytes()); // disk number
            rec.extend(0u16.to_le_bytes()); // internal attributes
            rec.extend(0u32.to_le_bytes()); // external attributes
            rec.extend(e.offset.to_le_bytes());
            rec.extend(&e.name);
            self.put(&rec)?;
        }
        let size = u32::try_from(self.offset - start)
            .map_err(|_| Error::BadOption("The ZIP file would be larger than 4 GB."))?;
        let mut end = Vec::with_capacity(22);
        end.extend(0x0605_4B50u32.to_le_bytes());
        end.extend([0u8; 4]); // disk numbers
        end.extend((entries.len() as u16).to_le_bytes());
        end.extend((entries.len() as u16).to_le_bytes());
        end.extend(size.to_le_bytes());
        end.extend(
            u32::try_from(start)
                .map_err(|_| Error::BadOption("The ZIP file would be larger than 4 GB."))?
                .to_le_bytes(),
        );
        end.extend(0u16.to_le_bytes());
        self.put(&end)?;
        self.out.flush()?;
        Ok(self.out)
    }
}

// Export sessions ------------------------------------------------------------

enum Target {
    Folder {
        dir: PathBuf,
        written: Vec<Written>,
    },
    Zip {
        output: PathBuf,
        temp: PathBuf,
        zip: ZipWriter<BufWriter<File>>,
        count: u64,
        bytes: u64,
    },
}

/// One run of PDF to Images: pictures are added one by one, then `finish`
/// (or `cancel`, which removes everything written).
pub struct ImageExport(Target);

/// A name that is a plain file name with a picture extension.
fn check_name(name: &str) -> Result<()> {
    let plain = Path::new(name)
        .file_name()
        .is_some_and(|f| f.to_string_lossy() == name);
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if plain && matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg")) {
        Ok(())
    } else {
        Err(Error::BadOption("That is not a valid picture file name."))
    }
}

/// `dir/name`, or `dir/name (2).ext` and so on if taken: never overwrites.
fn unique_in(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    if !path.exists() {
        return path;
    }
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let ext = p
        .extension()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}).{ext}")))
        .find(|c| !c.exists())
        .expect("an unused name exists")
}

fn looks_like_picture(name: &str, data: &[u8]) -> bool {
    if name.to_ascii_lowercase().ends_with(".png") {
        data.starts_with(&[0x89, b'P', b'N', b'G'])
    } else {
        data.starts_with(&[0xFF, 0xD8])
    }
}

impl ImageExport {
    /// Pictures go into the existing folder `dir`.
    pub fn to_folder(dir: &Path) -> Result<ImageExport> {
        if !dir.is_dir() {
            return Err(Error::BadOption("Choose a folder to save the pictures in."));
        }
        Ok(ImageExport(Target::Folder {
            dir: dir.to_path_buf(),
            written: Vec::new(),
        }))
    }

    /// Pictures go into one ZIP file at `output`, which must not be `source`.
    pub fn to_zip(output: &Path, source: &Path) -> Result<ImageExport> {
        if same_file(source, output) {
            return Err(Error::OutputIsSource(output.to_path_buf()));
        }
        let mut temp = output.as_os_str().to_owned();
        temp.push(".tmp");
        let temp = PathBuf::from(temp);
        let file = File::create(&temp)?;
        Ok(ImageExport(Target::Zip {
            output: output.to_path_buf(),
            temp,
            zip: ZipWriter::new(BufWriter::new(file)),
            count: 0,
            bytes: 0,
        }))
    }

    /// Adds one picture (PNG or JPEG bytes) called `name`.
    pub fn add(&mut self, name: &str, data: &[u8]) -> Result<()> {
        check_name(name)?;
        if !looks_like_picture(name, data) {
            return Err(Error::BadOption("The page picture could not be created."));
        }
        match &mut self.0 {
            Target::Folder { dir, written } => {
                let path = unique_in(dir, name);
                std::fs::write(&path, data)?;
                written.push(Written {
                    path,
                    size: data.len() as u64,
                });
            }
            Target::Zip {
                zip, count, bytes, ..
            } => {
                zip.add(name, data)?;
                *count += 1;
                *bytes += data.len() as u64;
            }
        }
        Ok(())
    }

    /// Completes the export and lists what was written (one ZIP file, or
    /// every picture).
    pub fn finish(self) -> Result<Vec<Written>> {
        match self.0 {
            Target::Folder { written, .. } => Ok(written),
            Target::Zip {
                output, temp, zip, ..
            } => {
                let result = zip
                    .finish()
                    .and_then(|w| w.into_inner().map_err(|e| Error::Io(e.into_error())))
                    .and_then(|file| {
                        drop(file);
                        std::fs::rename(&temp, &output).map_err(Error::from)
                    });
                if let Err(e) = result {
                    let _ = std::fs::remove_file(&temp);
                    return Err(e);
                }
                Ok(vec![Written {
                    size: std::fs::metadata(&output)?.len(),
                    path: output,
                }])
            }
        }
    }

    /// Abandons the export and removes the files it created.
    pub fn cancel(self) {
        match self.0 {
            Target::Folder { written, .. } => {
                for w in written {
                    let _ = std::fs::remove_file(w.path);
                }
            }
            Target::Zip { temp, zip, .. } => {
                drop(zip);
                let _ = std::fs::remove_file(temp);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn names_must_be_plain_picture_files() {
        assert!(check_name("doc_p001.png").is_ok());
        assert!(check_name("doc_p001.JPG").is_ok());
        assert!(check_name("..\\evil.png").is_err());
        assert!(check_name("sub/evil.png").is_err());
        assert!(check_name("evil.exe").is_err());
        assert!(check_name("..").is_err());
    }
}
