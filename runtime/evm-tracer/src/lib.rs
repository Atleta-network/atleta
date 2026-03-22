#![cfg_attr(not(feature = "std"), no_std)]

use evm_tracing_events::StepEventFilter;

pub struct EvmTracer {
    step_event_filter: StepEventFilter,
}

impl EvmTracer {
    pub fn new() -> Self {
        let step_event_filter = evm_tracing_ext::evm_tracing_ext::step_event_filter();
        Self { step_event_filter }
    }

    #[cfg(feature = "evm-tracing")]
    pub fn trace<R, F: FnOnce() -> R>(self, f: F) -> R {
        use sp_std::{cell::RefCell, rc::Rc};

        let wrapped = Rc::new(RefCell::new(self));

        let mut gasometer = ListenerProxy(Rc::clone(&wrapped));
        let mut runtime = ListenerProxy(Rc::clone(&wrapped));
        let mut evm = ListenerProxy(Rc::clone(&wrapped));

        let f = || evm_runtime::tracing::using(&mut runtime, f);
        let f = || evm_gasometer::tracing::using(&mut gasometer, f);
        let f = || evm::tracing::using(&mut evm, f);
        f()
    }

    #[cfg(not(feature = "evm-tracing"))]
    pub fn trace<R, F: FnOnce() -> R>(self, f: F) -> R {
        let _ = self.step_event_filter;
        f()
    }
}

#[cfg(feature = "evm-tracing")]
use evm_tracing_events::{
    evm::{CreateScheme, Transfer},
    gasometer::Snapshot,
    runtime::{Capture, Memory, Stack, Trap},
    Context, EvmEvent, GasometerEvent, RuntimeEvent,
};
#[cfg(feature = "evm-tracing")]
use parity_scale_codec::Encode;
#[cfg(feature = "evm-tracing")]
use sp_std::{cell::RefCell, rc::Rc};

#[cfg(feature = "evm-tracing")]
struct ListenerProxy<T>(Rc<RefCell<T>>);

#[cfg(feature = "evm-tracing")]
impl<T: evm::tracing::EventListener> evm::tracing::EventListener for ListenerProxy<T> {
    fn event(&mut self, event: evm::tracing::Event<'_>) {
        self.0.borrow_mut().event(event);
    }
}

#[cfg(feature = "evm-tracing")]
impl<T: evm_runtime::tracing::EventListener> evm_runtime::tracing::EventListener
    for ListenerProxy<T>
{
    fn event(&mut self, event: evm_runtime::tracing::Event<'_>) {
        self.0.borrow_mut().event(event);
    }
}

#[cfg(feature = "evm-tracing")]
impl<T: evm_gasometer::tracing::EventListener> evm_gasometer::tracing::EventListener
    for ListenerProxy<T>
{
    fn event(&mut self, event: evm_gasometer::tracing::Event) {
        self.0.borrow_mut().event(event);
    }
}

#[cfg(feature = "evm-tracing")]
impl evm::tracing::EventListener for EvmTracer {
    fn event(&mut self, event: evm::tracing::Event<'_>) {
        let event = convert_evm_event(event);
        let encoded = event.encode();
        evm_tracing_ext::evm_tracing_ext::evm_event(encoded);
    }
}

#[cfg(feature = "evm-tracing")]
impl evm_runtime::tracing::EventListener for EvmTracer {
    fn event(&mut self, event: evm_runtime::tracing::Event<'_>) {
        let event = convert_runtime_event(event, self.step_event_filter);
        let encoded = event.encode();
        evm_tracing_ext::evm_tracing_ext::runtime_event(encoded);
    }
}

#[cfg(feature = "evm-tracing")]
impl evm_gasometer::tracing::EventListener for EvmTracer {
    fn event(&mut self, event: evm_gasometer::tracing::Event) {
        let event = convert_gasometer_event(event);
        let encoded = event.encode();
        evm_tracing_ext::evm_tracing_ext::gasometer_event(encoded);
    }
}

#[cfg(feature = "evm-tracing")]
fn convert_context(context: &evm::Context) -> Context {
    Context {
        address: context.address,
        caller: context.caller,
        apparent_value: context.apparent_value,
    }
}

#[cfg(feature = "evm-tracing")]
fn convert_transfer(transfer: &evm_runtime::Transfer) -> Transfer {
    Transfer { source: transfer.source, target: transfer.target, value: transfer.value }
}

#[cfg(feature = "evm-tracing")]
fn convert_create_scheme(scheme: evm_runtime::CreateScheme) -> CreateScheme {
    match scheme {
        evm_runtime::CreateScheme::Legacy { caller } => CreateScheme::Legacy { caller },
        evm_runtime::CreateScheme::Create2 { caller, code_hash, salt } => {
            CreateScheme::Create2 { caller, code_hash, salt }
        },
        evm_runtime::CreateScheme::Fixed(address) => CreateScheme::Fixed(address),
    }
}

