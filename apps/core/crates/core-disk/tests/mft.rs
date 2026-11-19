use core_disk::index::flag;
use core_disk::mft::record::{apply_fixup, data_runs, parse, Parsed, RecordError, Run, NS_DOS, NS_WIN32};
use core_disk::mft::sample::{self, Data, Spec, EXPECTED_BYTES, RECORD};
use core_disk::mft::tree::{Table, TreeError};
use core_disk::mft::wire::{self, check_hello, hello, parse_helper_args, write_frame, Frame, FrameError, KIND_ERROR, KIND_INDEX, KIND_PROGRESS, PIPE_PREFIX};
use core_disk::mft::{accept, consume, drive_letter, produce, MemorySource, MftError};
use core_disk::Index;

fn table() -> Table {
    let mut img = sample::image();
    let mut t = Table::new(64);
    t.add_chunk(&mut img, 0, RECORD);
    t
}

fn index() -> Index {
    table().build("Q:\\").unwrap()
}

fn node(ix: &Index, path: &str) -> core_disk::Node {
    ix.nodes[ix.find(path).unwrap_or_else(|| panic!("missing {path}")) as usize]
}

#[test]
fn fixup_restores_sector_tails_and_rejects_torn_records() {
    let spec = Spec::file(5, "a.txt").alloc(4096, 10);
    let raw = sample::record(&spec, 40);
    assert_eq!(&raw[510..512], &[0x07, 0x00]);
    let mut ok = raw.clone();
    apply_fixup(&mut ok).unwrap();
    assert_eq!(&ok[510..512], &[0, 0]);
    let mut torn = raw.clone();
    torn[1022] = 0x99;
    assert_eq!(apply_fixup(&mut torn), Err(RecordError::Fixup));
    let mut magic = raw.clone();
    magic[0] = b'B';
    assert_eq!(apply_fixup(&mut magic), Err(RecordError::Magic));
    let mut short = raw[..512].to_vec();
    assert_eq!(apply_fixup(&mut short), Err(RecordError::Fixup));
}

#[test]
fn parses_standard_info_names_and_data() {
    let list = sample::records();
    let spec = &list.iter().find(|(n, _)| *n == 19).unwrap().1;
    let mut rec = sample::record(spec, 19);
    let mut p = Parsed::default();
    parse(&mut rec, &mut p).unwrap();
    assert!(p.in_use && !p.dir);
    assert_eq!(p.names.len(), 2);
    assert_eq!((p.names[0].parent, p.names[0].ns, p.name(&p.names[0])), (17, NS_WIN32, "Ảnh dài tên.jpg"));
    assert_eq!((p.names[1].ns, p.name(&p.names[1])), (NS_DOS, "ANHDAI~1.JPG"));
    assert_eq!(p.data, Some((4096, 4000)));
    assert_eq!(p.attrs, Some(0x20));
    assert!(p.mtime > 1_600_000_000);

    let mut tiny = Spec::file(5, "t.txt");
    tiny.data = Some(Data::Resident(123));
    let mut rec = sample::record(&tiny, 41);
    parse(&mut rec, &mut p).unwrap();
    assert_eq!(p.data, Some((0, 123)));

    let mut packed = Spec::file(5, "p.log");
    packed.data = Some(Data::Runs { alloc: 65536, len: 99, compressed: Some(8192), vcn: 0, runs: vec![0x11, 0x10, 0x05, 0x00] });
    let mut rec = sample::record(&packed, 42);
    parse(&mut rec, &mut p).unwrap();
    assert_eq!(p.data, Some((8192, 99)));
    let (a, b) = p.runs.unwrap();
    assert_eq!(data_runs(&rec[a..b]), Some(vec![Run { lcn: 5, clusters: 16 }]));

    let mut later = Spec::file(5, "x");
    later.data = Some(Data::Runs { alloc: 1, len: 1, compressed: None, vcn: 16, runs: vec![0x11, 0x01, 0x01, 0x00] });
    let mut rec = sample::record(&later, 43);
    parse(&mut rec, &mut p).unwrap();
    assert_eq!(p.data, None);
}

