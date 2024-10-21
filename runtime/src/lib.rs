//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

// Module level attributes
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]
#![allow(clippy::new_without_default, clippy::or_fun_call)]
#![allow(clippy::identity_op)]
#![cfg_attr(feature = "runtime-benchmarks", deny(unused_crate_dependencies))]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

// Imports
use parity_scale_codec::{Decode, Encode};
use sp_api::impl_runtime_apis;
use sp_core::{
    crypto::{ByteArray, KeyTypeId},
    OpaqueMetadata, H160, H256, U256,
};
use sp_runtime::{
    create_runtime_str,
    curve::PiecewiseLinear,
    generic::{self, Era},
    impl_opaque_keys,
    traits::{
        self, AccountIdConversion, BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable, Get,
        IdentifyAccount, IdentityLookup, Keccak256, NumberFor, One, OpaqueKeys, PostDispatchInfoOf,
        SaturatedConversion, UniqueSaturatedInto, Verify,
    },
    transaction_validity::{
        TransactionPriority, TransactionSource, TransactionValidity, TransactionValidityError,
    },
    ApplyExtrinsicResult, ConsensusEngineId, ExtrinsicInclusionMode, FixedU128, Perbill, Percent,
    Permill,
};
use sp_staking::currency_to_vote::U128CurrencyToVote;
use sp_std::{collections::btree_map::BTreeMap, marker::PhantomData, prelude::*};
use sp_version::RuntimeVersion;
// Substrate FRAME
use frame_election_provider_support::{
    bounds::ElectionBoundsBuilder, onchain, BalancingConfig, ElectionDataProvider,
    SequentialPhragmen, VoteWeight,
};
#[cfg(feature = "with-paritydb-weights")]
use frame_support::weights::constants::ParityDbWeight as RuntimeDbWeight;
#[cfg(feature = "with-rocksdb-weights")]
use frame_support::weights::constants::RocksDbWeight as RuntimeDbWeight;
use frame_support::{
    construct_runtime,
    dispatch::DispatchClass,
    genesis_builder_helper::{build_state, get_preset},
    parameter_types,
    traits::{
        fungible::HoldConsideration,
        tokens::{PayFromAccount, UnityAssetBalanceConversion},
        ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse, EqualPrivilegeOnly, FindAuthor,
        KeyOwnerProofSystem, LinearStoragePrice, LockIdentifier, OnFinalize,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_REF_TIME_PER_MILLIS},
        IdentityFee, Weight,
    },
    PalletId,
};
use pallet_election_provider_multi_phase::SolutionAccuracyOf;
use pallet_grandpa::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter};
// Frontier
use fp_account::EthereumSignature;
use fp_evm::weight_per_gas;
use fp_rpc::TransactionStatus;
use pallet_ethereum::{
    Call::transact, PostLogContent, Transaction as EthereumTransaction, TransactionAction,
    TransactionData,
};
use pallet_evm::{
    Account as EVMAccount, EnsureAccountId20, FeeCalculator, IdentityAddressMapping, Runner,
};
use sp_consensus_beefy::{
    ecdsa_crypto::{AuthorityId as BeefyId, Signature as BeefySignature},
    mmr::{BeefyDataProvider, MmrLeafVersion},
};
// Parachains
use runtime_parachains::{
    assigner_coretime as parachains_assigner_coretime,
    assigner_on_demand as parachains_assigner_on_demand, configuration as parachains_configuration,
    disputes as parachains_disputes, disputes::slashing as parachains_slashing,
    dmp as parachains_dmp, hrmp as parachains_hrmp, inclusion as parachains_inclusion,
    initializer as parachains_initializer, origin as parachains_origin, paras as parachains_paras,
    paras_inherent as parachains_paras_inherent,
    runtime_api_impl::v10 as parachains_runtime_api_impl, scheduler as parachains_scheduler,
    session_info as parachains_session_info, shared as parachains_shared,
};
// Polkadot
use polkadot_primitives::{
    runtime_api, slashing, CandidateEvent, CandidateHash, CommittedCandidateReceipt, CoreState,
    DisputeState, ExecutorParams, GroupRotationInfo, Id as ParaId, InboundDownwardMessage,
    InboundHrmpMessage, OccupiedCoreAssumption, PersistedValidationData, ScrapedOnChainVotes,
    SessionIndex, SessionInfo, ValidationCode, ValidationCodeHash, ValidatorId, ValidatorIndex,
    PARACHAIN_KEY_TYPE_ID,
};
use runtime_common::{paras_registrar, paras_sudo_wrapper, slots};
// other
use static_assertions::const_assert;

// Local imports
use constants::{currency::*, time::*};
use precompiles::FrontierPrecompiles;

// A few exports that help ease life for downstream crates.
pub use frame_system::{limits::BlockWeights, Call as SystemCall, EnsureRoot, EnsureSigned};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::Multiplier;

#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;

// Module definitions
pub mod constants;
mod precompiles;
mod utils;
mod voter_bags;
mod weights;

// XCM configurations.
pub mod xcm_config;

// Type aliases

/// Type of block number.
pub type BlockNumber = u32;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = EthereumSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem;

/// Opaque types used by the CLI to instantiate machinery without needing runtime specifics.
/// This allows them to remain compatible across runtime upgrades.
///
/// These abstractions enable the CLI to handle data like extrinsics agnostically, ensuring
/// seamless network synchronization even if core data structures change.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub babe: Babe,
            pub grandpa: Grandpa,
            pub im_online: ImOnline,
            pub para_validator: Initializer,
            pub para_assignment: ParaSessionInfo,
            pub authority_discovery: AuthorityDiscovery,
            pub beefy: Beefy,
        }
    }
}

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
    sp_consensus_babe::BabeEpochConfiguration {
        c: PRIMARY_PROBABILITY,
        allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
    };

