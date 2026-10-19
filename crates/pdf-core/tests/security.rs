//! Unlock and Protect: round trips over RC4 40/128-bit, AES-128 and AES-256
//! files. The fixtures in `tests/fixtures` were written by pypdf (an
//! independent implementation), so decrypting them proves interoperability and
//! not just that lopdf agrees with itself.

mod common;

use std::path::{Path, PathBuf};

use common::*;
use lopdf::{dictionary, Document, Object, Permissions, Stream};
use pdf_core::{
    inspect_protection, load, load_with, protect_file, unlock_file, unlock_for_session,
    viewable_bytes, Error, ProtectOptions, Protection,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{name}.pdf"))
}

/// Text drawn on page `n` of the fixtures, read back after decryption.
fn page_text(doc: &Document, n: u32) -> String {
    let id = doc.get_pages()[&n];
    String::from_utf8_lossy(&doc.get_page_content(id)).into_owned()
}

fn assert_fixture_content(doc: &Document) {
    assert_eq!(doc.get_pages().len(), 2);
    assert!(page_text(doc, 1).contains("FIXTURE-1"));
    assert!(page_text(doc, 2).contains("FIXTURE-2"));
}

fn assert_plain_file(path: &Path) {
    let doc = Document::load(path).unwrap();
    assert!(
        !doc.is_encrypted() && !doc.was_encrypted(),
        "still encrypted"
    );
    assert!(!std::fs::read(path)
        .unwrap()
        .windows(8)
        .any(|w| w == b"/Encrypt"));
}

fn options(open: &str) -> ProtectOptions {
    ProtectOptions {
        open_password: open.into(),
        owner_password: None,
        allow_printing: true,
        allow_copying: true,
        allow_editing: true,
    }
}

const OPEN_PASSWORD_FIXTURES: [(&str, &str); 4] = [
    ("rc4_40", "RC4 40-bit"),
    ("rc4_128", "RC4 128-bit"),
    ("aes128", "AES-128"),
    ("aes256", "AES-256"),
];

#[test]
fn open_password_files_are_recognised_and_need_the_password() {
    for (name, algorithm) in OPEN_PASSWORD_FIXTURES {
        let path = fixture(name);
        let info = inspect_protection(&path).unwrap();
        assert_eq!(info.protection, Protection::OpenPassword, "{name}");
        assert_eq!(info.algorithm.as_deref(), Some(algorithm), "{name}");

        assert!(
            matches!(load_with(&path, None), Err(Error::Encrypted)),
            "{name}"
        );
        assert!(
            matches!(load_with(&path, Some("nope")), Err(Error::WrongPassword)),
            "{name}"
        );
        assert_fixture_content(&load_with(&path, Some("user")).unwrap());
    }
}

#[test]
fn owner_password_opens_aes_256_but_is_refused_clearly_for_older_files() {
    // lopdf would "succeed" on the older files and return scrambled content.
    for (name, _) in &OPEN_PASSWORD_FIXTURES[..3] {
        assert!(
            matches!(
                load_with(&fixture(name), Some("owner")),
                Err(Error::OwnerPasswordOnly)
            ),
            "{name}"
        );
    }
    assert_fixture_content(&load_with(&fixture("aes256"), Some("owner")).unwrap());
}

#[test]
fn unlock_round_trip_for_every_algorithm() {
    let t = Tmp::new("unlock-all");
    for (name, algorithm) in OPEN_PASSWORD_FIXTURES {
        let out = t.path(&format!("{name}-unlocked.pdf"));
        assert!(matches!(
            unlock_file(&fixture(name), &out, None),
            Err(Error::Encrypted)
        ));
        assert!(!out.exists(), "{name}: nothing may be written on failure");
        assert!(matches!(
            unlock_file(&fixture(name), &out, Some("wrong")),
            Err(Error::WrongPassword)
        ));

        let report = unlock_file(&fixture(name), &out, Some("user")).unwrap();
        assert_eq!(report.protection.protection, Protection::OpenPassword);
        assert_eq!(report.protection.algorithm.as_deref(), Some(algorithm));
        assert_eq!(report.size, std::fs::metadata(&out).unwrap().len());
        assert_plain_file(&out);
        assert_fixture_content(&load(&out).unwrap());
    }
}