#[cfg(feature = "evm-tracing")]
fn convert_evm_event(event: evm::tracing::Event<'_>) -> EvmEvent {
    match event {
        evm::tracing::Event::Call {
            code_address,
            transfer,
            input,
            target_gas,
            is_static,
            context,
        } => EvmEvent::Call {
            code_address,
            transfer: transfer.as_ref().map(convert_transfer),
            input: input.to_vec(),
            target_gas,
            is_static,
            context: convert_context(context),
        },
        evm::tracing::Event::Create { caller, address, scheme, value, init_code, target_gas } => {
            EvmEvent::Create {
                caller,
                address,
                scheme: convert_create_scheme(scheme),
                value,
                init_code: init_code.to_vec(),
                target_gas,
            }
        },
        evm::tracing::Event::Suicide { address, target, balance } => {
            EvmEvent::Suicide { address, target, balance }
        },
        evm::tracing::Event::Exit { reason, return_value } => {
            EvmEvent::Exit { reason: reason.clone(), return_value: return_value.to_vec() }
        },
        evm::tracing::Event::TransactCall { caller, address, value, data, gas_limit } => {
            EvmEvent::TransactCall { caller, address, value, data: data.to_vec(), gas_limit }
        },
        evm::tracing::Event::TransactCreate { caller, value, init_code, gas_limit, address } => {
            EvmEvent::TransactCreate {
                caller,
                value,
                init_code: init_code.to_vec(),
                gas_limit,
                address,
            }
        },
        evm::tracing::Event::TransactCreate2 {
            caller,
            value,
            init_code,
            salt,
            gas_limit,
            address,
        } => EvmEvent::TransactCreate2 {
            caller,
            value,
            init_code: init_code.to_vec(),
            salt,
            gas_limit,
            address,
        },
        evm::tracing::Event::PrecompileSubcall {
            code_address,
            transfer,
            input,
            target_gas,
            is_static,
            context,
        } => EvmEvent::PrecompileSubcall {
            code_address,
            transfer: transfer.as_ref().map(convert_transfer),
            input: input.to_vec(),
            target_gas,
            is_static,
            context: convert_context(context),
        },
    }
}

#[cfg(feature = "evm-tracing")]
fn convert_snapshot(snapshot: Option<evm_gasometer::Snapshot>) -> Snapshot {
    match snapshot {
        Some(snapshot) => Snapshot {
            gas_limit: snapshot.gas_limit,
            memory_gas: snapshot.memory_gas,
            used_gas: snapshot.used_gas,
            refunded_gas: snapshot.refunded_gas,
        },
        None => Snapshot::default(),
    }
}

#[cfg(feature = "evm-tracing")]
fn convert_gasometer_event(event: evm_gasometer::tracing::Event) -> GasometerEvent {
    match event {
        evm_gasometer::tracing::Event::RecordCost { cost, snapshot } => {
            GasometerEvent::RecordCost { cost, snapshot: convert_snapshot(snapshot) }
        },
        evm_gasometer::tracing::Event::RecordRefund { refund, snapshot } => {
            GasometerEvent::RecordRefund { refund, snapshot: convert_snapshot(snapshot) }
        },
        evm_gasometer::tracing::Event::RecordStipend { stipend, snapshot } => {
            GasometerEvent::RecordStipend { stipend, snapshot: convert_snapshot(snapshot) }
        },
        evm_gasometer::tracing::Event::RecordDynamicCost {
            gas_cost,
            memory_gas,
            gas_refund,
            snapshot,
        } => GasometerEvent::RecordDynamicCost {
            gas_cost,
            memory_gas,
            gas_refund,
            snapshot: convert_snapshot(snapshot),
        },
        evm_gasometer::tracing::Event::RecordTransaction { cost, snapshot } => {
            GasometerEvent::RecordTransaction { cost, snapshot: convert_snapshot(snapshot) }
        },
    }
}

#[cfg(feature = "evm-tracing")]
fn convert_runtime_context(context: &evm_runtime::Context) -> Context {
    Context {
        address: context.address,
        caller: context.caller,
        apparent_value: context.apparent_value,
    }
}

#[cfg(feature = "evm-tracing")]
fn convert_stack(stack: &evm::Stack) -> Stack {
    Stack { data: stack.data().clone(), limit: stack.limit() as u64 }
}

