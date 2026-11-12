use core_remote_proto::frame::{self, Chunker, Frame, Kind};
use core_remote_proto::Error;
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::fail;

fn chunks(kind: u8, stream: u32, payload: &[u8], claim: bool) -> Result<Chunker<'_>, JsError> {
    let kind = Kind::try_from(kind).map_err(fail)?;
    Chunker::new(kind, stream, payload)
        .and_then(|c| c.claiming(claim))
        .map_err(fail)
}

#[wasm_bindgen(js_name = encodeFrames)]
pub fn encode_frames(kind: u8, stream: u32, payload: &[u8], claim: bool) -> Result<Array, JsError> {
    let out = Array::new();
    for frame in chunks(kind, stream, payload, claim)? {
        let bytes = frame.encode().map_err(fail)?;
        out.push(&Uint8Array::from(bytes.as_slice()));
    }
    Ok(out)
}

#[wasm_bindgen]
pub struct Message {
    kind: u8,
    stream: u32,
    claim: bool,
    payload: Vec<u8>,
}

#[wasm_bindgen]
impl Message {
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> u8 {
        self.kind
    }

    #[wasm_bindgen(getter)]
    pub fn stream(&self) -> u32 {
        self.stream
    }

    #[wasm_bindgen(getter)]
    pub fn claim(&self) -> bool {
        self.claim
    }

    #[wasm_bindgen(getter)]
    pub fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn text(&self) -> Result<String, JsError> {
        String::from_utf8(self.payload.clone()).map_err(|_| fail(Error::BadFrame))
    }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct Reassembler(frame::Reassembler);

#[wasm_bindgen]
impl Reassembler {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Reassembler {
        Reassembler::default()
    }

    pub fn push(&mut self, frame: &[u8]) -> Result<Option<Message>, JsError> {
        let frame = Frame::decode(frame).map_err(fail)?;
        let done = self.0.push(frame).map_err(fail)?;
        Ok(done.map(|m| Message {
            kind: m.kind as u8,
            stream: m.stream,
            claim: m.claim,
            payload: m.payload,
        }))
    }

    #[wasm_bindgen(getter, js_name = pendingStreams)]
    pub fn pending_streams(&self) -> usize {
        self.0.pending_streams()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[wasm_bindgen]
pub struct Transport(core_remote_proto::Transport);

impl Transport {
    pub(crate) fn wrap(inner: core_remote_proto::Transport) -> Transport {
        Transport(inner)
    }
}

#[wasm_bindgen]
impl Transport {
    pub fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, JsError> {
        self.0.seal(plaintext).map_err(fail)
    }

    pub fn open(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, JsError> {
        self.0.open(ciphertext).map_err(fail)
    }

    #[wasm_bindgen(js_name = sealMessage)]
    pub fn seal_message(&mut self, kind: u8, stream: u32, payload: &[u8], claim: bool) -> Result<Array, JsError> {
        let out = Array::new();
        for frame in chunks(kind, stream, payload, claim)? {
            let sealed = self.0.seal(&frame.encode().map_err(fail)?).map_err(fail)?;
            out.push(&Uint8Array::from(sealed.as_slice()));
        }
        Ok(out)
    }

    #[wasm_bindgen(js_name = openMessage)]
    pub fn open_message(&mut self, ciphertext: &[u8], reassembler: &mut Reassembler) -> Result<Option<Message>, JsError> {
        let plain = self.0.open(ciphertext).map_err(fail)?;
        reassembler.push(&plain)
    }

    #[wasm_bindgen(getter, js_name = isClosed)]
    pub fn is_closed(&self) -> bool {
        self.0.is_closed()
    }

    #[wasm_bindgen(getter, js_name = remoteStatic)]
    pub fn remote_static(&self) -> Option<Vec<u8>> {
        self.0.remote_static().map(|k| k.to_vec())
    }
}