#[test]
fn restrictions_only_files_open_and_unlock_without_a_password() {
    let t = Tmp::new("unlock-restricted");
    for (name, algorithm) in [
        ("rc4_128_restricted", "RC4 128-bit"),
        ("aes256_restricted", "AES-256"),
    ] {
        let path = fixture(name);
        let info = inspect_protection(&path).unwrap();
        assert_eq!(info.protection, Protection::RestrictionsOnly, "{name}");
        assert_eq!(info.algorithm.as_deref(), Some(algorithm), "{name}");
        assert_eq!(info.blocked, ["copying text and images"], "{name}");
        // Every other tool just opens it.
        assert_fixture_content(&load(&path).unwrap());

        let out = t.path(&format!("{name}-unlocked.pdf"));
        let report = unlock_file(&path, &out, None).unwrap();
        assert_eq!(report.protection.protection, Protection::RestrictionsOnly);
        assert_plain_file(&out);
        assert_fixture_content(&load(&out).unwrap());
    }
}

#[test]
fn unlocking_a_file_with_nothing_to_remove_is_reported() {
    let t = Tmp::new("unlock-plain");
    let plain = fixture("plain");
    assert_eq!(
        inspect_protection(&plain).unwrap().protection,
        Protection::None
    );
    assert!(matches!(
        unlock_file(&plain, &t.path("out.pdf"), None),
        Err(Error::NotProtected)
    ));
}

#[test]
fn unlock_never_overwrites_the_source() {
    let t = Tmp::new("unlock-source");
    let src = t.path("in.pdf");
    std::fs::copy(fixture("aes256_restricted"), &src).unwrap();
    let before = std::fs::read(&src).unwrap();
    assert!(matches!(
        unlock_file(&src, &src, None),
        Err(Error::OutputIsSource(_))
    ));
    assert_eq!(std::fs::read(&src).unwrap(), before);
}

// --- Protect -------------------------------------------------------------------------

#[test]
fn protect_writes_aes_256_that_needs_the_open_password() {
    let t = Tmp::new("protect");
    let out = t.path("protected.pdf");
    let size = protect_file(&fixture("plain"), &out, &options("open sesame")).unwrap();
    assert_eq!(size, std::fs::metadata(&out).unwrap().len());

    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"%PDF-2.0"));
    assert!(
        !bytes.windows(9).any(|w| w == b"FIXTURE-1"),
        "page text must not be readable"
    );
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/AESV3") && text.contains("/R 6") && text.contains("/V 5"));
    assert!(text.contains("/ID"));

    let info = inspect_protection(&out).unwrap();
    assert_eq!(info.protection, Protection::OpenPassword);
    assert_eq!(info.algorithm.as_deref(), Some("AES-256"));
    assert!(matches!(load_with(&out, None), Err(Error::Encrypted)));
    assert!(matches!(
        load_with(&out, Some("open")),
        Err(Error::WrongPassword)
    ));
    assert_fixture_content(&load_with(&out, Some("open sesame")).unwrap());
    // With no restrictions the open password doubles as the owner password.
    assert_fixture_content(&load_with(&out, Some("open sesame")).unwrap());
}

#[test]
fn protect_then_unlock_round_trips() {
    let t = Tmp::new("protect-unlock");
    let protected = t.path("p.pdf");
    protect_file(&fixture("plain"), &protected, &options("pw")).unwrap();
    let unlocked = t.path("u.pdf");
    unlock_file(&protected, &unlocked, Some("pw")).unwrap();
    assert_plain_file(&unlocked);
    assert_fixture_content(&load(&unlocked).unwrap());
}