#[cfg(feature = "evm-tracing")]
fn convert_memory(memory: &evm::Memory) -> Memory {
    Memory {
        data: memory.data().clone(),
        effective_len: memory.effective_len(),
        limit: memory.limit() as u64,
    }
}

#[cfg(feature = "evm-tracing")]
fn convert_runtime_event(
    event: evm_runtime::tracing::Event<'_>,
    filter: StepEventFilter,
) -> RuntimeEvent {
    match event {
        evm_runtime::tracing::Event::Step { context, opcode, position, stack, memory } => {
            RuntimeEvent::Step {
                context: convert_runtime_context(context),
                opcode: opcode_to_string(opcode),
                position: match position {
                    Ok(position) => Ok(*position as u64),
                    Err(reason) => Err(reason.clone()),
                },
                stack: if filter.enable_stack { Some(convert_stack(stack)) } else { None },
                memory: if filter.enable_memory { Some(convert_memory(memory)) } else { None },
            }
        },
        evm_runtime::tracing::Event::StepResult { result, return_value } => {
            RuntimeEvent::StepResult {
                result: match result {
                    Ok(()) => Ok(()),
                    Err(capture) => match capture {
                        evm::Capture::Exit(reason) => Err(Capture::Exit(reason.clone())),
                        evm::Capture::Trap(trap) => Err(Capture::Trap(opcode_to_string(*trap))),
                    },
                },
                return_value: return_value.to_vec(),
            }
        },
        evm_runtime::tracing::Event::SLoad { address, index, value } => {
            RuntimeEvent::SLoad { address, index, value }
        },
        evm_runtime::tracing::Event::SStore { address, index, value } => {
            RuntimeEvent::SStore { address, index, value }
        },
    }
}

