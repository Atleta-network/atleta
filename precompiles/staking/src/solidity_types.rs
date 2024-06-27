use precompile_utils::{
    prelude::*,
    solidity::codec::{Reader, Writer},
};

#[repr(u8)]
pub enum RewardDestinationKind {
    Staked = 0,
    Stash,
    Account,
    None,
}

impl RewardDestinationKind {
    pub fn conv_with<Addr>(&self, addr: Addr) -> super::RewardDestination<Addr> {
        match self {
            Self::Staked => super::RewardDestination::Staked,
            Self::Stash => super::RewardDestination::Stash,
            Self::Account => super::RewardDestination::Account(addr),
            Self::None => super::RewardDestination::None,
        }
    }
}

impl solidity::Codec for RewardDestinationKind {
    fn read(reader: &mut Reader) -> MayRevert<Self> {
        match reader.read().in_field("variant")? {
            0u8 => Ok(Self::Staked),
            1u8 => Ok(Self::Stash),
            2u8 => Ok(Self::Account),
            3u8 => Ok(Self::None),
            _ => Err(RevertReason::custom("Unknown RewardDestinationKind variant").into()),
        }
    }

    fn write(writer: &mut Writer, value: Self) {
        let encoded = value as u8;
        solidity::Codec::write(writer, encoded);
    }

    fn has_static_size() -> bool {
        true
    }
    fn signature() -> String {
        u8::signature()
    }
}

#[derive(solidity::Codec)]
pub struct Foobar {
    kind: RewardDestinationKind,
    address: Address,
}
