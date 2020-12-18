// This file is part of Substrate.

// Copyright (C) 2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Primitive traits for providing election functionality.
//!
//! This crate provides two traits that could potentially be implemented by two entities. The struct
//! receiving the functionality election should implement [`ElectionDataProvider`] and the struct
//! providing the election functionality implements [`ElectionProvider`], as such:
//!
//! ```ignore
//!                                         ElectionDataProvider
//!                          <------------------------------------------+
//!                          |                                          |
//!                          v                                          |
//!                    +-----+----+                              +------+---+
//!                    |          |                              |          |
//! pallet-do-election |          |                              |          |pallet-elect-winner
//!                    |          |                              |          |
//!                    |          |                              |          |
//!                    +-----+----+                              +------+---+
//!                          |                                          ^
//!                          |                                          |
//!                          +------------------------------------------+
//!                                         ElectionProvider
//!
//! ```
//!
//! > It could also be possible that a third party pallet (C), provides the data of election to an
//! > election provider (B), which then passes the election result to another pallet (A).
//!
//! Note that the [`ElectionProvider`] does not have a hard tie to the [`ElectionDataProvider`],
//! rather the link must be created by other means during implementation (i.e. an associated type in
//! `Config` trait the case of FRAME pallets).
//!
//! ## Election Types
//!
//! Typically, two types of elections exist:
//!
//! 1. Stateless: Election data is provided, and the election result is immediately ready.
//! 2. Stateful: Election data is is provided, and the election result might be ready some number of
//!    blocks in the future.
//!
//! To accommodate both type of elections, the traits lean toward stateless election, as it is more
//! general than the stateless. This translates to the [`ElectionProvider::elect`] to have no
//! parameters. All value and type parameter must be provided by the [`ElectionDataProvider`] trait.
//!
//! ## Election Data
//!
//! The data associated with an election, essentially what the [`ElectionDataProvider`] must convey
//! is as follows:
//!
//! 1. A list of voters, with their stake.
//! 2. A list of targets (i.e. _candidates_).
//! 3. A number of desired targets to be elected (i.e. _winners_)
//! 4. An accuracy for the election's fixed point arithmetic.
//!
//! In addition to that, the [`ElectionDataProvider`] must also hint [`ElectionProvider`] at when
//! the next election might happen ([`ElectionDataProvider::next_election_prediction`]).
//! Nonetheless, an [`ElectionProvider`] shan't rely on this and should preferably provide some
//! means of fallback election as well.
//!
//! ## Example
//!
//! ```rust
//! # use sp_election_providers::*;
//! # use sp_npos_elections::Support;
//!
//! type AccountId = u64;
//!	type Balance = u64;
//!	type BlockNumber = u32;
//!
//! mod data_provider {
//! 	use super::*;
//!
//! 	pub trait Config {
//! 		type AccountId;
//! 		type ElectionProvider: ElectionProvider<Self::AccountId>;
//! 	}
//!
//!		pub struct Module<T: Config>(std::marker::PhantomData<T>);
//!
//!		impl<T: Config> ElectionDataProvider<AccountId, BlockNumber> for Module<T> {
//!			fn desired_targets() -> u32 {
//!				1
//!			}
//!			fn voters() -> Vec<(AccountId, VoteWeight, Vec<AccountId>)> {
//!				Default::default()
//!			}
//!			fn targets() -> Vec<AccountId> {
//!				vec![10, 20, 30]
//!			}
//!			fn feasibility_check_assignment<P: PerThing>(
//!				who: &AccountId,
//!				distribution: &[(AccountId, P)],
//!			) -> bool {
//!				true
//!			}
//!			fn next_election_prediction(now: BlockNumber) -> BlockNumber {
//!				0
//!			}
//!		}
//! }
//!
//!
//! mod election_provider {
//! 	use super::*;
//!
//! 	pub struct SomeElectionProvider<T: Config>(std::marker::PhantomData<T>);
//!
//! 	pub trait Config {
//! 		type DataProvider: ElectionDataProvider<AccountId, BlockNumber>;
//! 	}
//!
//! 	impl<T: Config> ElectionProvider<AccountId> for SomeElectionProvider<T> {
//! 		type Error = ();
//!
//! 		fn elect<P: PerThing128>() -> Result<Supports<AccountId>, Self::Error> {
//! 			T::DataProvider::targets()
//! 				.first()
//! 				.map(|winner| vec![(*winner, Support::default())])
//! 				.ok_or(())
//! 		}
//! 		fn ongoing() -> bool {
//!				false
//!			}
//! 	}
//! }
//!
//! mod runtime {
//! 	use super::election_provider;
//! 	use super::data_provider;
//! 	use super::AccountId;
//!
//! 	struct Runtime;
//! 	impl election_provider::Config for Runtime {
//! 		type DataProvider = data_provider::Module<Runtime>;
//! 	}
//!
//! 	impl data_provider::Config for Runtime {
//! 		type AccountId = AccountId;
//! 		type ElectionProvider = election_provider::SomeElectionProvider<Runtime>;
//! 	}
//!
//! }
//!
//! # fn main() {}
//! ```
//!
//!
//! ### ['ElectionDataProvider']'s side

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

