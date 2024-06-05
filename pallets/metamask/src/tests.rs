#![allow(unused_imports)]

use super::*;

use frame_support::assert_ok;

pub use mock::{
    logger::Event as LoggerEvent, new_test_ext, Logger, LoggerCall, Metamask, MetamaskCall,
    RuntimeCall, RuntimeEvent as TestEvent, RuntimeOrigin, System, TestRuntime,
};

use sp_core::{ecdsa::Signature, H160, H256};
use sp_io::crypto::secp256k1_ecdsa_recover_compressed;
use sp_std::str::FromStr;

fn random_addr() -> H160 {
    let mut addr = H160::zero();
    addr.randomize();
    addr
}

#[test]
fn it_recovers_address_from_signature() {
    let address = H160::from_str("0xb48860b17a4e5577ca6104429d806709df430289").expect("address");
    let data_hash =
        H256::from_str("0x7e972c35e3505118083e81f940180bc7433c78d67edc5a2685d39063464eff80")
            .expect("hash");

    let signature = crate::parse_signature(b"0xfa430178296f29edeb026c74ef2f8dd8f66adc97032648a2ccb78ce00aa388467f34b2d32805e0a000ace481384ee5133c5fb7fc180bd945286c7e8c62f8fe761b")
        .expect("signature");

    let recovered = crate::recover_signer_address(signature, data_hash).expect("recover");

    assert_eq!(recovered, address);
}

#[test]
fn logger_emits_events() {
    let sender = random_addr();
    new_test_ext().execute_with(|| {
        // let call = Box::new(RuntimeCall::Logger(LoggerCall::log { blob: vec![] }));
        assert_ok!(Logger::log(RuntimeOrigin::signed(sender), vec![]));
        System::assert_has_event(TestEvent::Logger(LoggerEvent::Log {
            signed_by: sender,
            data: vec![],
        }));
    });
}

#[test]
#[ignore]
fn metamask_mimics_origin() {
    use parity_scale_codec::Encode;

    // let account: <TestRuntime as frame_system::pallet::Config>::AccountId = 42;
    // let data = vec![13, 17, 19];

    let sender = random_addr();
    let data = b"foobar".to_vec();

    new_test_ext().execute_with(|| {
        let nonce = 0; // TODO: value from runtime
        let log_call = Box::new(RuntimeCall::Logger(LoggerCall::log { data: data.clone() }));
        let signature = b"TODO".to_vec(); // TODO: pre-calculate w/ private key

        assert_ok!(Metamask::signed_call(
            RuntimeOrigin::none(),
            sender,
            nonce,
            signature,
            log_call,
        ));

        assert_eq!(
            dbg!(System::events()).into_iter().map(|er| er.event).collect::<Vec<_>>(),
            vec![
                TestEvent::Logger(LoggerEvent::Log { signed_by: sender, data }),
                TestEvent::Metamask(Event::Authorized { signed_by: sender, call_result: Ok(()) })
            ]
        );
    });
}