#[test]
fn deleted_records_parse_as_unused() {
    let mut spec = Spec::file(5, "gone").alloc(4096, 1);
    spec.in_use = false;
    let mut rec = sample::record(&spec, 50);
    let mut p = Parsed::default();
    parse(&mut rec, &mut p).unwrap();
    assert!(!p.in_use);
    assert!(p.names.is_empty());
}

#[test]
fn broken_attribute_lengths_are_errors() {
    let mut rec = sample::record(&Spec::file(5, "a"), 51);
    let first = u16::from_le_bytes([rec[0x14], rec[0x15]]) as usize;
    rec[first + 4..first + 8].copy_from_slice(&0x7000u32.to_le_bytes());
    let mut p = Parsed::default();
    assert_eq!(parse(&mut rec, &mut p), Err(RecordError::Bounds));
}

#[test]
fn data_runs_handle_negative_offsets_and_reject_garbage() {
    let runs = [0x21, 0x10, 0x00, 0x01, 0x11, 0x08, 0xF0, 0x00];
    assert_eq!(data_runs(&runs), Some(vec![Run { lcn: 256, clusters: 16 }, Run { lcn: 240, clusters: 8 }]));
    assert_eq!(data_runs(&[0x11, 0x08, 0x80, 0x00]), None);
    assert_eq!(data_runs(&[0x01, 0x08, 0x00]), None);
    assert_eq!(data_runs(&[0x21, 0x10]), None);
    assert_eq!(data_runs(&[0x00]), Some(vec![]));
}

#[test]
fn tree_builds_paths_sizes_and_flags() {
    let ix = index();
    assert_eq!(ix.root_paths(), vec!["Q:\\".to_string()]);
    assert_eq!(ix.stats.bytes, EXPECTED_BYTES);
    assert_eq!(node(&ix, "Q:\\Users\\jin\\a.bin").size, 8192);
    assert_eq!(node(&ix, "Q:\\Users\\jin\\a.bin").len, 5000);
    assert_eq!(node(&ix, "Q:\\Users\\jin\\Ảnh dài tên.jpg").size, 4096);
    assert!(ix.find("Q:\\Users\\jin\\ANHDAI~1.JPG").is_none());
    assert!(ix.find("Q:\\Users\\jin\\ghost.txt").is_none());
    assert!(ix.find("Q:\\Users\\jin\\orphan.txt").is_none());
    assert!(ix.find("Q:\\$MFT").is_none());
    assert!(ix.find("Q:\\$Extend").is_none());
    let l1 = node(&ix, "Q:\\Users\\jin\\link1.dat");
    let l2 = node(&ix, "Q:\\Users\\link2.dat");
    assert_eq!(l1.size + l2.size, 65536);
    assert_eq!((l1.flags | l2.flags) & flag::HARDLINK, flag::HARDLINK);
    assert_eq!(l1.flags & l2.flags & flag::HARDLINK, 0);
    let cloud = node(&ix, "Q:\\Users\\cloud.docx");
    assert_eq!((cloud.size, cloud.flags & flag::CLOUD), (0, flag::CLOUD));
    let link = node(&ix, "Q:\\Link");
    assert_eq!(link.flags & (flag::LINK | flag::DIR), flag::LINK | flag::DIR);
    assert_eq!(link.count, 0);
    assert_eq!(node(&ix, "Q:\\big.iso").size, 1 << 20);
    assert_eq!(node(&ix, "Q:\\big.iso").len, 1_000_000);
    assert_eq!(node(&ix, "Q:\\packed.log").size, 16384);
    let case = node(&ix, "Q:\\Case");
    assert_eq!((case.size, case.len), (0, 10));
    assert_ne!(case.flags & flag::HIDDEN, 0);
    assert_ne!(case.flags & flag::SYSTEM, 0);
    assert_eq!(node(&ix, "Q:\\tiny.txt").len, 100);
    assert_eq!(node(&ix, "Q:\\Users").size, 8192 + 4096 + 65536);
    assert!(node(&ix, "Q:\\").mtime > 0);
}

