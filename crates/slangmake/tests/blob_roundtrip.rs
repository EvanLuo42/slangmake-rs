#![cfg(feature = "runtime")]

use slangmake::runtime::{BlobReader, BlobWriter, DependencyInfo};
use slangmake::{Codec, Permutation, Target};

fn make_perm(name: &str, value: &str) -> Permutation {
    let mut p = Permutation::new().unwrap();
    p.add_constant(name, value).unwrap();
    p
}

#[test]
fn empty_blob_round_trips() {
    let writer = BlobWriter::new(Target::Spirv).unwrap();
    let blob = writer.finalize().unwrap();
    assert!(
        !blob.is_empty(),
        "header alone should produce a non-empty blob"
    );

    let reader = BlobReader::open_owned(blob.to_vec()).unwrap();
    assert_eq!(reader.entry_count(), 0);
    assert_eq!(reader.target(), Target::Spirv);
    assert!(reader.at(0).is_none());
}

#[test]
fn round_trip_two_entries_preserves_payload() {
    let perm_a = make_perm("MODE", "FAST");
    let perm_b = make_perm("MODE", "SLOW");

    let code_a: Vec<u8> = (0..32u8).collect();
    let code_b: Vec<u8> = (32..64u8).collect();
    let refl_a: Vec<u8> = vec![0xAA; 16];
    let refl_b: Vec<u8> = vec![0xBB; 24];

    let mut writer = BlobWriter::new(Target::Spirv).unwrap();
    writer.add_entry(&perm_a, &code_a, &refl_a, &[]);
    writer.add_entry(&perm_b, &code_b, &refl_b, &[]);
    writer.set_options_hash(0xDEAD_BEEF_CAFE_BABE);
    let blob = writer.finalize().unwrap();

    let reader = BlobReader::open_owned(blob.to_vec()).unwrap();
    assert_eq!(reader.entry_count(), 2);
    assert_eq!(reader.options_hash(), 0xDEAD_BEEF_CAFE_BABE);

    let entry_a = reader.find(&perm_a).expect("perm_a present");
    assert_eq!(entry_a.code(), code_a.as_slice());
    assert_eq!(entry_a.reflection(), refl_a.as_slice());

    let entry_b = reader.find(&perm_b).expect("perm_b present");
    assert_eq!(entry_b.code(), code_b.as_slice());
    assert_eq!(entry_b.reflection(), refl_b.as_slice());

    let missing = make_perm("MODE", "OTHER");
    assert!(reader.find(&missing).is_none());
}

#[test]
fn dependencies_round_trip() {
    let mut writer = BlobWriter::new(Target::Dxil).unwrap();
    writer
        .set_dependencies(&[
            DependencyInfo {
                path: "shaders/lit.slang".into(),
                content_hash: 1,
            },
            DependencyInfo {
                path: "shaders/common.slang".into(),
                content_hash: 2,
            },
        ])
        .unwrap();
    writer.add_entry(&make_perm("X", "0"), &[1, 2, 3], &[], &[0, 1]);
    let blob = writer.finalize().unwrap();

    let reader = BlobReader::open_owned(blob.to_vec()).unwrap();
    let deps: Vec<_> = reader
        .dependencies()
        .map(|(p, h)| (p.to_string(), h))
        .collect();
    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0], ("shaders/lit.slang".into(), 1));
    assert_eq!(deps[1], ("shaders/common.slang".into(), 2));

    let entry = reader.at(0).unwrap();
    assert_eq!(entry.dep_indices(), &[0u32, 1]);
}

#[test]
fn write_to_file_then_open_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.slangbin");

    let mut writer = BlobWriter::new(Target::Spirv).unwrap();
    writer.add_entry(&make_perm("Q", "1"), b"payload", &[], &[]);
    writer.write_to_file(&path).unwrap();
    assert!(path.exists());

    let reader = BlobReader::open_file(&path).unwrap();
    assert_eq!(reader.entry_count(), 1);
    let entry = reader.at(0).unwrap();
    assert_eq!(entry.code(), b"payload");
}

#[test]
fn open_borrowed_lifetime_matches_buffer() {
    let mut writer = BlobWriter::new(Target::Spirv).unwrap();
    writer.add_entry(&make_perm("K", "v"), &[7, 8, 9], &[], &[]);
    let blob = writer.finalize().unwrap();
    let bytes = blob.to_vec();

    let reader = BlobReader::open_borrowed(&bytes).unwrap();
    assert_eq!(reader.entry_count(), 1);
    assert_eq!(reader.at(0).unwrap().code(), &[7, 8, 9]);
    drop(reader);
    drop(bytes);
}

#[test]
fn open_borrowed_rejects_garbage() {
    // All-zero bytes fail the magic check. Upstream may signal failure either
    // by returning NULL from the constructor (NullHandle) or by returning a
    // handle whose valid() reports false (InvalidBlob); accept either.
    let bytes = vec![0u8; 64];
    let err = BlobReader::open_borrowed(&bytes).unwrap_err();
    assert!(
        matches!(
            err,
            slangmake::Error::InvalidBlob | slangmake::Error::NullHandle { .. }
        ),
        "unexpected error variant: {err:?}"
    );
}

#[test]
fn compression_setting_round_trips() {
    let mut writer = BlobWriter::new(Target::Spirv).unwrap();
    writer.set_compression(Codec::None);
    writer.add_entry(&make_perm("Z", "z"), &vec![0xCC; 256], &[], &[]);
    let blob = writer.finalize().unwrap();

    let reader = BlobReader::open_owned(blob.to_vec()).unwrap();
    assert_eq!(reader.compression(), Codec::None);
    assert_eq!(reader.at(0).unwrap().code(), vec![0xCC; 256].as_slice());
}