// Runtime version
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("atleta"),
    impl_name: create_runtime_str!("atleta"),
    authoring_version: 1,
    spec_version: 4,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 3,
    state_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> sp_version::NativeVersion {
    sp_version::NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

// Constants
/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2000ms of compute with a 6 second average block time.
pub const WEIGHT_MILLISECS_PER_BLOCK: u64 = 2000;
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
    Weight::from_parts(WEIGHT_MILLISECS_PER_BLOCK * WEIGHT_REF_TIME_PER_MILLIS, u64::MAX);
pub const MAXIMUM_BLOCK_LENGTH: u32 = 5 * 1024 * 1024;

const CONTRACTS_DEBUG_OUTPUT: pallet_contracts::DebugInfo =
    pallet_contracts::DebugInfo::UnsafeDebug;
const CONTRACTS_EVENTS: pallet_contracts::CollectEvents =
    pallet_contracts::CollectEvents::UnsafeCollect;

// frame_system config
parameter_types! {
    pub const Version: RuntimeVersion = VERSION;
    pub const BlockHashCount: BlockNumber = 256;
    pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
        ::max_with_normal_ratio(MAXIMUM_BLOCK_LENGTH, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u16 = conf!(mainnet: 2440, testnet: 2340, devnet: 2300);
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The aggregated RuntimeTask type.
    type RuntimeTask = RuntimeTask;
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = frame_support::traits::Everything;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = BlockLength;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The index type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = IdentityLookup<AccountId>;
    /// The block type.
    type Block = Block;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RuntimeDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The set code logic, just the default since we're not a parachain.
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;

    /// All migrations that should run in the next runtime upgrade.
    type SingleBlockMigrations = ();
    /// The migrator that is used to run Multi-Block-Migrations.
    type MultiBlockMigrator = ();

    /// A callback that executes in _every block_ directly before all inherents were applied.
    type PreInherents = ();
    /// A callback that executes in _every block_ directly after all inherents were applied.
    type PostInherents = ();

    /// A callback that executes in _every block_ directly after all transactions were applied.
    type PostTransactions = ();
}

parameter_types! {
    // NOTE: Currently it is not possible to change the epoch duration after the chain has started.
    //       Attempting to do so will brick block production.
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
    pub const ReportLongevity: u64 =
        BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
    pub const MaxAuthorities: u32 = 10_000;
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
    type DisabledValidators = Session;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = MaxNominators;
    type KeyOwnerProof =
        <Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
    type EquivocationReportSystem =
        pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = MaxNominators;
    type MaxSetIdSessionEntries = ConstU64<168>;
    type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
    type EquivocationReportSystem =
        pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

// timestamp
parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = Moment;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

// balances
parameter_types! {
    pub const ExistentialDeposit: Balance = 30 * CENTS;
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
    pub const MaxFreezes: u32 = 1;
}

impl pallet_balances::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Self>;
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type MaxFreezes = MaxFreezes;
    type RuntimeFreezeReason = RuntimeFreezeReason;
}

// transaction payment
parameter_types! {
    pub const TransactionByteFee: Balance = 1;
    pub const TransactionPicosecondFee: Balance = 8;
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type WeightToFee = IdentityFee<Balance>;
    type LengthToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
    type OperationalFeeMultiplier = ConstU8<5>;
}

// treasury
parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"ATTREASU");
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub ProposalBondMinimum: Balance = 10 * DOLLARS;
    pub ProposalBondMaximum: Balance = 50 * DOLLARS;
    pub const SpendPeriod: BlockNumber = 30 * DAYS;
    pub const Burn: Permill = Permill::from_percent(1);

    pub const TipCountdown: BlockNumber = 2 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(5);
    pub TipReportDepositBase: Balance = deposit(1, 0);
    pub BountyDepositBase: Balance = deposit(1, 0);
    pub const BountyDepositPayoutDelay: BlockNumber = conf!(mainnet: 9 * DAYS, testnet: 6 * DAYS, devnet: 1 * DAYS);
    pub const BountyUpdatePeriod: BlockNumber = conf!(mainnet: 45 * DAYS, testnet: 35 * DAYS, devnet: 15 * DAYS);
    pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
    pub CuratorDepositMin: Balance = DOLLARS;
    pub CuratorDepositMax: Balance = 100 * DOLLARS;
    pub BountyValueMinimum: Balance = 5 * DOLLARS;
    pub DataDepositPerByte: Balance = deposit(0, 1);
    pub const MaximumReasonLength: u32 = 8192;
    pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;

    pub const SevenDays: BlockNumber = 7 * DAYS;
    pub const OneDay: BlockNumber = DAYS;

    pub TreasuryAccount: AccountId =
    TreasuryPalletId::get().try_into_account().expect("Can't create treasury account");
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type ApproveOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
    >;
    type RejectOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
    >;
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
    type RuntimeEvent = RuntimeEvent;
    type OnSlash = Treasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type ProposalBondMaximum = ProposalBondMaximum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = ();
    type WeightInfo = ();
    type MaxApprovals = ConstU32<30>;
    type AssetKind = ();
    type Beneficiary = AccountId;
    type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
    type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
    type BalanceConverter = UnityAssetBalanceConversion;
    type PayoutPeriod = PayoutSpendPeriod;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

// sudo
impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Self>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

pub enum AllowBalancesCall {}

impl frame_support::traits::Contains<RuntimeCall> for AllowBalancesCall {
    fn contains(call: &RuntimeCall) -> bool {
        matches!(call, RuntimeCall::Balances(BalancesCall::transfer_allow_death { .. }))
    }
}

const fn deposit(items: u32, bytes: u32) -> Balance {
    (items as Balance * CENTS + (bytes as Balance) * (5 * MILLICENTS / 100)) / 10
}

fn schedule<T: pallet_contracts::Config>() -> pallet_contracts::Schedule<T> {
    pallet_contracts::Schedule {
        limits: pallet_contracts::Limits {
            runtime_memory: 1024 * 1024 * 1024,
            ..Default::default()
        },
        ..Default::default()
    }
}

parameter_types! {
    pub const DepositPerItem: Balance = deposit(1, 0);
    pub const DepositPerByte: Balance = deposit(0, 1);
    pub Schedule: pallet_contracts::Schedule<Runtime> = schedule::<Runtime>();
    pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
    pub const MaxDelegateDependencies: u32 = 32;
}

impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = AllowBalancesCall;
    type DepositPerItem = DepositPerItem;
    type DepositPerByte = DepositPerByte;
    type CallStack = [pallet_contracts::Frame<Self>; 23];
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type Schedule = Schedule;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 256 * 1024 }>;
    type DefaultDepositLimit = DefaultDepositLimit;
    type MaxStorageKeyLen = ConstU32<128>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
    type UnsafeUnstableInterface = ConstBool<true>;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type MaxDelegateDependencies = MaxDelegateDependencies;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Environment = ();
    type Debug = ();
    type Migrations = ();
    type Xcm = ();
    type ApiVersion = ();
    type UploadOrigin = EnsureSigned<AccountId>;
    type InstantiateOrigin = EnsureSigned<AccountId>;
}

// election provider
parameter_types! {
    // phase durations. 1/4 of the last session for each.
    pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
    pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;

    // signed config
    pub const SignedRewardBase: Balance = DOLLARS;
    pub const SignedFixedDeposit: Balance = DOLLARS;
    pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
    pub const SignedDepositByte: Balance = CENTS;

    pub BetterUnsignedThreshold: Perbill = Perbill::from_rational(1u32, 10_000);

    // miner configs
    /// We prioritize im-online heartbeats over election solution submission.
    pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::MAX / 2;
    pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
    pub MinerMaxWeight: Weight = RuntimeBlockWeights::get()
        .get(DispatchClass::Normal)
        .max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
        .saturating_sub(BlockExecutionWeight::get());
    // Solution can occupy 90% of normal block size
    pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
        *BlockLength::get()
        .max
        .get(DispatchClass::Normal);
}

frame_election_provider_support::generate_solution_type!(
    #[compact]
    pub struct NposSolution16::<
        VoterIndex = u32,
        TargetIndex = u16,
        Accuracy = sp_runtime::PerU16,
        MaxVoters = MaxElectingVoters,
    >(16)
);

parameter_types! {
    pub const MaxNominations: u32 = <NposSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
    pub MaxElectingVoters: u32 = 40_000;
    pub MaxElectableTargets: u16 = 10_000;
    // OnChain values are lower.
    pub MaxOnChainElectingVoters: u32 = 5000;
    pub MaxOnChainElectableTargets: u16 = 1250;
    // The maximum winners that can be elected by the Election pallet which is equivalent to the
    // maximum active validators the staking pallet can have.
    pub MaxActiveValidators: u32 = 1000;
    pub ElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
        ElectionBoundsBuilder::default().voters_count(MaxElectingVoters::get().into()).build();
}

/// Configurations for benchmarking the election provider to prevent memory issues.
/// These numbers are smaller than the maximum staking pallet limits for now.
///
/// This ensures the election snapshot does not exhaust memory, and since staking is bounded,
/// the weight pipeline won't take too long to complete for this pallet.
pub struct ElectionProviderBenchmarkConfig;

impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
    const VOTERS: [u32; 2] = [1000, 2000];
    const TARGETS: [u32; 2] = [500, 1000];
    const ACTIVE_VOTERS: [u32; 2] = [500, 800];
    const DESIRED_TARGETS: [u32; 2] = [200, 400];
    const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
    const MINER_MAXIMUM_VOTERS: u32 = 1000;
    const MAXIMUM_TARGETS: u32 = 300;
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;

impl Get<Option<BalancingConfig>> for OffchainRandomBalancing {
    fn get() -> Option<BalancingConfig> {
        use sp_runtime::traits::TrailingZeroInput;
        let iterations = match MINER_MAX_ITERATIONS {
            0 => 0,
            max => {
                let seed = sp_io::offchain::random_seed();
                let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
                    .expect("input is padded with zeroes; qed")
                    % max.saturating_add(1);
                random as usize
            },
        };

        let config = BalancingConfig { iterations, tolerance: 0 };
        Some(config)
    }
}

pub struct OnChainSeqPhragmen;

impl onchain::Config for OnChainSeqPhragmen {
    type System = Runtime;
    type Solver = SequentialPhragmen<
        AccountId,
        pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
    >;
    type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
    type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
    type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
    type Bounds = ElectionBounds;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
    type AccountId = AccountId;
    type MaxLength = MinerMaxLength;
    type MaxWeight = MinerMaxWeight;
    type Solution = NposSolution16;
    type MaxVotesPerVoter =
	<<Self as pallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
    type MaxWinners = MaxActiveValidators;

    // The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
    // weight estimate function is wired to this call's weight.
    fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
        <
			<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
    }
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    RuntimeCall: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = RuntimeCall;
}

impl pallet_election_provider_multi_phase::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EstimateCallFee = TransactionPayment;
    type SignedPhase = SignedPhase;
    type UnsignedPhase = UnsignedPhase;
    // type BetterUnsignedThreshold = BetterUnsignedThreshold;
    type BetterSignedThreshold = ();
    type OffchainRepeat = OffchainRepeat;
    type MinerTxPriority = MultiPhaseUnsignedPriority;
    type MinerConfig = Self;
    type SignedMaxSubmissions = ConstU32<10>;
    type SignedRewardBase = SignedRewardBase;
    type SignedDepositBase = pallet_election_provider_multi_phase::signed::GeometricDepositBase<
        Balance,
        SignedFixedDeposit,
        SignedDepositIncreaseFactor,
    >;
    type SignedDepositByte = SignedDepositByte;
    type SignedMaxRefunds = ConstU32<3>;
    type SignedDepositWeight = ();
    type SignedMaxWeight = MinerMaxWeight;
    type SlashHandler = (); // burn slashes
    type RewardHandler = (); // nothing to do upon rewards
    type DataProvider = Staking;
    type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type MaxWinners = MaxActiveValidators;
    type ElectionBounds = ElectionBounds;
    type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
    type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Self>;
}

// voter bags
parameter_types! {
    pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
}

pub type VoterBagsListInstance = pallet_bags_list::Instance1;

impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    /// The voter bags-list is loosely kept up to date, and the real source of truth for the score
    /// of each node is the staking pallet.
    type ScoreProvider = Staking;
    type BagThresholds = BagThresholds;
    type Score = VoteWeight;
    type WeightInfo = pallet_bags_list::weights::SubstrateWeight<Runtime>;
}

// faucet
parameter_types! {
    pub AccumulationPeriod: BlockNumber = HOURS * 24;
    pub const FaucetAmount: Balance = 1000 * DOLLARS;
    pub const FaucetPalletId: PalletId = PalletId(*b"ATFAUCET");
}

impl pallet_faucet::Config for Runtime {
    type AccumulationPeriod = AccumulationPeriod;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletId = FaucetPalletId;
    type FaucetAmount = FaucetAmount;
    type WeightInfo = pallet_faucet::weights::FaucetWeight<Runtime>;
}

// nomination pools
parameter_types! {
    pub const PostUnbondPoolsWindow: u32 = 4;
    pub const NominationPoolsPalletId: PalletId = PalletId(*b"py/nopls");
    pub const MaxPointsToBalance: u8 = 10;
}

use sp_runtime::traits::Convert;
pub struct BalanceToU256;
impl Convert<Balance, sp_core::U256> for BalanceToU256 {
    fn convert(balance: Balance) -> sp_core::U256 {
        sp_core::U256::from(balance)
    }
}

pub struct U256ToBalance;
impl Convert<sp_core::U256, Balance> for U256ToBalance {
    fn convert(n: sp_core::U256) -> Balance {
        n.try_into().unwrap_or(Balance::MAX)
    }
}

impl pallet_nomination_pools::Config for Runtime {
    type WeightInfo = ();
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type RewardCounter = FixedU128;
    type BalanceToU256 = BalanceToU256;
    type U256ToBalance = U256ToBalance;
    type Staking = Staking;
    type PostUnbondingPoolsWindow = PostUnbondPoolsWindow;
    type MaxMetadataLen = ConstU32<256>;
    type MaxUnbonding = ConstU32<8>;
    type PalletId = NominationPoolsPalletId;
    type MaxPointsToBalance = MaxPointsToBalance;
    type AdminOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
    >;
}

// session
impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
    type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = opaque::SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

// Democracy
parameter_types! {
    pub const LaunchPeriod: BlockNumber = conf!(mainnet: 28 * DAYS, testnet: 8 * HOURS, devnet: 2 * HOURS);
    pub const VotingPeriod: BlockNumber = conf!(mainnet: 28 * DAYS, testnet: 8 * HOURS, devnet: 2 * HOURS);
    pub const FastTrackVotingPeriod: BlockNumber = conf!(mainnet: 3 * HOURS, testnet: 2 * HOURS, devnet: 30 * MINUTES);
    pub const MinimumDeposit: Balance = 100 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = conf!(mainnet: 28 * DAYS, testnet: 1 * HOURS, devnet: 1 * HOURS);
    pub const CooloffPeriod: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 1 * HOURS, devnet: 1 * HOURS);
    pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
    type SubmitOrigin = EnsureSigned<AccountId>;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
    type InstantOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
    type InstantAllowed = ConstBool<true>;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
    >;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    // Any single technical committee member may veto a coming council proposal, however they can
    // only do it once and it lasts only for the cool-off period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
    type CooloffPeriod = CooloffPeriod;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type MaxVotes = ConstU32<100>;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = MaxProposals;
    type Preimages = Preimage;
    type MaxDeposits = ConstU32<100>;
    type MaxBlacklisted = ConstU32<100>;
}

// Council
parameter_types! {
    pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
    pub const CouncilMotionDuration: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 5 * DAYS, devnet: 1 * DAYS);
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = EnsureRoot<Self::AccountId>;
    type MaxProposalWeight = MaxCollectivesProposalWeight;
}

parameter_types! {
    pub const CandidacyBond: Balance = 10 * DOLLARS;
    // 1 storage item created, key size is 32 bytes, value size is 16+16.
    pub const VotingBondBase: Balance = deposit(1, 64);
    // additional data per vote is 32 bytes (account id).
    pub const VotingBondFactor: Balance = deposit(0, 32);
    pub const TermDuration: BlockNumber = 7 * DAYS;
    pub const DesiredMembers: u32 = 13;
    pub const DesiredRunnersUp: u32 = 7;
    pub const MaxVotesPerVoter: u32 = 16;
    pub const MaxVoters: u32 = 512;
    pub const MaxCandidates: u32 = 64;
    pub const ElectionsPhragmenPalletId: LockIdentifier = *b"phrelect";
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = ElectionsPhragmenPalletId;
    type Currency = Balances;
    type ChangeMembers = Council;
    // NOTE: this implies that council's genesis members cannot be set directly and must come from
    // this module.
    type InitializeMembers = Council;
    type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBondBase = VotingBondBase;
    type VotingBondFactor = VotingBondFactor;
    type LoserCandidate = ();
    type KickedMember = ();
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type MaxVoters = MaxVoters;
    type MaxVotesPerVoter = MaxVotesPerVoter;
    type MaxCandidates = MaxCandidates;
    type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

// Technical Committee
parameter_types! {
    pub const TechnicalMotionDuration: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 5 * DAYS, devnet: 1 * DAYS);
    pub const TechnicalMaxProposals: u32 = 100;
    pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = TechnicalMotionDuration;
    type MaxProposals = TechnicalMaxProposals;
    type MaxMembers = TechnicalMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = EnsureRoot<Self::AccountId>;
    type MaxProposalWeight = MaxCollectivesProposalWeight;
}

