//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
    use crate::Balance;

    pub const UNITS: Balance = 1_000_000_000_000_000_000; // 1 ATLA (assuming it's the base unit)
    pub const MILLI_ATLA: Balance = UNITS / 1_000;
    pub const MICRO_ATLA: Balance = MILLI_ATLA / 1_000;
    pub const NANO_ATLA: Balance = MICRO_ATLA / 1_000;
    pub const PICO_ATLA: Balance = NANO_ATLA / 1_000;
    pub const FEMTO_ATLA: Balance = PICO_ATLA / 1_000;
    pub const ATTO_VTRS: Balance = FEMTO_ATLA / 1_000;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 150 * MILLI_ATLA + (bytes as Balance) * 60 * MILLI_ATLA
    }
}

/// Time.
pub mod time {
    use crate::{conf, BlockNumber};

    pub const MILLISECS_PER_BLOCK: u64 = 6000;

    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    // Time is measured by number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;

    // NOTE: Currently it is not possible to change the epoch duration after the chain has started.
    //       Attempting to do so will brick block production.
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber =
        conf!(mainnet: 60 * MINUTES, testnet: 10 * MINUTES, devnet: 10 * MINUTES);
    pub const EPOCH_DURATION_IN_SLOTS: u64 = {
        const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

        (EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
    };

    // 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
    // The choice of is done in accordance to the slot duration and expected target
    // block time, for safely resisting network delays of maximum two seconds.
    // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}