#[cfg(feature = "evm-tracing")]
fn opcode_to_string(opcode: evm::Opcode) -> Trap {
    let name = match opcode {
        evm::Opcode(0) => "Stop",
        evm::Opcode(1) => "Add",
        evm::Opcode(2) => "Mul",
        evm::Opcode(3) => "Sub",
        evm::Opcode(4) => "Div",
        evm::Opcode(5) => "SDiv",
        evm::Opcode(6) => "Mod",
        evm::Opcode(7) => "SMod",
        evm::Opcode(8) => "AddMod",
        evm::Opcode(9) => "MulMod",
        evm::Opcode(10) => "Exp",
        evm::Opcode(11) => "SignExtend",
        evm::Opcode(16) => "Lt",
        evm::Opcode(17) => "Gt",
        evm::Opcode(18) => "Slt",
        evm::Opcode(19) => "Sgt",
        evm::Opcode(20) => "Eq",
        evm::Opcode(21) => "IsZero",
        evm::Opcode(22) => "And",
        evm::Opcode(23) => "Or",
        evm::Opcode(24) => "Xor",
        evm::Opcode(25) => "Not",
        evm::Opcode(26) => "Byte",
        evm::Opcode(27) => "Shl",
        evm::Opcode(28) => "Shr",
        evm::Opcode(29) => "Sar",
        evm::Opcode(32) => "Keccak256",
        evm::Opcode(48) => "Address",
        evm::Opcode(49) => "Balance",
        evm::Opcode(50) => "Origin",
        evm::Opcode(51) => "Caller",
        evm::Opcode(52) => "CallValue",
        evm::Opcode(53) => "CallDataLoad",
        evm::Opcode(54) => "CallDataSize",
        evm::Opcode(55) => "CallDataCopy",
        evm::Opcode(56) => "CodeSize",
        evm::Opcode(57) => "CodeCopy",
        evm::Opcode(58) => "GasPrice",
        evm::Opcode(59) => "ExtCodeSize",
        evm::Opcode(60) => "ExtCodeCopy",
        evm::Opcode(61) => "ReturnDataSize",
        evm::Opcode(62) => "ReturnDataCopy",
        evm::Opcode(63) => "ExtCodeHash",
        evm::Opcode(64) => "BlockHash",
        evm::Opcode(65) => "Coinbase",
        evm::Opcode(66) => "Timestamp",
        evm::Opcode(67) => "Number",
        evm::Opcode(68) => "Difficulty",
        evm::Opcode(69) => "GasLimit",
        evm::Opcode(70) => "ChainId",
        evm::Opcode(80) => "Pop",
        evm::Opcode(81) => "MLoad",
        evm::Opcode(82) => "MStore",
        evm::Opcode(83) => "MStore8",
        evm::Opcode(84) => "SLoad",
        evm::Opcode(85) => "SStore",
        evm::Opcode(86) => "Jump",
        evm::Opcode(87) => "JumpI",
        evm::Opcode(88) => "GetPc",
        evm::Opcode(89) => "MSize",
        evm::Opcode(90) => "Gas",
        evm::Opcode(91) => "JumpDest",
        evm::Opcode(92) => "TLoad",
        evm::Opcode(93) => "TStore",
        evm::Opcode(94) => "MCopy",
        evm::Opcode(96) => "Push1",
        evm::Opcode(97) => "Push2",
        evm::Opcode(98) => "Push3",
        evm::Opcode(99) => "Push4",
        evm::Opcode(100) => "Push5",
        evm::Opcode(101) => "Push6",
        evm::Opcode(102) => "Push7",
        evm::Opcode(103) => "Push8",
        evm::Opcode(104) => "Push9",
        evm::Opcode(105) => "Push10",
        evm::Opcode(106) => "Push11",
        evm::Opcode(107) => "Push12",
        evm::Opcode(108) => "Push13",
        evm::Opcode(109) => "Push14",
        evm::Opcode(110) => "Push15",
        evm::Opcode(111) => "Push16",
        evm::Opcode(112) => "Push17",
        evm::Opcode(113) => "Push18",
        evm::Opcode(114) => "Push19",
        evm::Opcode(115) => "Push20",
        evm::Opcode(116) => "Push21",
        evm::Opcode(117) => "Push22",
        evm::Opcode(118) => "Push23",
        evm::Opcode(119) => "Push24",
        evm::Opcode(120) => "Push25",
        evm::Opcode(121) => "Push26",
        evm::Opcode(122) => "Push27",
        evm::Opcode(123) => "Push28",
        evm::Opcode(124) => "Push29",
        evm::Opcode(125) => "Push30",
        evm::Opcode(126) => "Push31",
        evm::Opcode(127) => "Push32",
        evm::Opcode(128) => "Dup1",
        evm::Opcode(129) => "Dup2",
        evm::Opcode(130) => "Dup3",
        evm::Opcode(131) => "Dup4",
        evm::Opcode(132) => "Dup5",
        evm::Opcode(133) => "Dup6",
        evm::Opcode(134) => "Dup7",
        evm::Opcode(135) => "Dup8",
        evm::Opcode(136) => "Dup9",
        evm::Opcode(137) => "Dup10",
        evm::Opcode(138) => "Dup11",
        evm::Opcode(139) => "Dup12",
        evm::Opcode(140) => "Dup13",
        evm::Opcode(141) => "Dup14",
        evm::Opcode(142) => "Dup15",
        evm::Opcode(143) => "Dup16",
        evm::Opcode(144) => "Swap1",
        evm::Opcode(145) => "Swap2",
        evm::Opcode(146) => "Swap3",
        evm::Opcode(147) => "Swap4",
        evm::Opcode(148) => "Swap5",
        evm::Opcode(149) => "Swap6",
        evm::Opcode(150) => "Swap7",
        evm::Opcode(151) => "Swap8",
        evm::Opcode(152) => "Swap9",
        evm::Opcode(153) => "Swap10",
        evm::Opcode(154) => "Swap11",
        evm::Opcode(155) => "Swap12",
        evm::Opcode(156) => "Swap13",
        evm::Opcode(157) => "Swap14",
        evm::Opcode(158) => "Swap15",
        evm::Opcode(159) => "Swap16",
        evm::Opcode(160) => "Log0",
        evm::Opcode(161) => "Log1",
        evm::Opcode(162) => "Log2",
        evm::Opcode(163) => "Log3",
        evm::Opcode(164) => "Log4",
        evm::Opcode(176) => "JumpTo",
        evm::Opcode(177) => "JumpIf",
        evm::Opcode(178) => "JumpSub",
        evm::Opcode(180) => "JumpSubv",
        evm::Opcode(181) => "BeginSub",
        evm::Opcode(182) => "BeginData",
        evm::Opcode(184) => "ReturnSub",
        evm::Opcode(185) => "PutLocal",
        evm::Opcode(186) => "GetLocal",
        evm::Opcode(225) => "SLoadBytes",
        evm::Opcode(226) => "SStoreBytes",
        evm::Opcode(227) => "SSize",
        evm::Opcode(240) => "Create",
        evm::Opcode(241) => "Call",
        evm::Opcode(242) => "CallCode",
        evm::Opcode(243) => "Return",
        evm::Opcode(244) => "DelegateCall",
        evm::Opcode(245) => "Create2",
        evm::Opcode(250) => "StaticCall",
        evm::Opcode(252) => "TxExecGas",
        evm::Opcode(253) => "Revert",
        evm::Opcode(254) => "Invalid",
        evm::Opcode(255) => "SelfDestruct",
        evm::Opcode(_) => "Unknown",
    };

    name.as_bytes().to_vec()
}
