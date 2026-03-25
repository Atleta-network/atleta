//! Host function interface for bridging EVM trace events from WASM runtime to native host.
//!
//! Provides [`EvmTracingExt`] runtime interface with host functions that encode/decode
//! EVM execution events across the WASM boundary.

#![cfg_attr(not(feature = "std"), no_std)]

use evm_tracing_events::StepEventFilter;
#[cfg(feature = "std")]
use evm_tracing_events::{Event, EvmEvent, GasometerEvent, RuntimeEvent};
#[cfg(feature = "std")]
use parity_scale_codec::Decode;
use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;

/// Host functions for EVM tracing event bridging.
#[runtime_interface]
pub trait EvmTracingExt {
    /// Receive and re-emit an EVM event on the host side.
    fn evm_event(&mut self, event: Vec<u8>) {
        if let Ok(event) = EvmEvent::decode(&mut &event[..]) {
            Event::Evm(event).emit();
        }
    }

    /// Receive and re-emit a gasometer event on the host side.
    fn gasometer_event(&mut self, event: Vec<u8>) {
        if let Ok(event) = GasometerEvent::decode(&mut &event[..]) {
            Event::Gasometer(event).emit();
        }
    }

    /// Receive and re-emit a runtime event on the host side.
    fn runtime_event(&mut self, event: Vec<u8>) {
        if let Ok(event) = RuntimeEvent::decode(&mut &event[..]) {
            Event::Runtime(event).emit();
        }
    }

    /// Signal the start of a new transaction in block-level tracing.
    fn call_list_new(&mut self) {
        Event::CallListNew().emit();
    }

    /// Query which step events the host-side listener wants to receive.
    fn step_event_filter(&self) -> StepEventFilter {
        evm_tracing_events::step_event_filter().unwrap_or_default()
    }
}
