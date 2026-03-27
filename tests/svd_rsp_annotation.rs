//! Integration tests for #5: SVD file on disk + RSP memory commands → same annotation path as the proxy.

use rsgdb::protocol::commands::GdbCommand;
use rsgdb::svd::SvdIndex;
use std::path::Path;

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.svd")
}

#[test]
fn svd_load_from_fixture_file() {
    let idx = SvdIndex::load_from_path(&fixture_path()).expect("load SVD");
    assert!(idx.register_count() >= 2);
    assert_eq!(idx.lookup(0x4002_0000), Some("GPIOA.MODER"));
}

#[test]
fn rsp_read_memory_m_packet_maps_to_register() {
    let idx = SvdIndex::load_from_path(&fixture_path()).expect("load");
    let cmd = GdbCommand::parse(b"m40020000,4").expect("parse m");
    match cmd {
        GdbCommand::ReadMemory { addr, len } => {
            let note = idx
                .annotate_access(addr, len as u64)
                .expect("expected SVD annotation");
            assert!(
                note.contains("GPIOA.MODER"),
                "annotation should name MODER: {note}"
            );
            assert!(
                note.contains("MODE0") && note.contains("MODE1"),
                "annotation should list field names: {note}"
            );
        }
        other => panic!("expected ReadMemory, got {other:?}"),
    }
}

#[test]
fn rsp_write_memory_m_packet_maps_to_register() {
    let idx = SvdIndex::load_from_path(&fixture_path()).expect("load");
    // Maddr,len:hexdata — 4 bytes at MODER
    let cmd = GdbCommand::parse(b"M40020000,4:11223344").expect("parse M");
    match cmd {
        GdbCommand::WriteMemory { addr, data } => {
            assert_eq!(data, vec![0x11, 0x22, 0x33, 0x44]);
            let note = idx
                .annotate_access(addr, data.len() as u64)
                .expect("expected SVD annotation");
            assert!(
                note.contains("GPIOA.MODER"),
                "annotation should name MODER: {note}"
            );
            assert!(
                note.contains("MODE0"),
                "expected field overlap in note: {note}"
            );
        }
        other => panic!("expected WriteMemory, got {other:?}"),
    }
}

#[test]
fn rsp_read_spanning_two_registers_notes_range() {
    let idx = SvdIndex::load_from_path(&fixture_path()).expect("load");
    // From MODER base through first byte of BSRR region (0x18): 0x40020000 + 0x19 bytes
    let cmd = GdbCommand::parse(b"m40020000,19").expect("parse m");
    match cmd {
        GdbCommand::ReadMemory { addr, len } => {
            let note = idx.annotate_access(addr, len as u64).expect("annotate");
            assert!(
                note.contains("GPIOA.MODER") && note.contains("GPIOA.BSRR"),
                "expected span across MODER and BSRR: {note}"
            );
        }
        other => panic!("expected ReadMemory, got {other:?}"),
    }
}
