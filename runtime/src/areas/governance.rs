use crate::*;
use frame_support::traits::fungible::HoldConsideration;
use frame_support::traits::tokens::{PayFromAccount, UnityAssetBalanceConversion};
use frame_support::traits::{LinearStoragePrice, LockIdentifier};
use frame_support::{parameter_types, traits::EitherOfDiverse, weights::Weight, PalletId};
use frame_system::EnsureRoot;
use sp_core::ConstU32;
use sp_runtime::traits::{AccountIdConversion, IdentityLookup};
use sp_runtime::{Perbill, Permill};
use static_assertions::const_assert;

parameter_types! {
    pub const StakingRewardsPalletId: PalletId = PalletId(*b"STKREWRD");
    pub const LiquidityPalletId: PalletId = PalletId(*b"LIQDUITY");
    pub const LiquidityReservesPalletId: PalletId = PalletId(*b"LQRESVRS");
}

parameter_types! {
    pub const PreimageBaseDeposit: Balance = deposit(2, 64);
    pub const PreimageByteDeposit: Balance = deposit(0,1);
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

parameter_types! {
    pub const LaunchPeriod: BlockNumber = conf!(mainnet: 28 * DAYS, testnet: 8 * HOURS, devnet: 2 * HOURS);
    pub const VotingPeriod: BlockNumber = conf!(mainnet: 28 * DAYS, testnet: 8 * HOURS, devnet: 2 * HOURS);
    pub const FastTrackVotingPeriod: BlockNumber = conf!(mainnet: 3 * HOURS, testnet: 2 * HOURS, devnet: 30 * MINUTES);
    pub const MinimumDeposit: Balance = 100 * UNITS;
    pub const EnactmentPeriod: BlockNumber = conf!(mainnet: 28 * DAYS, testnet: 1 * HOURS, devnet: 1 * HOURS);
    pub const CooloffPeriod: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 1 * HOURS, devnet: 1 * HOURS);
    pub const InstantAllowed: bool = true;
    pub const MaxProposals: u32 = 100;
    pub const MaxVotes: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = EnactmentPeriod;
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
    type InstantAllowed = InstantAllowed;
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
    type MaxVotes = MaxVotes;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = MaxProposals;
    type Preimages = Preimage;
    type MaxDeposits = ConstU32<100>;
    type MaxBlacklisted = ConstU32<100>;
}

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"ATTREASU");
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub ProposalBondMinimum: Balance = 10 * UNITS;
    pub ProposalBondMaximum: Balance = 50 * UNITS;
    pub const SpendPeriod: BlockNumber = 30 * DAYS;
    pub const Burn: Permill = Permill::from_percent(1);
    pub const MaxApprovals: u32 = 30;

    pub const TipCountdown: BlockNumber = 2 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(5);
    pub TipReportDepositBase: Balance = deposit(1, 0);
    pub BountyDepositBase: Balance = deposit(1, 0);
    pub const BountyDepositPayoutDelay: BlockNumber = conf!(mainnet: 9 * DAYS, testnet: 6 * DAYS, devnet: 1 * DAYS);
    pub const BountyUpdatePeriod: BlockNumber = conf!(mainnet: 45 * DAYS, testnet: 35 * DAYS, devnet: 15 * DAYS);
    pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
    pub CuratorDepositMin: Balance = UNITS;
    pub CuratorDepositMax: Balance = 100 * UNITS;
    pub BountyValueMinimum: Balance = 5 * UNITS;
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
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type MaxApprovals = MaxApprovals;
    type AssetKind = ();
    type Beneficiary = AccountId;
    type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
    type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
    type BalanceConverter = UnityAssetBalanceConversion;
    type PayoutPeriod = PayoutSpendPeriod;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
    pub const CouncilMotionDuration: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 5 * DAYS, devnet: 1 * DAYS);
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
}

pub type CouncilCollective = pallet_collective::Instance1;

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
    pub const CandidacyBond: Balance = 10 * UNITS;
    pub const VotingBondBase: Balance = deposit(1, 64);
    pub const VotingBondFactor: Balance = deposit(0, 32);
    pub const TermDuration: BlockNumber = 356 * DAYS;
    pub const DesiredMembers: u32 = 4;
    pub const DesiredRunnersUp: u32 = 4;
    pub const MaxVotesPerVoter: u32 = 128;
    pub const MaxVoters: u32 = 2048;
    pub const MaxCandidates: u32 = 64;
    pub const ElectionsPhragmenPalletId: LockIdentifier = *b"PHRELECT";
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = ElectionsPhragmenPalletId;
    type Currency = Balances;
    type ChangeMembers = Council;
    type InitializeMembers = Council;
    type CurrencyToVote = U128CurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBondBase = VotingBondBase;
    type VotingBondFactor = VotingBondFactor;
    type LoserCandidate = Treasury;
    type KickedMember = Treasury;
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type MaxVoters = MaxVoters;
    type MaxVotesPerVoter = MaxVotesPerVoter;
    type MaxCandidates = MaxCandidates;
    type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TechnicalMotionDuration: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 5 * DAYS, devnet: 1 * DAYS);
    pub const TechnicalMaxProposals: u32 = 100;
    pub const TechnicalMaxMembers: u32 = 100;
}

pub type TechnicalCollective = pallet_collective::Instance2;
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

pub type EnsureRootOrHalfCouncil = EitherOfDiverse<
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