/// Re-export some type as they are used in the interface.
pub use sp_npos_elections::{CompactSolution, ExtendedBalance, PerThing128, Supports, VoteWeight};
pub use sp_arithmetic::PerThing;

/// Something that can provide the data to something else that implements [`ElectionProvider`].
///
/// The underlying purpose of this is to provide auxillary data to stateful election providers. For
/// example, multi-block election provider needs to know the voters/targets list well in advance and
/// before a call to [`ElectionProvider::elect`].
pub trait ElectionDataProvider<AccountId, BlockNumber> {
	/// All possible targets for the election, i.e. the candidates.
	fn targets() -> Vec<AccountId>;

	/// All possible voters for the election.
	///
	/// Note that if a notion of self-vote exists, it should be represented here.
	fn voters() -> Vec<(AccountId, VoteWeight, Vec<AccountId>)>;

	/// The number of targets to elect.
	fn desired_targets() -> u32;

	/// Check the feasibility of a single assignment for the underlying `ElectionProvider`. In other
	/// words, check if `who` having a weight distribution described as `distribution` is correct or
	/// not.
	///
	/// This might be called by the [`ElectionProvider`] upon processing election solutions.
	///
	/// Note that this is any feasibility check specific to `Self` that `ElectionProvider` is not
	/// aware of. Simple feasibility (such as "did voter X actually vote for Y") should be checked
	/// by `ElectionProvider` in any case.
	fn feasibility_check_assignment<P: PerThing>(
		who: &AccountId,
		distribution: &[(AccountId, P)],
	) -> bool;

	/// Provide a best effort prediction about when the next election is about to happen.
	///
	/// In essence, the implementor should predict with this function when it will trigger the
	/// [`ElectionDataProvider::elect`].
	fn next_election_prediction(now: BlockNumber) -> BlockNumber;
}

/// Something that can compute the result of an election and pass it back to the caller.
///
/// This trait only provides an interface to _request_ an election, i.e.
/// [`ElectionProvider::elect`]. That data required for the election need to be passed to the
/// implemented of this trait through some other way. One example of such is the
/// [`ElectionDataProvider`] traits.
pub trait ElectionProvider<AccountId> {
	/// The error type that is returned by the provider.
	type Error;

	/// Elect a new set of winners.
	///
	/// The result is returned in a target major format, namely as vector of  supports.
	///
	/// The implementation should, if possible, use the accuracy `P` to compute the election result.
	fn elect<P: PerThing128>() -> Result<Supports<AccountId>, Self::Error>;

	/// Returns true if an election is still ongoing.
	///
	/// This can be used by the call site to dynamically check of a stateful is still on-going or
	/// not.
	fn ongoing() -> bool;
}