// membership
type EnsureRootOrHalfCouncil = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;
impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRootOrHalfCouncil;
    type RemoveOrigin = EnsureRootOrHalfCouncil;
    type SwapOrigin = EnsureRootOrHalfCouncil;
    type ResetOrigin = EnsureRootOrHalfCouncil;
    type PrimeOrigin = EnsureRootOrHalfCouncil;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
    type MaxMembers = TechnicalMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

// preimage
parameter_types! {
    pub const PreimageBaseDeposit: Balance = DOLLARS;
    // One cent: $10,000 / MB
    pub const PreimageByteDeposit: Balance = CENTS;
    pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type Consideration = HoldConsideration<
        AccountId,
        Balances,
        PreimageHoldReason,
        LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
    >;
}

// scheduler
parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
        RuntimeBlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    #[cfg(feature = "runtime-benchmarks")]
    type MaxScheduledPerBlock = ConstU32<512>;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MaxScheduledPerBlock = ConstU32<50>;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = Preimage;
}

// staking
parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 6;
    pub const BondingDuration: sp_staking::EraIndex = conf!(mainnet:6, testnet: 2, devnet: 2);
    pub const SlashDeferDuration: sp_staking::EraIndex = conf!(mainnet:5, testnet: 1, devnet: 1);
    pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
    pub const MaxExposurePageSize: u32 = 256;
    pub const MaxControllersInDeprecationBatch: u32 = 5900;
    pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
    pub OffchainRepeat: BlockNumber = 5;
    pub HistoryDepth: u32 = 84;
    pub const MaxNominators: u32 = 64;
}

pallet_staking_reward_curve::build! {
    const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,
        max_inflation: 0_100_000,
        ideal_stake: 0_500_000,
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}

pub struct StakingBenchmarkingConfig;

impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
    type MaxNominators = ConstU32<1000>;
    type MaxValidators = ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
    type NominationsQuota = pallet_staking::FixedNominationsQuota<{ MaxNominations::get() }>;
    type Currency = Balances;
    type CurrencyBalance = Balance;
    type UnixTime = Timestamp;
    type CurrencyToVote = U128CurrencyToVote;
    type RewardRemainder = Treasury;
    type RuntimeEvent = RuntimeEvent;
    type Slash = Treasury;
    type Reward = (); // rewards are minted from the void
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    // A super-majority of the council can cancel the slash.
    type AdminOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
    >;
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type MaxExposurePageSize = MaxExposurePageSize;
    type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
    type ElectionProvider = ElectionProviderMultiPhase;
    type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type VoterList = VoterList;
    // This a placeholder, to be introduced in the next PR as an instance of bags-list
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type MaxUnlockingChunks = ConstU32<32>;
    type HistoryDepth = HistoryDepth;
    type EventListeners = NominationPools;
    type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
    type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
    type BenchmarkingConfig = StakingBenchmarkingConfig;
}

parameter_types! {
    pub const Deposit: u128 = DOLLARS;
    pub const BatchSize: u32 = 64;
    pub const MaxErasToCheckPerBlock: u32 = 1;
}

// fast unstake
impl pallet_fast_unstake::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Deposit = Deposit;
    type ControlOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
    >;
    type BatchSize = BatchSize;
    type MaxErasToCheckPerBlock = MaxErasToCheckPerBlock;
    type WeightInfo = pallet_fast_unstake::weights::SubstrateWeight<Runtime>;
    type Staking = Staking;
}

// authorship
impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type EventHandler = (Staking, ImOnline);
}

// utility
impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

// offences
impl pallet_offences::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
}

impl pallet_authority_discovery::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
}

// i'm online
parameter_types! {
    pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
    pub const MaxKeys: u32 = 10_000;
    pub const MaxPeerInHeartbeats: u32 = 10_000;
    pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
    RuntimeCall: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: RuntimeCall,
        public: <Signature as traits::Verify>::Signer,
        account: AccountId,
        nonce: Index,
    ) -> Option<(RuntimeCall, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
        let tip = 0;
        // take the biggest period possible.
        let period =
            BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
        let current_block = System::block_number()
            .saturated_into::<u64>()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let era = Era::mortal(period, current_block);
        let extra = (
            frame_system::CheckNonZeroSender::<Runtime>::new(),
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(era),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra)
            .map_err(|e| {
                log::warn!("Unable to create signed payload: {:?}", e);
            })
            .ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (account, signature, extra)))
    }
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as traits::Verify>::Signer;
    type Signature = Signature;
}

impl pallet_im_online::Config for Runtime {
    type AuthorityId = ImOnlineId;
    type RuntimeEvent = RuntimeEvent;
    type NextSessionRotation = Babe;
    type ValidatorSet = Historical;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
    type MaxKeys = MaxKeys;
    type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

parameter_types! {
    pub const BeefySetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_beefy::Config for Runtime {
    type BeefyId = BeefyId;
    type MaxAuthorities = MaxAuthorities;
    type MaxSetIdSessionEntries = BeefySetIdSessionEntries;
    type MaxNominators = MaxNominators;
    type OnNewValidatorSet = MmrLeaf;
    type WeightInfo = ();
    type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, BeefyId)>>::Proof;
    type EquivocationReportSystem =
        pallet_beefy::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

mod mmr {
    use super::Runtime;
    pub use pallet_mmr::primitives::*;

    pub type Leaf = <<Runtime as pallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
    pub type Hashing = <Runtime as pallet_mmr::Config>::Hashing;
}

impl pallet_mmr::Config for Runtime {
    const INDEXING_PREFIX: &'static [u8] = mmr::INDEXING_PREFIX;
    type Hashing = Keccak256;
    type OnNewRoot = pallet_beefy_mmr::DepositBeefyDigest<Runtime>;
    type WeightInfo = ();
    type LeafData = pallet_beefy_mmr::Pallet<Runtime>;
    type BlockHashProvider = pallet_mmr::DefaultBlockHashProvider<Runtime>;
}

pub struct ParaHeadsRootProvider;
impl BeefyDataProvider<H256> for ParaHeadsRootProvider {
    fn extra_data() -> H256 {
        let mut para_heads: Vec<(u32, Vec<u8>)> = parachains_paras::Parachains::<Runtime>::get()
            .into_iter()
            .filter_map(|id| {
                parachains_paras::Heads::<Runtime>::get(id).map(|head| (id.into(), head.0))
            })
            .collect();
        para_heads.sort();
        binary_merkle_tree::merkle_root::<mmr::Hashing, _>(
            para_heads.into_iter().map(|pair| pair.encode()),
        )
    }
}

impl pallet_beefy_mmr::Config for Runtime {
    type LeafVersion = LeafVersion;
    type BeefyAuthorityToMerkleLeaf = pallet_beefy_mmr::BeefyEcdsaToEthereum;
    type LeafExtra = H256;
    type BeefyDataProvider = ParaHeadsRootProvider;
}

parameter_types! {
    /// Version of the produced MMR leaf.
    ///
    /// The version consists of two parts;
    /// - `major` (3 bits)
    /// - `minor` (5 bits)
    ///
    /// `major` should be updated only if decoding the previous MMR Leaf format from the payload
    /// is not possible (i.e. backward incompatible change).
    /// `minor` should be updated if fields are added to the previous MMR Leaf, which given SCALE
    /// encoding does not prevent old leafs from being decoded.
    ///
    /// Hence we expect `major` to be changed really rarely (think never).
    /// See [`MmrLeafVersion`] type documentation for more details.
    pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(0, 0);
}

// EVM
impl pallet_evm_chain_id::Config for Runtime {}

pub struct FindAuthorTruncated<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        if let Some(author_index) = F::find_author(digests) {
            let (public, _) = Babe::authorities()[author_index as usize].clone();
            return Some(H160::from_slice(&public.to_raw_vec()[4..24]));
        }
        None
    }
}

