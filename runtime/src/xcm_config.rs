// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! XCM configurations for Westend.

use super::{
    parachains_origin, AccountId, AllPalletsWithSystem, Balances, Dmp, ParaId, Runtime,
    RuntimeCall, RuntimeEvent, RuntimeOrigin, TransactionByteFee, Treasury, XcmPallet, CENTS,
};
use frame_support::{
    parameter_types,
    traits::{Contains, Equals, Everything, Nothing},
};
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use runtime_common::{
    xcm_sender::{ChildParachainRouter, ExponentialPrice},
    ToAuthor,
};
use sp_core::ConstU32;
use xcm::latest::prelude::*;
use xcm::opaque::v3::{MultiAsset, MultiAssets, MultiLocation};
use xcm_builder::{
    AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses,
    AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom,
    ChildParachainAsNative, ChildParachainConvertsVia, DescribeAllTerminal, DescribeFamily,
    FixedWeightBounds, FrameTransactionalProcessor, FungibleAdapter, HashedDescription,
    IsChildSystemParachain, IsConcrete, MintLocation, OriginToPluralityVoice,
    SignedAccountId32AsNative, SignedAccountKey20AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeWeightCredit, TrailingSetTopicAsId, UsingComponents,
    WeightInfoBounds, WithComputedOrigin, WithUniqueTopic, XcmFeeManagerFromComponents,
    XcmFeeToAccount,
};
use xcm_executor::{
    traits::{TransactAsset, WeightTrader},
    AssetsInHolding, XcmExecutor,
};

parameter_types! {
    pub const BaseXcmWeight: xcm::latest::Weight = Weight::from_parts(1_000, 1_000);
    pub const TokenLocation: Location = Here.into_location();
    pub const AnyNetwork: Option<NetworkId> = None;
    pub const RootLocation: Location = Location::here();
    pub const ThisNetwork: NetworkId = Westend;
    pub const UniversalLocation: InteriorLocation = xcm::latest::Junctions::Here;
    pub CheckAccount: AccountId = XcmPallet::check_account();
    pub LocalCheckAccount: (AccountId, MintLocation) = (CheckAccount::get(), MintLocation::Local);
    pub TreasuryAccount: AccountId = Treasury::account_id();
    /// The asset ID for the asset that we use to pay for message delivery fees.
    pub FeeAssetId: AssetId = AssetId(TokenLocation::get());
    /// The base fee for the message delivery fees.
    pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

pub type LocationConverter = (
    // We can convert a child parachain using the standard `AccountId` conversion.
    ChildParachainConvertsVia<ParaId, AccountId>,
    // We can directly alias an `AccountId32` into a local account.
    AccountId32Aliases<ThisNetwork, AccountId>,
    // Foreign locations alias into accounts according to a hash of their standard description.
    HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

pub type LocalAssetTransactor = FungibleAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<TokenLocation>,
    // We can convert the Locations with our converter above:
    LocationConverter,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // It's a native asset so we keep track of the teleports to maintain total issuance.
    LocalCheckAccount,
>;

pub type PriceForChildParachainDelivery =
    ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, Dmp>;

/// The XCM router. When we want to send an XCM message, we use this type. It amalgamates all of our
/// individual routers.
pub type XcmRouter = WithUniqueTopic<
    // Only one router so far - use DMP to communicate with child parachains.
    ChildParachainRouter<Runtime, XcmPallet, PriceForChildParachainDelivery>,
>;

parameter_types! {
    pub MaxInstructions: u32 = 100;
    pub MaxAssetsIntoHolding: u32 = 64;
}

pub struct OnlyParachains;
impl Contains<Location> for OnlyParachains {
    fn contains(location: &Location) -> bool {
        matches!(location.unpack(), (0, [Parachain(_)]))
    }
}

pub struct Fellows;
impl Contains<Location> for Fellows {
    fn contains(location: &Location) -> bool {
        matches!(
            location.unpack(),
            (0, [Parachain(COLLECTIVES_ID), Plurality { id: BodyId::Technical, .. }])
        )
    }
}

pub struct LocalPlurality;
impl Contains<Location> for LocalPlurality {
    fn contains(loc: &Location) -> bool {
        matches!(loc.unpack(), (0, [Plurality { .. }]))
    }
}

pub struct DoNothingRouter;
impl SendXcm for DoNothingRouter {
    type Ticket = ();
    fn validate(_dest: &mut Option<Location>, _msg: &mut Option<Xcm<()>>) -> SendResult<()> {
        Ok(((), Assets::new()))
    }
    fn deliver(_: ()) -> Result<XcmHash, SendError> {
        Ok([0; 32])
    }
}

/// The barriers one of which must be passed for an XCM message to be executed.
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct DummyAssetTransactor;
impl TransactAsset for DummyAssetTransactor {
    fn deposit_asset(_what: &Asset, _who: &Location, _context: Option<&XcmContext>) -> XcmResult {
        Ok(())
    }

    fn withdraw_asset(
        _what: &Asset,
        _who: &Location,
        _maybe_context: Option<&XcmContext>,
    ) -> Result<AssetsInHolding, XcmError> {
        let asset: Assets = (Parent, 100_000).into();
        Ok(asset.into())
    }
}

pub struct DummyWeightTrader;
impl WeightTrader for DummyWeightTrader {
    fn new() -> Self {
        DummyWeightTrader
    }

    fn buy_weight(
        &mut self,
        _weight: Weight,
        _payment: AssetsInHolding,
        _context: &XcmContext,
    ) -> Result<AssetsInHolding, XcmError> {
        Ok(AssetsInHolding::default())
    }
}

type OriginConverter = (
    pallet_xcm::XcmPassthrough<super::RuntimeOrigin>,
    SignedAccountKey20AsNative<AnyNetwork, super::RuntimeOrigin>,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = DoNothingRouter;
    type AssetTransactor = DummyAssetTransactor;
    type OriginConverter = OriginConverter;
    type IsReserve = ();
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<BaseXcmWeight, super::RuntimeCall, MaxInstructions>;
    type Trader = DummyWeightTrader;
    type ResponseHandler = XcmPallet;
    type AssetTrap = XcmPallet;
    type AssetLocker = ();
    type AssetExchanger = ();
    type AssetClaims = XcmPallet;
    type SubscriptionService = XcmPallet;
    type PalletInstancesInfo = ();
    type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = Everything;
    type Aliasers = Nothing;
    type TransactionalProcessor = FrameTransactionalProcessor;
    type HrmpNewChannelOpenRequestHandler = ();
    type HrmpChannelAcceptedHandler = ();
    type HrmpChannelClosingHandler = ();
}

/// location of this chain.
pub type LocalOriginToLocation = ();

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    // Note that this configuration of `SendXcmOrigin` is different from the one present in
    // production.
    type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = DoNothingRouter;
    // Anyone can execute XCM messages locally.
    type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = ();
    type MaxLockers = ConstU32<8>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
    type AdminOrigin = EnsureRoot<crate::AccountId>;
}
