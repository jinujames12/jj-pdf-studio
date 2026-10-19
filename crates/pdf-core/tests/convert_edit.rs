//! Properties (metadata) and the PDF to Images writer.

mod common;

use std::path::{Path, PathBuf};

use common::{make_sized_pdf, Tmp};
use lopdf::{dictionary, Document, Object, Stream};
use pdf_core::{properties_file, read_properties, Error, ImageExport, PropertiesEdit, Protection};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{name}.pdf"))
}

/// A document with Info entries and an XMP stream, like an office export.
fn described() -> Document {
    let mut doc = make_sized_pdf("P", &[(612, 792), (842, 1191)]);
    doc.get_dictionary_mut(doc.get_pages()[&2])
        .unwrap()
        .set("Rotate", 90);
    let info = doc.add_object(dictionary! {
        "Title" => Object::string_literal("Old title"),
        "Author" => Object::string_literal("Old author"),
        "Producer" => Object::string_literal("Some Writer 3"),
        "CreationDate" => Object::string_literal("D:20240517093000+05'30'"),
    });
    doc.trailer.set("Info", info);
    let xmp = doc.add_object(Stream::new(
        dictionary! { "Type" => "Metadata", "Subtype" => "XML" },
        b"<x:xmpmeta>Old author</x:xmpmeta>".to_vec(),
    ));
    let catalog = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    doc.get_dictionary_mut(catalog)
        .unwrap()
        .set("Metadata", xmp);
    doc
}

#[test]
fn properties_are_read_for_display() {
    let t = Tmp::new("props-read");
    let src = t.path("in.pdf");
    described().save(&src).unwrap();
    let p = read_properties(&src).unwrap();
    assert_eq!(
        (p.title.as_str(), p.author.as_str()),
        ("Old title", "Old author")
    );
    assert_eq!(p.producer, "Some Writer 3");
    assert_eq!(p.created, "2024-05-17 09:30");
    assert_eq!(p.modified, "");
    assert_eq!(p.pdf_version, "1.5");
    assert!(p.has_xmp);
    assert_eq!(p.protection.protection, Protection::None);
    assert_eq!(p.size, std::fs::metadata(&src).unwrap().len());
    assert_eq!(p.pages.len(), 2);
    assert_eq!(
        (p.pages[1].width, p.pages[1].height, p.pages[1].rotate),
        (842.0, 1191.0, 90)
    );
}

#[test]
fn editing_changes_info_keeps_the_rest_and_drops_stale_xmp() {
    let t = Tmp::new("props-edit");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    described().save(&src).unwrap();
    let edit = PropertiesEdit {
        title: "  Quarterly report ".into(),
        author: "Zoë Ångström 日本".into(),
        subject: "Finance".into(),
        keywords: "q3, finance".into(),
        creator: String::new(),
        remove_all: false,
    };
    properties_file(&src, &out, &edit).unwrap();
    let p = read_properties(&out).unwrap();
    assert_eq!(p.title, "Quarterly report");
    assert_eq!(p.author, "Zoë Ångström 日本");
    assert_eq!(
        (p.subject.as_str(), p.keywords.as_str()),
        ("Finance", "q3, finance")
    );
    assert_eq!(p.creator, "");
    assert_eq!(
        p.producer, "Some Writer 3",
        "fields that are not editable stay"
    );
    assert_eq!(p.created, "2024-05-17 09:30");
    assert!(!p.has_xmp, "the old XMP would contradict the new title");
    // The pages are untouched.
    let doc = Document::load(&out).unwrap();
    assert_eq!(doc.get_pages().len(), 2);
    assert_eq!(read_properties(&out).unwrap().pages[1].rotate, 90);
    // The original is not modified.
    assert_eq!(read_properties(&src).unwrap().title, "Old title");
}

