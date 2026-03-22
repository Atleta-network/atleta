#![cfg_attr(not(feature = "std"), no_std)]

use evm_tracing_events::StepEventFilter;
#[cfg(feature = "std")]
use evm_tracing_events::{Event, EvmEvent, GasometerEvent, RuntimeEvent};
#[cfg(feature = "std")]
use parity_scale_codec::Decode;
use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;

#[runtime_interface]
pub trait EvmTracingExt {
    fn evm_event(&mut self, event: Vec<u8>) {
        if let Ok(event) = EvmEvent::decode(&mut &event[..]) {
            Event::Evm(event).emit();
        }
    }

    fn gasometer_event(&mut self, event: Vec<u8>) {
        if let Ok(event) = GasometerEvent::decode(&mut &event[..]) {
            Event::Gasometer(event).emit();
        }
    }

    fn runtime_event(&mut self, event: Vec<u8>) {
        if let Ok(event) = RuntimeEvent::decode(&mut &event[..]) {
            Event::Runtime(event).emit();
        }
    }

    fn call_list_new(&mut self) {
        Event::CallListNew().emit();
    }

    fn step_event_filter(&self) -> StepEventFilter {
        evm_tracing_events::step_event_filter().unwrap_or_default()
    }
}