const BLOCK_GAS_LIMIT: u64 = 75_000_000;
const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
    pub const GasLimitPovSizeRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_POV_SIZE);
    pub PrecompilesValue: FrontierPrecompiles<Runtime> = FrontierPrecompiles::<_>::new();
    pub WeightPerGas: Weight = Weight::from_parts(weight_per_gas(BLOCK_GAS_LIMIT, NORMAL_DISPATCH_RATIO, WEIGHT_MILLISECS_PER_BLOCK), 0);
    pub SuicideQuickClearLimit: u32 = 0;
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = BaseFee;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
    type CallOrigin = EnsureAccountId20;
    type WithdrawOrigin = EnsureAccountId20;
    type AddressMapping = IdentityAddressMapping;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type PrecompilesType = FrontierPrecompiles<Self>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = EVMChainId;
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = ();
    type OnCreate = ();
    type FindAuthor = FindAuthorTruncated<Babe>;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type Timestamp = Timestamp;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
    type SuicideQuickClearLimit = SuicideQuickClearLimit;
}

// ethereum
parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
    type PostLogContent = PostBlockAndTxnHashes;
    type ExtraDataLength = ConstU32<30>;
}

// dynamic fee
parameter_types! {
    pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
    type MinGasPriceBoundDivisor = BoundDivision;
}

// base fee
parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(1_000_000_000);
    pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;

impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
    fn lower() -> Permill {
        Permill::zero()
    }
    fn ideal() -> Permill {
        Permill::from_parts(500_000)
    }
    fn upper() -> Permill {
        Permill::from_parts(1_000_000)
    }
}

impl pallet_base_fee::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Threshold = BaseFeeThreshold;
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    type DefaultElasticity = DefaultElasticity;
}

// hotfix sufficients
impl pallet_hotfix_sufficients::Config for Runtime {
    type AddressMapping = IdentityAddressMapping;
    type WeightInfo = pallet_hotfix_sufficients::weights::SubstrateWeight<Self>;
}

impl parachains_origin::Config for Runtime {}

impl parachains_configuration::Config for Runtime {
    type WeightInfo = weights::runtime_parachains_configuration::WeightInfo<Runtime>;
}

impl parachains_shared::Config for Runtime {
    type DisabledValidators = Session;
}

impl parachains_session_info::Config for Runtime {
    type ValidatorSet = Historical;
}

/// Special `RewardValidators` that does nothing ;)
pub struct RewardValidators;
impl runtime_parachains::inclusion::RewardValidators for RewardValidators {
    fn reward_backing(_: impl IntoIterator<Item = ValidatorIndex>) {}
    fn reward_bitfields(_: impl IntoIterator<Item = ValidatorIndex>) {}
}

impl parachains_inclusion::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type DisputesHandler = ParasDisputes;
    type RewardValidators = RewardValidators;
    type MessageQueue = ();
    type WeightInfo = weights::runtime_parachains_inclusion::WeightInfo<Runtime>;
}

parameter_types! {
    pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
}

impl parachains_paras::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::runtime_parachains_paras::WeightInfo<Runtime>;
    type UnsignedPriority = ParasUnsignedPriority;
    type QueueFootprinter = ParaInclusion;
    type NextSessionRotation = Babe;
    type OnNewHead = Registrar;
    type AssignCoretime = CoretimeAssignmentProvider;
}

parameter_types! {
    /// Amount of weight that can be spent per block to service messages.
    ///
    /// # WARNING
    ///
    /// This is not a good value for para-chains since the `Scheduler` already uses up to 80% block weight.
    pub MessageQueueServiceWeight: Weight = Perbill::from_percent(20) * RuntimeBlockWeights::get().max_block;
    pub const MessageQueueHeapSize: u32 = 65_536;
    pub const MessageQueueMaxStale: u32 = 8;
}

impl parachains_dmp::Config for Runtime {}

parameter_types! {
    pub const DefaultChannelSizeAndCapacityWithSystem: (u32, u32) = (51200, 500);
}

impl parachains_hrmp::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ChannelManager = EnsureRoot<AccountId>;
    type DefaultChannelSizeAndCapacityWithSystem = DefaultChannelSizeAndCapacityWithSystem;
    type WeightInfo = weights::runtime_parachains_hrmp::WeightInfo<Self>;
}

impl parachains_paras_inherent::Config for Runtime {
    type WeightInfo = weights::runtime_parachains_paras_inherent::WeightInfo<Runtime>;
}

impl parachains_scheduler::Config for Runtime {
    type AssignmentProvider = CoretimeAssignmentProvider;
}

parameter_types! {
    pub const OnDemandTrafficDefaultValue: FixedU128 = FixedU128::from_u32(1);
}

impl parachains_assigner_on_demand::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type TrafficDefaultValue = OnDemandTrafficDefaultValue;
    type WeightInfo = weights::runtime_parachains_assigner_on_demand::WeightInfo<Runtime>;
}

impl parachains_assigner_coretime::Config for Runtime {}

impl parachains_initializer::Config for Runtime {
    type Randomness = pallet_babe::RandomnessFromOneEpochAgo<Runtime>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type CoretimeOnNewSession = ();
    type WeightInfo = weights::runtime_parachains_initializer::WeightInfo<Runtime>;
}

impl parachains_disputes::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RewardValidators = ();
    type SlashingHandler = parachains_slashing::SlashValidatorsForDisputes<ParasSlashing>;
    type WeightInfo = weights::runtime_parachains_disputes::WeightInfo<Runtime>;
}

impl parachains_slashing::Config for Runtime {
    type KeyOwnerProofSystem = Historical;
    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, ValidatorId)>>::Proof;
    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        ValidatorId,
    )>>::IdentificationTuple;
    type HandleReports = parachains_slashing::SlashingReportHandler<
        Self::KeyOwnerIdentification,
        Offences,
        ReportLongevity,
    >;
    type WeightInfo = parachains_slashing::TestWeightInfo;
    type BenchmarkingConfig = parachains_slashing::BenchConfig<1000>;
}

parameter_types! {
    pub const ParaDeposit: Balance = 20000 * DOLLARS;
    pub const ParaDataByteDeposit: Balance = 2;
}

impl paras_registrar::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type OnSwap = Slots;
    type ParaDeposit = ParaDeposit;
    type DataDepositPerByte = ParaDataByteDeposit;
    type WeightInfo = weights::runtime_common_paras_registrar::WeightInfo<Runtime>;
}