#[test]
fn a_file_without_info_gets_one_and_clearing_everything_removes_it() {
    let t = Tmp::new("props-new");
    let (src, mid, out) = (t.path("in.pdf"), t.path("mid.pdf"), t.path("out.pdf"));
    make_sized_pdf("P", &[(200, 300)]).save(&src).unwrap();
    assert_eq!(read_properties(&src).unwrap().title, "");
    let edit = PropertiesEdit {
        title: "Hello".into(),
        ..Default::default()
    };
    properties_file(&src, &mid, &edit).unwrap();
    assert_eq!(read_properties(&mid).unwrap().title, "Hello");
    properties_file(&mid, &out, &PropertiesEdit::default()).unwrap();
    assert!(Document::load(&out).unwrap().trailer.get(b"Info").is_err());
}

#[test]
fn remove_all_strips_info_and_xmp_everywhere() {
    let t = Tmp::new("props-strip");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    let mut doc = described();
    // A page-level XMP stream and a direct Info are covered too.
    let page = doc.get_pages()[&1];
    let xmp = doc.add_object(Stream::new(
        dictionary! {},
        b"<xmp>secret author</xmp>".to_vec(),
    ));
    doc.get_dictionary_mut(page).unwrap().set("Metadata", xmp);
    doc.save(&src).unwrap();
    let edit = PropertiesEdit {
        title: "ignored".into(),
        remove_all: true,
        ..Default::default()
    };
    properties_file(&src, &out, &edit).unwrap();
    let p = read_properties(&out).unwrap();
    assert_eq!(
        (&p.title, &p.author, &p.producer, &p.created),
        (
            &String::new(),
            &String::new(),
            &String::new(),
            &String::new()
        )
    );
    assert!(!p.has_xmp);
    let bytes = std::fs::read(&out).unwrap();
    for needle in ["Old author", "Old title", "secret author", "Some Writer"] {
        assert!(
            !bytes.windows(needle.len()).any(|w| w == needle.as_bytes()),
            "{needle} still in the file"
        );
    }
    assert_eq!(Document::load(&out).unwrap().get_pages().len(), 2);
}

#[test]
fn restricted_files_open_and_are_saved_without_the_restrictions_flagged_in_the_report() {
    let t = Tmp::new("props-restricted");
    let out = t.path("out.pdf");
    let src = fixture("rc4_128_restricted");
    let before = read_properties(&src).unwrap();
    assert_eq!(before.protection.protection, Protection::RestrictionsOnly);
    properties_file(
        &src,
        &out,
        &PropertiesEdit {
            title: "T".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let after = read_properties(&out).unwrap();
    assert_eq!(after.title, "T");
    assert_eq!(after.protection.protection, Protection::None);
    assert_eq!(after.pages.len(), before.pages.len());
}

#[test]
fn properties_never_overwrite_the_source() {
    let t = Tmp::new("props-same");
    let src = t.path("in.pdf");
    described().save(&src).unwrap();
    assert!(matches!(
        properties_file(&src, &src, &PropertiesEdit::default()),
        Err(Error::OutputIsSource(_))
    ));
}

// Image export -------------------------------------------------------------------

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 13, 10, 26, 10, 1, 2, 3];
const JPG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 9, 9, 9, 0xFF, 0xD9];

#[test]
fn a_folder_export_writes_files_and_never_overwrites() {
    let t = Tmp::new("exp-folder");
    std::fs::write(t.path("doc_p001.png"), b"mine").unwrap();
    let mut e = ImageExport::to_folder(&t.0).unwrap();
    e.add("doc_p001.png", PNG).unwrap();
    e.add("doc_p002.jpg", JPG).unwrap();
    let written = e.finish().unwrap();
    assert_eq!(written.len(), 2);
    assert_eq!(std::fs::read(t.path("doc_p001.png")).unwrap(), b"mine");
    assert_eq!(std::fs::read(t.path("doc_p001 (2).png")).unwrap(), PNG);
    assert_eq!(written[0].size, PNG.len() as u64);
    assert!(ImageExport::to_folder(&t.path("missing")).is_err());
}

