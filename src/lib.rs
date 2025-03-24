// RGB schemata by LNP/BP Standards Association
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2023-2024 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2023-2024 LNP/BP Standards Association. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate strict_types;

mod cfa;
mod nia;
mod uda;

pub use cfa::{CfaWrapper, CollectibleFungibleAsset, CFA_SCHEMA_ID};
pub use nia::{NiaWrapper, NonInflatableAsset, NIA_SCHEMA_ID};
use rgbstd::{AssignmentType, GlobalStateType, TransitionType};
pub use uda::{UdaWrapper, UniqueDigitalAsset, UDA_SCHEMA_ID};

pub const LNPBP_IDENTITY: &str = "ssi:LZS1ux-gjD9nXPF-OcetUUkW-6r3uSCS6-aQhs9W5f-8JE7w";

pub const GS_ART: GlobalStateType = GlobalStateType::with(3000);
pub const GS_ATTACH: GlobalStateType = GlobalStateType::with(2104);
pub const GS_DETAILS: GlobalStateType = GlobalStateType::with(3004);
pub const GS_ENGRAVINGS: GlobalStateType = GlobalStateType::with(2103);
pub const GS_ISSUED_SUPPLY: GlobalStateType = GlobalStateType::with(2010);
pub const GS_NAME: GlobalStateType = GlobalStateType::with(3001);
pub const GS_NOMINAL: GlobalStateType = GlobalStateType::with(2000);
pub const GS_PRECISION: GlobalStateType = GlobalStateType::with(3005);
pub const GS_TERMS: GlobalStateType = GlobalStateType::with(2001);
pub const GS_TOKENS: GlobalStateType = GlobalStateType::with(2102);

pub const OS_ASSET: AssignmentType = AssignmentType::with(4000);

pub const TS_TRANSFER: TransitionType = TransitionType::with(10000);

pub const ERRNO_NON_EQUAL_IN_OUT: u8 = 0;
pub const ERRNO_ISSUED_MISMATCH: u8 = 1;
pub const ERRNO_NON_FRACTIONAL: u8 = 10;

pub mod dumb {
    use bp::Tx;
    use rgbstd::validation::{ResolveWitness, WitnessResolverError};
    use rgbstd::vm::WitnessOrd;
    use rgbstd::{ChainNet, Txid};

    pub struct NoResolver;

    impl ResolveWitness for NoResolver {
        fn resolve_pub_witness(&self, _: Txid) -> Result<Tx, WitnessResolverError> {
            unreachable!()
        }

        fn resolve_pub_witness_ord(&self, _: Txid) -> Result<WitnessOrd, WitnessResolverError> {
            unreachable!()
        }

        fn check_chain_net(&self, _: ChainNet) -> Result<(), WitnessResolverError> {
            unreachable!()
        }
    }
}