parameter_types! {
    pub LeasePeriod: BlockNumber = conf!(mainnet: 24 * DAYS, testnet: 1 * DAYS, devnet: 2 * HOURS);
}

impl slots::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Registrar = Registrar;
    type LeasePeriod = LeasePeriod;
    type LeaseOffset = ();
    type ForceOrigin = EnsureRoot<Self::AccountId>;
    type WeightInfo = weights::runtime_common_slots::WeightInfo<Runtime>;
}

impl paras_sudo_wrapper::Config for Runtime {}

// Construct runtime
// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Babe: pallet_babe,
        Grandpa: pallet_grandpa,
        Balances: pallet_balances,
        Faucet: pallet_faucet,
        TransactionPayment: pallet_transaction_payment,
        Sudo: pallet_sudo,
        Treasury: pallet_treasury,
        Ethereum: pallet_ethereum,
        // Authorship must be before session in order to note author in the correct session and era
        // for im-online and staking.
        Authorship: pallet_authorship,
        Utility: pallet_utility,
        Offences: pallet_offences,
        ImOnline: pallet_im_online,
        // staking related pallets
        ElectionProviderMultiPhase: pallet_election_provider_multi_phase,
        Staking: pallet_staking,
        FastUnstake: pallet_fast_unstake,
        Session: pallet_session,
        Democracy: pallet_democracy,
        Council: pallet_collective::<Instance1>,
        TechnicalCommittee: pallet_collective::<Instance2>,
        Elections: pallet_elections_phragmen,
        TechnicalMembership: pallet_membership::<Instance1>,
        VoterList: pallet_bags_list::<Instance1>,
        Historical: pallet_session::historical::{Pallet},
        AuthorityDiscovery: pallet_authority_discovery,
        Scheduler: pallet_scheduler,
        Preimage: pallet_preimage,
        NominationPools: pallet_nomination_pools,
        // EVM
        EVM: pallet_evm,
        EVMChainId: pallet_evm_chain_id,
        DynamicFee: pallet_dynamic_fee,
        BaseFee: pallet_base_fee,
        HotfixSufficients: pallet_hotfix_sufficients,
        // Smart contracts
        RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
        Contracts: pallet_contracts,
        // Parachains pallets
        ParachainsOrigin: parachains_origin,
        Configuration: parachains_configuration,
        ParasShared: parachains_shared,
        ParaInclusion: parachains_inclusion,
        ParaInherent: parachains_paras_inherent,
        ParaScheduler: parachains_scheduler,
        Paras: parachains_paras,
        Initializer: parachains_initializer,
        Dmp: parachains_dmp,
        Hrmp: parachains_hrmp,
        ParaSessionInfo: parachains_session_info,
        ParasDisputes: parachains_disputes,
        ParasSlashing: parachains_slashing,
        OnDemandAssignmentProvider: parachains_assigner_on_demand,
        CoretimeAssignmentProvider: parachains_assigner_coretime,
        // Parachain Onboarding Pallets. Start indices at 80 to leave room.
        Registrar: paras_registrar,
        Slots: slots,
        ParasSudoWrapper: paras_sudo_wrapper,
        // Pallet for sending XCM.
        XcmPallet: pallet_xcm,
        // Generalized message queue
        // MessageQueue: pallet_message_queue,
        Beefy: pallet_beefy,
        // MMR leaf construction must be after session in order to have a leaf's next_auth_set
        // refer to block<N>. See https://github.com/polkadot-fellows/runtimes/issues/160 for details.
        Mmr: pallet_mmr,
        MmrLeaf: pallet_beefy_mmr,
    }
);

#[derive(Clone)]
pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
    fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
        UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        )
    }
}

impl fp_rpc::ConvertTransaction<opaque::UncheckedExtrinsic> for TransactionConverter {
    fn convert_transaction(
        &self,
        transaction: pallet_ethereum::Transaction,
    ) -> opaque::UncheckedExtrinsic {
        let extrinsic = UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        );
        let encoded = extrinsic.encode();
        opaque::UncheckedExtrinsic::decode(&mut &encoded[..])
            .expect("Encoded extrinsic is always valid")
    }
}

/// The address format for describing accounts.
pub type Address = AccountId;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic =
    fp_self_contained::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra, H160>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

/// All migrations that will run on the next runtime upgrade.
///
/// This contains the combined migrations of the last 10 releases. It allows to skip runtime
/// upgrades in case governance decides to do so. THE ORDER IS IMPORTANT.
pub type Migrations = migrations::Unreleased;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

pub type EventRecord = frame_system::EventRecord<
    <Runtime as frame_system::Config>::RuntimeEvent,
    <Runtime as frame_system::Config>::Hash,
>;

/// The runtime migrations per release.
#[allow(deprecated, missing_docs)]
pub mod migrations {
    pub type Unreleased = ();
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
    type SignedInfo = H160;