#[test]
fn restrictions_are_written_and_read_back() {
    let t = Tmp::new("protect-restrict");
    let out = t.path("p.pdf");
    let mut opts = options("open");
    opts.owner_password = Some("owner".into());
    opts.allow_copying = false;
    opts.allow_editing = false;
    protect_file(&fixture("plain"), &out, &opts).unwrap();

    for password in ["open", "owner"] {
        assert_fixture_content(&load_with(&out, Some(password)).unwrap());
    }
    let doc = Document::load_with_password(&out, "open").unwrap();
    let p = doc.encryption_state.as_ref().unwrap().permissions();
    assert!(p.contains(Permissions::PRINTABLE));
    assert!(!p.contains(Permissions::COPYABLE));
    assert!(!p.contains(Permissions::MODIFIABLE));

    // Unlocking with the open password drops the restrictions as well.
    let unlocked = t.path("u.pdf");
    unlock_file(&out, &unlocked, Some("open")).unwrap();
    assert_plain_file(&unlocked);
}

#[test]
fn protect_checks_its_passwords() {
    let t = Tmp::new("protect-checks");
    let out = t.path("p.pdf");
    assert!(matches!(
        protect_file(&fixture("plain"), &out, &options("")),
        Err(Error::NoPassword)
    ));

    // Restricting without an owner password, or with the same one, is refused.
    let mut opts = options("open");
    opts.allow_printing = false;
    assert!(matches!(
        protect_file(&fixture("plain"), &out, &opts),
        Err(Error::BadOwnerPassword)
    ));
    opts.owner_password = Some("open".into());
    assert!(matches!(
        protect_file(&fixture("plain"), &out, &opts),
        Err(Error::BadOwnerPassword)
    ));
    assert!(!out.exists());

    let src = t.path("in.pdf");
    std::fs::copy(fixture("plain"), &src).unwrap();
    assert!(matches!(
        protect_file(&src, &src, &options("pw")),
        Err(Error::OutputIsSource(_))
    ));
}

#[test]
fn non_ascii_passwords_work() {
    let t = Tmp::new("protect-unicode");
    let out = t.path("p.pdf");
    protect_file(&fixture("plain"), &out, &options("pässwörd-密码")).unwrap();
    assert_fixture_content(&load_with(&out, Some("pässwörd-密码")).unwrap());
}

#[test]
fn a_file_without_an_id_still_protects() {
    // make_pdf builds documents with no trailer /ID.
    let t = Tmp::new("protect-noid");
    let src = t.path("in.pdf");
    write_pdf(&src, "NOID", 3);
    let out = t.path("p.pdf");
    protect_file(&src, &out, &options("pw")).unwrap();
    let doc = load_with(&out, Some("pw")).unwrap();
    assert_eq!(doc.get_pages().len(), 3);
}

// --- Passwords remembered for the session ----------------------------------------------

#[test]
fn a_remembered_password_opens_the_file_in_every_tool() {
    let t = Tmp::new("session");
    let src = t.path("locked.pdf");
    std::fs::copy(fixture("aes256"), &src).unwrap();

    assert!(matches!(load(&src), Err(Error::Encrypted)));
    assert!(matches!(
        unlock_for_session(&src, "wrong"),
        Err(Error::WrongPassword)
    ));
    assert!(matches!(load(&src), Err(Error::Encrypted)));

    unlock_for_session(&src, "user").unwrap();
    assert_fixture_content(&load(&src).unwrap());
    assert_eq!(pdf_core::inspect(&src).unwrap().pages.len(), 2);

    // Pdf.js gets a decrypted in-memory copy; the file on disk is untouched.
    let bytes = viewable_bytes(&src).unwrap();
    assert_fixture_content(&Document::load_mem(&bytes).unwrap());
    assert!(inspect_protection(&src).unwrap().protection == Protection::OpenPassword);

    // Page operations work on it and the result is not protected.
    let out = t.path("out.pdf");
    pdf_core::save_pages(&src, &out, &[pdf_core::PageOp::Page { page: 2, rotate: 0 }]).unwrap();
    assert_plain_file(&out);
    assert!(page_text(&load(&out).unwrap(), 1).contains("FIXTURE-2"));

    // The unlock tool uses the remembered password when none is passed.
    let unlocked = t.path("unlocked.pdf");
    unlock_file(&src, &unlocked, None).unwrap();
    assert_fixture_content(&load(&unlocked).unwrap());
}