#[test]
fn cancelling_removes_what_was_written() {
    let t = Tmp::new("exp-cancel");
    let mut e = ImageExport::to_folder(&t.0).unwrap();
    e.add("a.png", PNG).unwrap();
    e.cancel();
    assert!(!t.path("a.png").exists());

    let src = t.path("in.pdf");
    let mut z = ImageExport::to_zip(&t.path("pages.zip"), &src).unwrap();
    z.add("a.png", PNG).unwrap();
    z.cancel();
    assert!(!t.path("pages.zip").exists() && !t.path("pages.zip.tmp").exists());
}

#[test]
fn only_plain_picture_names_with_real_picture_data_are_accepted() {
    let t = Tmp::new("exp-names");
    let mut e = ImageExport::to_folder(&t.0).unwrap();
    for bad in ["..\\x.png", "../x.png", "sub/x.png", "x.exe", "x"] {
        assert!(e.add(bad, PNG).is_err(), "{bad}");
    }
    assert!(e.add("x.png", JPG).is_err(), "JPEG bytes under a .png name");
    assert!(e.add("x.jpg", b"not a picture").is_err());
    assert!(e.finish().unwrap().is_empty());
}

/// Reads the entries back with a small independent ZIP reader.
fn read_zip(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]) as usize;
    let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
    let end = bytes.len() - 22;
    assert_eq!(u32_at(end), 0x0605_4B50, "end of central directory");
    let (count, mut at) = (u16_at(end + 10), u32_at(end + 16));
    let mut out = Vec::new();
    for _ in 0..count {
        assert_eq!(u32_at(at), 0x0201_4B50);
        let (crc, size) = (u32_at(at + 16), u32_at(at + 24));
        let (name_len, extra, comment) = (u16_at(at + 28), u16_at(at + 30), u16_at(at + 32));
        let local = u32_at(at + 42);
        let name = String::from_utf8(bytes[at + 46..at + 46 + name_len].to_vec()).unwrap();
        assert_eq!(u32_at(local), 0x0403_4B50);
        let data_at = local + 30 + u16_at(local + 26) + u16_at(local + 28);
        let data = bytes[data_at..data_at + size].to_vec();
        let mut c = !0u32;
        for &b in &data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        assert_eq!(!c as usize, crc, "crc of {name}");
        out.push((name, data));
        at += 46 + name_len + extra + comment;
    }
    out
}

#[test]
fn a_zip_export_holds_every_picture_intact() {
    let t = Tmp::new("exp-zip");
    let (src, out) = (t.path("in.pdf"), t.path("pages.zip"));
    let mut e = ImageExport::to_zip(&out, &src).unwrap();
    e.add("doc_p001.png", PNG).unwrap();
    e.add("doc_p002.jpg", JPG).unwrap();
    let big: Vec<u8> = [&PNG[..8], &vec![7u8; 300_000]].concat();
    e.add("doc_p003.png", &big).unwrap();
    let written = e.finish().unwrap();
    assert_eq!(written.len(), 1);
    assert_eq!(written[0].path, out);
    assert!(!t.path("pages.zip.tmp").exists());
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(written[0].size, bytes.len() as u64);
    let entries = read_zip(&bytes);
    let names: Vec<_> = entries.iter().map(|e| e.0.as_str()).collect();
    assert_eq!(names, ["doc_p001.png", "doc_p002.jpg", "doc_p003.png"]);
    assert_eq!(entries[0].1, PNG);
    assert_eq!(entries[1].1, JPG);
    assert_eq!(entries[2].1, big);
}

#[test]
fn the_zip_cannot_replace_the_source_file() {
    let t = Tmp::new("exp-zip-src");
    let src = t.path("in.pdf");
    std::fs::write(&src, b"x").unwrap();
    assert!(matches!(
        ImageExport::to_zip(&src, &src),
        Err(Error::OutputIsSource(_))
    ));
}
