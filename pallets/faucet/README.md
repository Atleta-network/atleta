Faucet Pallet
=============

This pallet implements a straightforward faucet mechanism. It allows users to request funds exclusively for their own accounts.

The origin should be signed. 

Users are limited to requesting up to the `Config::FaucetAmount` within a `Config::AccumulationPeriod` period.