use core_remote_proto::frame::{
    Chunker, Frame, Kind, Message, Reassembler, MAX_CHUNK, MAX_MESSAGE, MAX_PENDING_STREAMS,
};
use core_remote_proto::{Error, MAX_PLAINTEXT};

#[test]
fn frame_round_trip() {
    let frame = Frame {
        kind: Kind::PtyOut,
        more: true,
        claim: false,
        stream: 0xdead_beef,
        payload: b"abc".to_vec(),
    };
    let bytes = frame.encode().unwrap();
    assert_eq!(&bytes[..6], &[1, 1, 0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(Frame::decode(&bytes).unwrap(), frame);
}

#[test]
fn unknown_kind_and_bad_flags_fail() {
    assert_eq!(Frame::decode(&[9, 0, 0, 0, 0, 1]).unwrap_err(), Error::UnknownKind(9));
    assert_eq!(Frame::decode(&[0, 2, 0, 0, 0, 1]).unwrap_err(), Error::BadFrame);
    assert_eq!(Frame::decode(&[0, 0, 0]).unwrap_err(), Error::BadFrame);
    let mut long = vec![0u8; 6 + MAX_CHUNK + 1];
    long[0] = 1;
    assert_eq!(Frame::decode(&long).unwrap_err(), Error::TooLarge);
}

#[test]
fn json_one_mib_splits_and_joins() {
    let mut json = Vec::with_capacity(MAX_MESSAGE);
    json.extend_from_slice(b"{\"data\":\"");
    while json.len() < MAX_MESSAGE - 2 {
        json.push(b'a' + (json.len() % 26) as u8);
    }
    json.extend_from_slice(b"\"}");
    assert_eq!(json.len(), MAX_MESSAGE);

    let frames: Vec<Frame> = Chunker::new(Kind::Json, 7, &json).unwrap().collect();
    assert_eq!(frames.len(), MAX_MESSAGE.div_ceil(MAX_CHUNK));
    let mut re = Reassembler::new();
    let mut out = None;
    for (i, f) in frames.iter().enumerate() {
        let encoded = f.encode().unwrap();
        assert!(encoded.len() <= MAX_PLAINTEXT);
        assert_eq!(f.more, i + 1 < frames.len());
        let got = re.push(Frame::decode(&encoded).unwrap()).unwrap();
        if i + 1 < frames.len() {
            assert!(got.is_none());
        } else {
            out = got;
        }
    }
    let msg = out.unwrap();
    assert_eq!(msg.kind, Kind::Json);
    assert_eq!(msg.stream, 7);
    assert_eq!(msg.payload, json);
    assert_eq!(re.pending_streams(), 0);
}

#[test]
fn over_one_mib_fails() {
    let big = vec![1u8; MAX_MESSAGE + 1];
    assert_eq!(Chunker::new(Kind::Json, 1, &big).unwrap_err(), Error::TooLarge);

    let mut re = Reassembler::new();
    let chunk = vec![2u8; MAX_CHUNK];
    let mut sent = 0;
    let err = loop {
        let frame = Frame {
            kind: Kind::Json,
            more: true,
            claim: false,
            stream: 1,
            payload: chunk.clone(),
        };
        match re.push(frame) {
            Ok(None) => sent += MAX_CHUNK,
            Ok(Some(_)) => panic!("unexpected message"),
            Err(e) => break e,
        }
    };
    assert_eq!(err, Error::TooLarge);
    assert!(sent <= MAX_MESSAGE);
    assert_eq!(re.pending_streams(), 0);
    let tail = Frame::new(Kind::Json, 1, b"x".to_vec());
    assert_eq!(re.push(tail).unwrap().unwrap().payload, b"x");
}

#[test]
fn streams_and_kinds_are_kept_apart() {
    let mut re = Reassembler::new();
    let part = |kind, stream, more, data: &[u8]| Frame {
        kind,
        more,
        claim: false,
        stream,
        payload: data.to_vec(),
    };
    assert!(re.push(part(Kind::PtyOut, 1, true, b"a")).unwrap().is_none());
    assert!(re.push(part(Kind::PtyOut, 2, true, b"b")).unwrap().is_none());
    assert!(re.push(part(Kind::Json, 1, true, b"c")).unwrap().is_none());
    assert_eq!(
        re.push(part(Kind::PtyOut, 1, false, b"A")).unwrap(),
        Some(Message {
            kind: Kind::PtyOut,
            claim: false,
            stream: 1,
            payload: b"aA".to_vec()
        })
    );
    assert_eq!(re.push(part(Kind::Json, 1, false, b"C")).unwrap().unwrap().payload, b"cC");
    assert_eq!(re.push(part(Kind::PtyOut, 2, false, b"B")).unwrap().unwrap().payload, b"bB");
}

#[test]
fn too_many_open_streams_fail() {
    let mut re = Reassembler::new();
    for s in 0..MAX_PENDING_STREAMS as u32 {
        let f = Frame {
            kind: Kind::PtyOut,
            more: true,
            claim: false,
            stream: s,
            payload: vec![0],
        };
        assert!(re.push(f).unwrap().is_none());
    }
    let f = Frame {
        kind: Kind::PtyOut,
        more: true,
        claim: false,
        stream: 999,
        payload: vec![0],
    };
    assert_eq!(re.push(f).unwrap_err(), Error::TooLarge);
}

#[test]
fn empty_and_exact_chunk_sizes() {
    let frames: Vec<Frame> = Chunker::new(Kind::Ping, 0, &[]).unwrap().collect();
    assert_eq!(frames.len(), 1);
    assert!(!frames[0].more);
    let exact = vec![5u8; MAX_CHUNK];
    let frames: Vec<Frame> = Chunker::new(Kind::PtyIn, 3, &exact).unwrap().collect();
    assert_eq!(frames.len(), 1);
    let plus = vec![5u8; MAX_CHUNK + 1];
    let frames: Vec<Frame> = Chunker::new(Kind::PtyIn, 3, &plus).unwrap().collect();
    assert_eq!(frames.len(), 2);
    assert!(frames[0].more && !frames[1].more);
}

#[test]
fn claim_bit_rides_the_frame_and_only_on_pty_input() {
    let mut frame = Frame::new(Kind::PtyIn, 4, b"k".to_vec());
    frame.claim = true;
    let bytes = frame.encode().unwrap();
    assert_eq!(&bytes[..6], &[2, 2, 0, 0, 0, 4]);
    assert_eq!(Frame::decode(&bytes).unwrap(), frame);
    frame.more = true;
    assert_eq!(frame.encode().unwrap()[1], 3);

    let mut json = Frame::new(Kind::Json, 0, vec![]);
    json.claim = true;
    assert_eq!(json.encode().unwrap_err(), Error::BadFrame);
    assert_eq!(Frame::decode(&[0, 2, 0, 0, 0, 0]).unwrap_err(), Error::BadFrame);
    assert_eq!(Frame::decode(&[1, 2, 0, 0, 0, 0]).unwrap_err(), Error::BadFrame);
    assert_eq!(Frame::decode(&[2, 4, 0, 0, 0, 0]).unwrap_err(), Error::BadFrame);
}

#[test]
fn chunker_claim_reaches_the_reassembled_message() {
    let data = vec![7u8; MAX_CHUNK + 10];
    let frames: Vec<Frame> = Chunker::new(Kind::PtyIn, 3, &data).unwrap().claiming(true).unwrap().collect();
    assert_eq!(frames.len(), 2);
    assert!(frames.iter().all(|f| f.claim));
    let mut re = Reassembler::new();
    let mut last = None;
    for f in frames {
        last = re.push(Frame::decode(&f.encode().unwrap()).unwrap()).unwrap();
    }
    let m = last.unwrap();
    assert!(m.claim);
    assert_eq!(m.payload, data);
    let plain: Vec<Frame> = Chunker::new(Kind::PtyIn, 3, b"x").unwrap().collect();
    assert!(!re.push(plain[0].clone()).unwrap().unwrap().claim);
    assert!(Chunker::new(Kind::Json, 0, b"{}").unwrap().claiming(true).is_err());
}
