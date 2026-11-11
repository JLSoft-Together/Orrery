use core_remote_proto::frame::{Frame, Reassembler};
use core_remote_proto::pair::{PairCode, PairResponder, QrPayload};
use core_remote_proto::relay::{decode, verify_token, RelayKey};
use core_remote_proto::session::{accept, SessionInitiator};
use core_remote_proto::StaticKeypair;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn bytes(&mut self, max: usize) -> Vec<u8> {
        let len = (self.next() as usize) % (max + 1);
        (0..len).map(|_| self.next() as u8).collect()
    }
}

#[test]
fn random_input_never_panics() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let desktop = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let relay = RelayKey::generate().unwrap();
    let mut re = Reassembler::new();
    for i in 0..10_000 {
        let mut input = rng.bytes(if i % 10 == 0 { 70_000 } else { 200 });
        if i % 3 == 0 && input.len() >= 2 {
            input[0] = 0;
            input[1] = 1;
        }
        if let Ok(frame) = Frame::decode(&input) {
            let _ = re.push(frame);
        }
        let _ = decode(&input);
        let _ = accept(&desktop, &input, |_, _| true);
        let _ = PairResponder::start(&desktop, &code, "machine-a", &input);
        let text = String::from_utf8_lossy(&input);
        let _ = QrPayload::decode(&text);
        let _ = verify_token(&relay, "machine-a", &text, 0);
    }
}

#[test]
fn random_msg2_never_panics_initiator() {
    let mut rng = Rng(0x0123_4567_89ab_cdef);
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    for _ in 0..2_000 {
        let (init, _msg1) = SessionInitiator::start(&phone, desktop.public(), "dev-1").unwrap();
        assert!(init.finish(&rng.bytes(300)).is_err());
    }
}
