use super::*;
use frame_support::assert_ok;
pub use mock::{
    logger::Event as LoggerEvent, new_test_ext, Logger, LoggerCall, Metamask, MetamaskCall,
    RuntimeCall, RuntimeEvent as TestEvent, RuntimeOrigin, System, TestRuntime,
};

#[test]
fn logger_emits_events() {
    new_test_ext().execute_with(|| {
        // let call = Box::new(RuntimeCall::Logger(LoggerCall::log { blob: vec![] }));
        assert_ok!(Logger::log_data(RuntimeOrigin::signed(1), vec![]));
        System::assert_has_event(TestEvent::Logger(LoggerEvent::Logged { by: 1, data: vec![] }));
    });
}

#[test]
fn metamask_mimics_origin() {
    use parity_scale_codec::Encode;
    let account: <TestRuntime as frame_system::pallet::Config>::AccountId = 42;
    let data = vec![13, 17, 19];

    new_test_ext().execute_with(|| {
        let call = Box::new(RuntimeCall::Logger(LoggerCall::log_data { data: data.clone() }));
        assert_ok!(Metamask::signed_call(RuntimeOrigin::none(), call, account.encode()));

        assert_eq!(
            dbg!(System::events()).into_iter().map(|er| er.event).collect::<Vec<_>>(),
            vec![
                TestEvent::Logger(LoggerEvent::Logged { by: account, data }),
                TestEvent::Metamask(Event::Authorized { signed_by: account, call_result: Ok(()) })
            ]
        );
    });
}