#[test]
fn tree_needs_the_root_record() {
    let list: Vec<_> = sample::records().into_iter().filter(|(n, _)| *n != 5).collect();
    let mut img = vec![0u8; 32 * RECORD];
    for (n, s) in &list {
        img[*n as usize * RECORD..(*n as usize + 1) * RECORD].copy_from_slice(&sample::record(s, *n));
    }
    let mut t = Table::new(32);
    t.add_chunk(&mut img, 0, RECORD);
    assert_eq!(t.build("Q:\\").unwrap_err(), TreeError::NoRoot);
}

#[test]
fn tree_survives_directory_cycles() {
    let mut list = sample::records();
    list.retain(|(n, _)| *n != 16 && *n != 17);
    list.push((16, Spec::dir(17, "A")));
    list.push((17, Spec::dir(16, "B")));
    list.push((31, Spec::dir(5, "C")));
    let mut img = vec![0u8; 32 * RECORD];
    for (n, s) in &list {
        img[*n as usize * RECORD..(*n as usize + 1) * RECORD].copy_from_slice(&sample::record(s, *n));
    }
    let mut t = Table::new(32);
    t.add_chunk(&mut img, 0, RECORD);
    let ix = t.build("Q:\\").unwrap();
    assert!(ix.find("Q:\\C").is_some());
    assert!(ix.find("Q:\\A").is_none());
}

#[test]
fn tree_counts_torn_records() {
    let mut img = sample::image();
    img[18 * RECORD + 1022] ^= 0xFF;
    let mut t = Table::new(64);
    t.add_chunk(&mut img, 0, RECORD);
    assert_eq!(t.bad, 1);
    let ix = t.build("Q:\\").unwrap();
    assert!(ix.find("Q:\\Users\\jin\\a.bin").is_none());
}

#[test]
fn tree_respects_record_limit() {
    let mut img = sample::image();
    let mut t = Table::with_limit(20);
    t.add_chunk(&mut img, 0, RECORD);
    let ix = t.build("Q:\\").unwrap();
    assert!(ix.find("Q:\\big.iso").is_none());
    assert!(ix.find("Q:\\Users\\jin\\a.bin").is_some());
}

#[test]
fn helper_arguments_are_strict() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let pipe = format!("{PIPE_PREFIX}{}", "0123456789abcdef".repeat(2));
    let tok = "ab".repeat(32);
    let ok = parse_helper_args(&s(&["C", &pipe, &tok])).unwrap();
    assert_eq!((ok.drive, ok.token[0]), ('C', 0xab));
    for bad in [
        s(&["c", &pipe, &tok]),
        s(&["CD", &pipe, &tok]),
        s(&["1", &pipe, &tok]),
        s(&["Đ", &pipe, &tok]),
        s(&["C", &pipe]),
        s(&["C", &pipe, &tok, "x"]),
        s(&["C", &format!("{pipe}0"), &tok]),
        s(&["C", &pipe.to_uppercase(), &tok]),
        s(&["C", r"\\server\pipe\orrery-mft-0123456789abcdef0123456789abcdef", &tok]),
        s(&["C", &pipe, &tok.to_uppercase()]),
        s(&["C", &pipe, &tok[..62]]),
        s(&["C", &pipe, &format!("{}zz", &tok[..62])]),
    ] {
        assert!(parse_helper_args(&bad).is_none(), "{bad:?}");
    }
}

#[test]
fn hello_needs_the_exact_token() {
    let t = [9u8; 32];
    assert!(check_hello(&hello(&t), &t));
    let mut other = t;
    other[31] = 8;
    assert!(!check_hello(&hello(&other), &t));
    assert!(!check_hello(&hello(&t)[..39], &t));
    let mut magic = hello(&t);
    magic[0] = b'X';
    assert!(!check_hello(&magic, &t));
}

#[test]
fn drive_letters_normalize() {
    assert_eq!(drive_letter("c"), Some('C'));
    assert_eq!(drive_letter("D:"), Some('D'));
    assert_eq!(drive_letter("e:\\"), Some('E'));
    assert_eq!(drive_letter("CD"), None);
    assert_eq!(drive_letter("\\\\srv\\share"), None);
    assert_eq!(drive_letter(""), None);
}