#[test]
fn a_stale_remembered_password_asks_again() {
    let t = Tmp::new("session-stale");
    let src = t.path("locked.pdf");
    std::fs::copy(fixture("rc4_128"), &src).unwrap();
    unlock_for_session(&src, "user").unwrap();
    // The file is replaced by one with a different password.
    std::fs::copy(fixture("aes128"), &src).unwrap();
    protect_file(
        &fixture("plain"),
        &src.with_extension("new"),
        &options("other"),
    )
    .unwrap();
    std::fs::rename(src.with_extension("new"), &src).unwrap();
    assert!(matches!(load(&src), Err(Error::Encrypted)));
    assert!(matches!(load(&src), Err(Error::Encrypted)));
}

// --- Why copying can still fail ------------------------------------------------------------

fn scanned_and_text_pdf() -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let image = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 2, "Height" => 2,
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
        },
        vec![0, 255, 255, 0],
    ));
    // A font with no Unicode map that is embedded and has no encoding.
    let font_file = doc.add_object(Stream::new(dictionary! {}, vec![0u8; 8]));
    let descriptor = doc.add_object(dictionary! {
        "Type" => "FontDescriptor", "FontName" => "AAAAAA+Odd", "FontFile2" => font_file,
    });
    let odd_font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "TrueType", "BaseFont" => "AAAAAA+Odd",
        "FontDescriptor" => descriptor,
    });
    let good_font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let resources = dictionary! {
        "XObject" => dictionary! { "Im0" => image },
        "Font" => dictionary! { "F1" => odd_font, "F2" => good_font },
    };
    let mut kids = Vec::new();
    for content in [
        // Page 1: only a full-page picture.
        "q 200 0 0 300 0 0 cm /Im0 Do Q",
        // Page 2: a full-page picture with real text on top (searchable scan).
        "q 200 0 0 300 0 0 cm /Im0 Do Q BT /F2 12 Tf 10 10 Td (hello) Tj ET",
        // Page 3: a small picture only.
        "q 20 0 0 20 5 5 cm /Im0 Do Q",
        // Page 4: a full-page picture reached through nested q / cm.
        "q 1 0 0 1 0 0 cm q 100 0 0 150 0 0 cm 2 0 0 2 0 0 cm /Im0 Do Q Q",
    ] {
        let c = doc.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
        kids.push(Object::Reference(doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => c,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 300.into()],
            "Resources" => resources.clone(),
        })));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => kids.len() as i64, "Kids" => kids,
        }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc
}

#[test]
fn unlock_explains_why_copying_may_still_fail() {
    let t = Tmp::new("hints");
    let src = t.path("in.pdf");
    scanned_and_text_pdf().save(&src).unwrap();
    let locked = t.path("locked.pdf");
    protect_file(&src, &locked, &options("pw")).unwrap();

    let report = unlock_file(&locked, &t.path("out.pdf"), Some("pw")).unwrap();
    assert_eq!(report.hints.scanned_pages, vec![1, 4]);
    assert_eq!(report.hints.fonts_without_unicode, 1);

    // Ordinary text documents raise no hint.
    let clean = unlock_file(&fixture("aes256_restricted"), &t.path("c.pdf"), None).unwrap();
    assert!(clean.hints.scanned_pages.is_empty());
    assert_eq!(clean.hints.fonts_without_unicode, 0);
}