    fn is_self_contained(&self) -> bool {
        match self {
            RuntimeCall::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => {
                call.pre_dispatch_self_contained(info, dispatch_info, len)
            },
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
                Some(call.dispatch(RuntimeOrigin::from(
                    pallet_ethereum::RawOrigin::EthereumTransaction(info),
                )))
            },
            _ => None,
        }
    }
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    define_benchmarks!(
        [frame_benchmarking, BaselineBench::<Runtime>]
        [frame_system, SystemBench::<Runtime>]
        [pallet_babe, Babe]
        [pallet_balances, Balances]
        [pallet_timestamp, Timestamp]
        [pallet_sudo, Sudo]
        [pallet_evm, EVM]
    );
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> sp_consensus_grandpa::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_grandpa::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Grandpa::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_grandpa::SetId,
            authority_id: GrandpaId,
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
            use parity_scale_codec::Encode;

            Historical::prove((sp_consensus_grandpa::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
        }
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeConfiguration {
            let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
            sp_consensus_babe::BabeConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: epoch_config.c,
                authorities: Babe::authorities().to_vec(),
                randomness: Babe::randomness(),
                allowed_slots: epoch_config.allowed_slots,
            }
        }

        fn current_epoch_start() -> sp_consensus_babe::Slot {
            Babe::current_epoch_start()
        }

        fn current_epoch() -> sp_consensus_babe::Epoch {
            Babe::current_epoch()
        }

        fn next_epoch() -> sp_consensus_babe::Epoch {
            Babe::next_epoch()
        }

        fn generate_key_ownership_proof(
            _slot: sp_consensus_babe::Slot,
            authority_id: sp_consensus_babe::AuthorityId,
        ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
            use parity_scale_codec::Encode;

            Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
            key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Babe::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }
    }

    impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
        fn authorities() -> Vec<sp_authority_discovery::AuthorityId> {
            parachains_runtime_api_impl::relevant_authority_ids::<Runtime>()
        }
    }

    impl sp_consensus_beefy::BeefyApi<Block, BeefyId> for Runtime {
        fn beefy_genesis() -> Option<BlockNumber> {
            pallet_beefy::GenesisBlock::<Runtime>::get()
        }

        fn validator_set() -> Option<sp_consensus_beefy::ValidatorSet<BeefyId>> {
            Beefy::validator_set()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_beefy::EquivocationProof<
                BlockNumber,
                BeefyId,
                BeefySignature,
            >,
            key_owner_proof: sp_consensus_beefy::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Beefy::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_beefy::ValidatorSetId,
            authority_id: BeefyId,
        ) -> Option<sp_consensus_beefy::OpaqueKeyOwnershipProof> {
            use parity_scale_codec::Encode;

            Historical::prove((sp_consensus_beefy::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_beefy::OpaqueKeyOwnershipProof::new)
        }
    }

    impl pallet_nomination_pools_runtime_api::NominationPoolsApi<Block, AccountId, Balance> for Runtime {
        fn pending_rewards(who: AccountId) -> Balance {
            NominationPools::api_pending_rewards(who).unwrap_or_default()
        }

        fn points_to_balance(pool_id: pallet_nomination_pools::PoolId, points: Balance) -> Balance {
            NominationPools::api_points_to_balance(pool_id, points)
        }

        fn balance_to_points(pool_id: pallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
            NominationPools::api_balance_to_points(pool_id, new_funds)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }

    impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
        fn nominations_quota(balance: Balance) -> u32 {
            Staking::api_nominations_quota(balance)
        }

        fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
            Staking::api_eras_stakers_page_count(era, account)
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            <Runtime as pallet_evm::Config>::ChainId::get()
        }

        fn account_basic(address: H160) -> EVMAccount {
            let (account, _) = pallet_evm::Pallet::<Runtime>::account_basic(&address);
            account
        }

        fn gas_price() -> U256 {
            let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
            gas_price
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            pallet_evm::AccountCodes::<Runtime>::get(address)
        }

        fn author() -> H160 {
            <pallet_evm::Pallet<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: U256) -> H256 {
            let mut tmp = [0u8; 32];
            index.to_big_endian(&mut tmp);
            pallet_evm::AccountStorages::<Runtime>::get(address, H256::from_slice(&tmp[..]))
        }

        fn call(
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let gas_limit = gas_limit.min(u64::MAX.into());
            let transaction_data = TransactionData::new(
                TransactionAction::Call(to),
                data.clone(),
                nonce.unwrap_or_default(),
                gas_limit,
                None,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                value,
                Some(<Runtime as pallet_evm::Config>::ChainId::get()),
                access_list.clone().unwrap_or_default(),
            );
            let (weight_limit, proof_size_base_cost) = pallet_ethereum::Pallet::<Runtime>::transaction_weight(&transaction_data);

            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit.unique_saturated_into(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                false,
                true,
                weight_limit,
                proof_size_base_cost,
                config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.error.into())
        }

        fn create(
            from: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let transaction_data = TransactionData::new(
                TransactionAction::Create,
                data.clone(),
                nonce.unwrap_or_default(),
                gas_limit,
                None,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                value,
                Some(<Runtime as pallet_evm::Config>::ChainId::get()),
                access_list.clone().unwrap_or_default(),
            );
            let (weight_limit, proof_size_base_cost) = pallet_ethereum::Pallet::<Runtime>::transaction_weight(&transaction_data);

            <Runtime as pallet_evm::Config>::Runner::create(
                from,
                data,
                value,
                gas_limit.unique_saturated_into(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                false,
                true,
                weight_limit,
                proof_size_base_cost,
                config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.error.into())
        }

        fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
            pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            pallet_ethereum::CurrentBlock::<Runtime>::get()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            pallet_ethereum::CurrentReceipts::<Runtime>::get()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<TransactionStatus>>
        ) {
            (
                pallet_ethereum::CurrentBlock::<Runtime>::get(),
                pallet_ethereum::CurrentReceipts::<Runtime>::get(),
                pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
            )
        }

        fn extrinsic_filter(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> Vec<EthereumTransaction> {
            xts.into_iter().filter_map(|xt| match xt.0.function {
                RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
                _ => None
            }).collect::<Vec<EthereumTransaction>>()
        }

        fn elasticity() -> Option<Permill> {
            Some(pallet_base_fee::Elasticity::<Runtime>::get())
        }

        fn gas_limit_multiplier_support() {}

        fn pending_block(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> (Option<pallet_ethereum::Block>, Option<Vec<TransactionStatus>>) {
            for ext in xts.into_iter() {
                let _ = Executive::apply_extrinsic(ext);
            }

            Ethereum::on_finalize(System::block_number() + 1);

            (
                pallet_ethereum::CurrentBlock::<Runtime>::get(),
                pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
            )
        }
    }

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
        fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
            UncheckedExtrinsic::new_unsigned(
                pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
            )
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }

        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }

        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_beefy_mmr::BeefyMmrApi<Block, Hash> for RuntimeApi {
        fn authority_set_proof() -> sp_consensus_beefy::mmr::BeefyAuthoritySet<Hash> {
            MmrLeaf::authority_set_proof()
        }

        fn next_authority_set_proof() -> sp_consensus_beefy::mmr::BeefyNextAuthoritySet<Hash> {
            MmrLeaf::next_authority_set_proof()
        }
    }

    impl pallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord>
        for Runtime
    {
        fn call(
            origin: AccountId,
            dest: AccountId,
            value: Balance,
            gas_limit: Option<Weight>,
            storage_deposit_limit: Option<Balance>,
            input_data: Vec<u8>,
        ) -> pallet_contracts::ContractExecResult<Balance, EventRecord> {
            let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
            Contracts::bare_call(
                origin,
                dest,
                value,
                gas_limit,
                storage_deposit_limit,
                input_data,
                CONTRACTS_DEBUG_OUTPUT,
                CONTRACTS_EVENTS,
                pallet_contracts::Determinism::Enforced,
            )
        }

        fn instantiate(
            origin: AccountId,
            value: Balance,
            gas_limit: Option<Weight>,
            storage_deposit_limit: Option<Balance>,
            code: pallet_contracts::Code<Hash>,
            data: Vec<u8>,
            salt: Vec<u8>,
        ) -> pallet_contracts::ContractInstantiateResult<AccountId, Balance, EventRecord>
        {
            let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
            Contracts::bare_instantiate(
                origin,
                value,
                gas_limit,
                storage_deposit_limit,
                code,
                data,
                salt,
                CONTRACTS_DEBUG_OUTPUT,
                CONTRACTS_EVENTS,
            )
        }

        fn upload_code(
            origin: AccountId,
            code: Vec<u8>,
            storage_deposit_limit: Option<Balance>,
            determinism: pallet_contracts::Determinism,
        ) -> pallet_contracts::CodeUploadResult<Hash, Balance>
        {
            Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
        }

        fn get_storage(
            address: AccountId,
            key: Vec<u8>,
        ) -> pallet_contracts::GetStorageResult {
            Contracts::get_storage(address, key)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl sp_mmr_primitives::MmrApi<Block, Hash, BlockNumber> for Runtime {
        fn mmr_root() -> Result<Hash, sp_mmr_primitives::Error> {
            Ok(Mmr::mmr_root())
        }

        fn mmr_leaf_count() -> Result<sp_mmr_primitives::LeafIndex, sp_mmr_primitives::Error> {
            Ok(Mmr::mmr_leaves())
        }

        fn generate_proof(
            block_numbers: Vec<BlockNumber>,
            best_known_block_number: Option<BlockNumber>,
        ) -> Result<(Vec<sp_mmr_primitives::EncodableOpaqueLeaf>, sp_mmr_primitives::Proof<Hash>), sp_mmr_primitives::Error> {
             Mmr::generate_proof(block_numbers, best_known_block_number).map(
                |(leaves, proof)| {
                    (
                        leaves
                            .into_iter()
                            .map(|leaf| mmr::EncodableOpaqueLeaf::from_leaf(&leaf))
                            .collect(),
                        proof,
                    )
                },
            )
        }

        fn verify_proof(leaves: Vec<sp_mmr_primitives::EncodableOpaqueLeaf>, proof: sp_mmr_primitives::Proof<Hash>)
            -> Result<(), sp_mmr_primitives::Error>
        {
             let leaves = leaves.into_iter().map(|leaf|
                leaf.into_opaque_leaf()
                .try_decode()
                .ok_or(mmr::Error::Verify)).collect::<Result<Vec<mmr::Leaf>, mmr::Error>>()?;
            Mmr::verify_leaves(leaves, proof)
        }

        fn verify_proof_stateless(
            root: Hash,
            leaves: Vec<sp_mmr_primitives::EncodableOpaqueLeaf>,
            proof: sp_mmr_primitives::Proof<Hash>
        ) -> Result<(), sp_mmr_primitives::Error> {
            let nodes = leaves.into_iter().map(|leaf|mmr::DataOrHash::Data(leaf.into_opaque_leaf())).collect();
            pallet_mmr::verify_leaves_proof::<mmr::Hashing, _>(root, nodes, proof)
        }
    }

    impl runtime_api::ParachainHost<Block> for Runtime {
        fn validators() -> Vec<ValidatorId> {
            parachains_runtime_api_impl::validators::<Runtime>()
        }

        fn validator_groups() -> (Vec<Vec<ValidatorIndex>>, GroupRotationInfo<BlockNumber>) {
            parachains_runtime_api_impl::validator_groups::<Runtime>()
        }

        fn availability_cores() -> Vec<CoreState<Hash, BlockNumber>> {
            parachains_runtime_api_impl::availability_cores::<Runtime>()
        }

        fn persisted_validation_data(para_id: ParaId, assumption: OccupiedCoreAssumption)
            -> Option<PersistedValidationData<Hash, BlockNumber>> {
            parachains_runtime_api_impl::persisted_validation_data::<Runtime>(para_id, assumption)
        }

        fn assumed_validation_data(
            para_id: ParaId,
            expected_persisted_validation_data_hash: Hash,
        ) -> Option<(PersistedValidationData<Hash, BlockNumber>, ValidationCodeHash)> {
            parachains_runtime_api_impl::assumed_validation_data::<Runtime>(
                para_id,
                expected_persisted_validation_data_hash,
            )
        }

        fn check_validation_outputs(
            para_id: ParaId,
            outputs: polkadot_primitives::CandidateCommitments,
        ) -> bool {
            parachains_runtime_api_impl::check_validation_outputs::<Runtime>(para_id, outputs)
        }

        fn session_index_for_child() -> SessionIndex {
            parachains_runtime_api_impl::session_index_for_child::<Runtime>()
        }

        fn validation_code(para_id: ParaId, assumption: OccupiedCoreAssumption)
            -> Option<ValidationCode> {
            parachains_runtime_api_impl::validation_code::<Runtime>(para_id, assumption)
        }

        fn candidate_pending_availability(para_id: ParaId) -> Option<CommittedCandidateReceipt<Hash>> {
            #[allow(deprecated)]
            parachains_runtime_api_impl::candidate_pending_availability::<Runtime>(para_id)
        }

        fn candidate_events() -> Vec<CandidateEvent<Hash>> {
            parachains_runtime_api_impl::candidate_events::<Runtime, _>(|ev| {
                match ev {
                    RuntimeEvent::ParaInclusion(ev) => {
                        Some(ev)
                    }
                    _ => None,
                }
            })
        }

        fn session_info(index: SessionIndex) -> Option<SessionInfo> {
            parachains_runtime_api_impl::session_info::<Runtime>(index)
        }

        fn session_executor_params(session_index: SessionIndex) -> Option<ExecutorParams> {
            parachains_runtime_api_impl::session_executor_params::<Runtime>(session_index)
        }

        fn dmq_contents(recipient: ParaId) -> Vec<InboundDownwardMessage<BlockNumber>> {
            parachains_runtime_api_impl::dmq_contents::<Runtime>(recipient)
        }

        fn inbound_hrmp_channels_contents(
            recipient: ParaId
        ) -> BTreeMap<ParaId, Vec<InboundHrmpMessage<BlockNumber>>> {
            parachains_runtime_api_impl::inbound_hrmp_channels_contents::<Runtime>(recipient)
        }

        fn validation_code_by_hash(hash: ValidationCodeHash) -> Option<ValidationCode> {
            parachains_runtime_api_impl::validation_code_by_hash::<Runtime>(hash)
        }

        fn on_chain_votes() -> Option<ScrapedOnChainVotes<Hash>> {
            parachains_runtime_api_impl::on_chain_votes::<Runtime>()
        }

        fn submit_pvf_check_statement(
            stmt: polkadot_primitives::PvfCheckStatement,
            signature: polkadot_primitives::ValidatorSignature
        ) {
            parachains_runtime_api_impl::submit_pvf_check_statement::<Runtime>(stmt, signature)
        }

        fn pvfs_require_precheck() -> Vec<ValidationCodeHash> {
            parachains_runtime_api_impl::pvfs_require_precheck::<Runtime>()
        }

        fn validation_code_hash(para_id: ParaId, assumption: OccupiedCoreAssumption)
            -> Option<ValidationCodeHash>
        {
            parachains_runtime_api_impl::validation_code_hash::<Runtime>(para_id, assumption)
        }

        fn disputes() -> Vec<(SessionIndex, CandidateHash, DisputeState<BlockNumber>)> {
            parachains_runtime_api_impl::get_session_disputes::<Runtime>()
        }

        fn unapplied_slashes(
        ) -> Vec<(SessionIndex, CandidateHash, slashing::PendingSlashes)> {
            parachains_runtime_api_impl::unapplied_slashes::<Runtime>()
        }

        fn key_ownership_proof(
            validator_id: ValidatorId,
        ) -> Option<slashing::OpaqueKeyOwnershipProof> {
            use parity_scale_codec::Encode;

            Historical::prove((PARACHAIN_KEY_TYPE_ID, validator_id))
                .map(|p| p.encode())
                .map(slashing::OpaqueKeyOwnershipProof::new)
        }

        fn submit_report_dispute_lost(
            dispute_proof: slashing::DisputeProof,
            key_ownership_proof: slashing::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            parachains_runtime_api_impl::submit_unsigned_slashing_report::<Runtime>(
                dispute_proof,
                key_ownership_proof,
            )
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;

            use baseline::Pallet as BaselineBench;
            use frame_system_benchmarking::Pallet as SystemBench;
            use pallet_hotfix_sufficients::Pallet as PalletHotfixSufficients;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);
            list_benchmark!(list, extra, pallet_hotfix_sufficients, PalletHotfixSufficients::<Runtime>);

            let storage_info = AllPalletsWithSystem::storage_info();
            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, add_benchmark};
            use frame_support::traits::TrackedStorageKey;

            use pallet_evm::Pallet as PalletEvmBench;
            use pallet_hotfix_sufficients::Pallet as PalletHotfixSufficientsBench;

            impl baseline::Config for Runtime {}
            impl frame_system_benchmarking::Config for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmark!(params, batches, pallet_evm, PalletEvmBench::<Runtime>);
            add_benchmark!(params, batches, pallet_hotfix_sufficients, PalletHotfixSufficientsBench::<Runtime>);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            log::info!("try-runtime::on_runtime_upgrade atleta.");
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect,
        ) -> Weight {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here.
            Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Runtime, WeightPerGas};
    #[test]
    fn configured_base_extrinsic_weight_is_evm_compatible() {
        let min_ethereum_transaction_weight = WeightPerGas::get() * 21_000;
        let base_extrinsic = <Runtime as frame_system::Config>::BlockWeights::get()
            .get(frame_support::dispatch::DispatchClass::Normal)
            .base_extrinsic;
        assert!(base_extrinsic.ref_time() <= min_ethereum_transaction_weight.ref_time());
    }
}
