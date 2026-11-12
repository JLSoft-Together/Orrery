use core_remote_proto::pair::{PairCode, PairInitiator, PairedDesktop, QrPayload};
use core_remote_proto::session::SessionInitiator;
use core_remote_proto::Error;
use wasm_bindgen::prelude::*;

use crate::{fail, key, secs, Identity, Transport};

#[wasm_bindgen]
pub struct Pairing {
    init: Option<PairInitiator>,
    paired: Option<PairedDesktop>,
    msg1: Vec<u8>,
}

impl Pairing {
    fn wrap(started: (PairInitiator, Vec<u8>)) -> Pairing {
        Pairing {
            init: Some(started.0),
            paired: None,
            msg1: started.1,
        }
    }

    fn desktop(&self) -> Result<&PairedDesktop, JsError> {
        self.paired.as_ref().ok_or_else(|| fail(Error::Closed))
    }
}

#[wasm_bindgen]
impl Pairing {
    #[wasm_bindgen(js_name = fromQr)]
    pub fn from_qr(identity: &Identity, qr: &str, device_id: &str, device_name: &str, now: f64) -> Result<Pairing, JsError> {
        let qr = QrPayload::decode(qr).map_err(fail)?;
        PairInitiator::start(identity.keypair(), &qr, device_id, device_name, secs(now)?)
            .map(Pairing::wrap)
            .map_err(fail)
    }

    #[wasm_bindgen(js_name = fromQrOn)]
    pub fn from_qr_on(identity: &Identity, qr: &str, device_id: &str, device_name: &str, platform: &str, now: f64) -> Result<Pairing, JsError> {
        let qr = QrPayload::decode(qr).map_err(fail)?;
        PairInitiator::start_on(identity.keypair(), &qr, device_id, device_name, platform, secs(now)?)
            .map(Pairing::wrap)
            .map_err(fail)
    }

    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(js_name = fromParts)]
    pub fn from_parts(
        identity: &Identity,
        machine: &str,
        mkey: &[u8],
        code: &[u8],
        exp: f64,
        device_id: &str,
        device_name: &str,
        now: f64,
    ) -> Result<Pairing, JsError> {
        let qr = QrPayload {
            machine: machine.to_owned(),
            name: String::new(),
            lan: Vec::new(),
            relay: None,
            ticket: None,
            mkey: key(mkey)?,
            code: PairCode::from_bytes(code).map_err(fail)?,
            exp: secs(exp)?,
        };
        PairInitiator::start(identity.keypair(), &qr, device_id, device_name, secs(now)?)
            .map(Pairing::wrap)
            .map_err(fail)
    }

    #[wasm_bindgen(getter)]
    pub fn msg1(&self) -> Vec<u8> {
        self.msg1.clone()
    }

    pub fn finish(&mut self, msg2: &[u8]) -> Result<Vec<u8>, JsError> {
        let init = self.init.take().ok_or_else(|| fail(Error::Closed))?;
        let (paired, msg3) = init.finish(msg2).map_err(fail)?;
        self.paired = Some(paired);
        Ok(msg3)
    }

    #[wasm_bindgen(getter, js_name = desktopStatic)]
    pub fn desktop_static(&self) -> Result<Vec<u8>, JsError> {
        Ok(self.desktop()?.desktop_static.to_vec())
    }

    #[wasm_bindgen(getter, js_name = machineId)]
    pub fn machine_id(&self) -> Result<String, JsError> {
        Ok(self.desktop()?.machine_id.clone())
    }

    #[wasm_bindgen(js_name = intoTransport)]
    pub fn into_transport(&mut self) -> Result<Transport, JsError> {
        let paired = self.paired.take().ok_or_else(|| fail(Error::Closed))?;
        Ok(Transport::wrap(paired.transport))
    }
}

#[cfg(feature = "test-vectors")]
#[wasm_bindgen]
impl Pairing {
    #[wasm_bindgen(js_name = fromQrFixed)]
    pub fn from_qr_fixed(
        identity: &Identity,
        qr: &str,
        device_id: &str,
        device_name: &str,
        now: f64,
        ephemeral: &[u8],
    ) -> Result<Pairing, JsError> {
        let qr = QrPayload::decode(qr).map_err(fail)?;
        PairInitiator::start_fixed(identity.keypair(), &qr, device_id, device_name, secs(now)?, &key(ephemeral)?)
            .map(Pairing::wrap)
            .map_err(fail)
    }
}

#[wasm_bindgen]
pub struct Session {
    init: Option<SessionInitiator>,
    msg1: Vec<u8>,
}

#[wasm_bindgen]
impl Session {
    pub fn start(identity: &Identity, desktop_static: &[u8], device_id: &str) -> Result<Session, JsError> {
        let (init, msg1) = SessionInitiator::start(identity.keypair(), &key(desktop_static)?, device_id).map_err(fail)?;
        Ok(Session { init: Some(init), msg1 })
    }

    #[wasm_bindgen(getter)]
    pub fn msg1(&self) -> Vec<u8> {
        self.msg1.clone()
    }

    pub fn finish(&mut self, msg2: &[u8]) -> Result<Transport, JsError> {
        let init = self.init.take().ok_or_else(|| fail(Error::Closed))?;
        init.finish(msg2).map(Transport::wrap).map_err(fail)
    }
}

#[cfg(feature = "test-vectors")]
#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(js_name = startFixed)]
    pub fn start_fixed(identity: &Identity, desktop_static: &[u8], device_id: &str, ephemeral: &[u8]) -> Result<Session, JsError> {
        let (init, msg1) =
            SessionInitiator::start_fixed(identity.keypair(), &key(desktop_static)?, device_id, &key(ephemeral)?).map_err(fail)?;
        Ok(Session { init: Some(init), msg1 })
    }
}