#[test]
fn produce_and_consume_round_trip() {
    let mut src = MemorySource::new(sample::image()).unwrap();
    let mut wire_bytes = Vec::new();
    produce(&mut src, 'Q', &mut wire_bytes).unwrap();
    let mut seen = Vec::new();
    let ix = consume(&mut &wire_bytes[..], 'Q', |r, t| seen.push((r, t))).unwrap();
    assert_eq!(ix.stats.bytes, EXPECTED_BYTES);
    assert_eq!(seen.last(), Some(&(31, 31)));
    assert_eq!(consume(&mut &wire_bytes[..], 'R', |_, _| {}).unwrap_err(), MftError::Helper("index root mismatch".into()));
}

#[test]
fn consume_reports_helper_errors_and_bad_frames() {
    let mut b = Vec::new();
    write_frame(&mut b, KIND_PROGRESS, &wire::progress_payload(1, 2)).unwrap();
    write_frame(&mut b, KIND_ERROR, b"access denied").unwrap();
    assert_eq!(consume(&mut &b[..], 'Q', |_, _| {}).unwrap_err(), MftError::Helper("access denied".into()));
    assert_eq!(consume(&mut &b[..21], 'Q', |_, _| {}).unwrap_err(), MftError::Helper("helper exited".into()));
    let mut bad = [9u8, 0, 0, 0, 0];
    assert_eq!(wire::read_frame(&mut &bad[..]), Err(FrameError::Bad("kind")));
    bad[0] = KIND_PROGRESS;
    bad[1] = 17;
    assert_eq!(wire::read_frame(&mut &bad[..]), Err(FrameError::Bad("length")));
    let mut huge = vec![KIND_INDEX];
    huge.extend_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(wire::read_frame(&mut &huge[..]), Err(FrameError::Bad("length")));
    let mut garbage = Vec::new();
    write_frame(&mut garbage, KIND_INDEX, b"not zstd").unwrap();
    assert!(matches!(consume(&mut &garbage[..], 'Q', |_, _| {}), Err(MftError::Helper(e)) if e.starts_with("index:")));
    let mut ok = Vec::new();
    write_frame(&mut ok, KIND_PROGRESS, &wire::progress_payload(3, 4)).unwrap();
    assert_eq!(wire::read_frame(&mut &ok[..]), Ok(Frame::Progress { records: 3, total: 4 }));
}

#[test]
fn accept_rejects_indexes_with_other_roots() {
    let ix = table().build("C:\\").unwrap();
    let body = zstd::stream::encode_all(&core_disk::store::encode(&ix)[..], 1).unwrap();
    assert!(accept(&body, 'C').is_ok());
    assert!(accept(&body, 'D').is_err());
}

#[test]
fn fixture_file_is_current() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("mft-sample.bin");
    let img = sample::image();
    if std::env::var_os("UPDATE_MFT_FIXTURE").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &img).unwrap();
    }
    assert_eq!(std::fs::read(&path).expect("run with UPDATE_MFT_FIXTURE=1 to create it"), img);
}

#[test]
fn memory_source_rejects_non_mft_bytes() {
    assert!(MemorySource::new(vec![0u8; 4096]).is_err());
    let mut img = sample::image();
    img[0x1C..0x20].copy_from_slice(&1000u32.to_le_bytes());
    assert!(MemorySource::new(img).is_err());
}

#[test]
fn records_pointing_past_the_table_are_skipped() {
    let mut img = sample::image();
    let mut rogue = Spec::file(4_000_000_000, "far.bin").alloc(4096, 1);
    rogue.base = 3_000_000_000;
    img.extend_from_slice(&sample::record(&rogue, 31));
    let mut t = Table::new(32);
    t.add_chunk(&mut img, 0, RECORD);
    assert_eq!(t.skipped, 1);
    assert_eq!(t.build("Q:\\").unwrap().stats.bytes, EXPECTED_BYTES);
}
