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

//! Permissioned Fungible Assets (PFA) schema.

use aluvm::isa::Instr;
use aluvm::library::{Lib, LibSite};
use amplify::confinement::Confined;
use rgbstd::contract::{
    AssignmentsFilter, ContractData, FungibleAllocation, IssuerWrapper, SchemaWrapper,
};
use rgbstd::persistence::{ContractStateRead, MemContract};
use rgbstd::schema::{
    AssignmentDetails, FungibleType, GenesisSchema, GlobalDetails, GlobalStateSchema, Occurrences,
    OwnedStateSchema, Schema, TransitionSchema,
};
use rgbstd::stl::{rgb_contract_stl, AssetSpec, ContractTerms, StandardTypes};
use rgbstd::validation::Scripts;
use rgbstd::vm::RgbIsa;
use rgbstd::{rgbasm, Amount, Identity, SchemaId, TransitionDetails};
use strict_types::TypeSystem;

use crate::{
    ERRNO_INVALID_SIGNATURE, ERRNO_ISSUED_MISMATCH, ERRNO_MISSING_PUBKEY, ERRNO_NON_EQUAL_IN_OUT,
    GS_ISSUED_SUPPLY, GS_NOMINAL, GS_PUBKEY, GS_TERMS, LNPBP_IDENTITY, OS_ASSET, TS_TRANSFER,
};

pub const PFA_SCHEMA_ID: SchemaId = SchemaId::from_array([
    0x9b, 0x66, 0x05, 0x4d, 0x03, 0xcf, 0xa0, 0xfb, 0x6b, 0x80, 0x3c, 0xea, 0x12, 0xd7, 0xfd, 0xa7,
    0x0f, 0xc1, 0x92, 0xf8, 0xa2, 0x6b, 0x6e, 0x86, 0xc2, 0x67, 0x75, 0x46, 0x3f, 0x19, 0x57, 0xf6,
]);

pub(crate) fn pfa_lib_transition() -> Lib {
    let code = rgbasm! {
        // Checking that the sum of inputs is equal to the sum of outputs
        put     a8[0],ERRNO_NON_EQUAL_IN_OUT;  // set errno
        svs     OS_ASSET;  // verify sum
        test;  // check it didn't fail

        // Check transition signature
        put     a8[0],ERRNO_MISSING_PUBKEY;  // set errno
        put     a32[0],0;  // set a32[0] to 0
        ldc     GS_PUBKEY,a32[0],s16[0];  // get global pubkey
        put     a8[0],ERRNO_INVALID_SIGNATURE;  // set errno
        vts     s16[0];  // verify signature
        test;  // check it didn't fail
        ret;  // return execution flow
    };
    Lib::assemble::<Instr<RgbIsa<MemContract>>>(&code).expect("wrong non-inflatable asset script")
}

pub(crate) fn pfa_lib_genesis() -> Lib {
    let code = rgbasm! {
        // Check genesis assignments amount against reported amount of issued assets present in the
        // global state
        put     a8[0],ERRNO_ISSUED_MISMATCH;  // set errno
        put     a8[1],0;  // set a8[1] to 0
        put     a16[0],0;  // set a16[0] to 0
        ldg     GS_ISSUED_SUPPLY,a8[1],s16[0];  // get global issued supply
        extr    s16[0],a64[0],a16[0];  // extract 64 bits from the beginning of s16[0] into a64[0]
        sas     OS_ASSET;  // verify sum of outputs against a64[0] value
        test;  // check it didn't fail
        ret;  // return execution flow
    };
    Lib::assemble::<Instr<RgbIsa<MemContract>>>(&code).expect("wrong non-inflatable asset script")
}

fn pfa_schema() -> Schema {
    let types = StandardTypes::with(rgb_contract_stl());

    let alu_lib_genesis = pfa_lib_genesis();
    let alu_id_genesis = alu_lib_genesis.id();

    let alu_lib_transition = pfa_lib_transition();
    let alu_id_transition = alu_lib_transition.id();

    Schema {
        ffv: zero!(),
        flags: none!(),
        name: tn!("PermissionedFungibleAsset"),
        timestamp: 1713343888,
        developer: Identity::from(LNPBP_IDENTITY),
        meta_types: none!(),
        global_types: tiny_bmap! {
            GS_NOMINAL => GlobalDetails {
                global_state_schema: GlobalStateSchema::once(types.get("RGBContract.AssetSpec")),
                name: fname!("spec"),
            },
            GS_TERMS => GlobalDetails {
                global_state_schema: GlobalStateSchema::once(types.get("RGBContract.ContractTerms")),
                name: fname!("terms"),
            },
            GS_ISSUED_SUPPLY => GlobalDetails {
                global_state_schema: GlobalStateSchema::once(types.get("RGBContract.Amount")),
                name: fname!("issuedSupply"),
            },
            GS_PUBKEY => GlobalDetails {
                global_state_schema: GlobalStateSchema::once(types.get("RGBContract.PublicKey")),
                name: fname!("pubkey"),
            },
        },
        owned_types: tiny_bmap! {
            OS_ASSET => AssignmentDetails {
                owned_state_schema: OwnedStateSchema::Fungible(FungibleType::Unsigned64Bit),
                name: fname!("assetOwner"),
                default_transition: TS_TRANSFER,
            }
        },
        genesis: GenesisSchema {
            metadata: none!(),
            globals: tiny_bmap! {
                GS_NOMINAL => Occurrences::Once,
                GS_TERMS => Occurrences::Once,
                GS_ISSUED_SUPPLY => Occurrences::Once,
                GS_PUBKEY => Occurrences::Once,
            },
            assignments: tiny_bmap! {
                OS_ASSET => Occurrences::OnceOrMore,
            },
            validator: Some(LibSite::with(0, alu_id_genesis)),
        },
        transitions: tiny_bmap! {
            TS_TRANSFER => TransitionDetails {
                transition_schema: TransitionSchema {
                    metadata: none!(),
                    globals: none!(),
                    inputs: tiny_bmap! {
                        OS_ASSET => Occurrences::OnceOrMore
                    },
                    assignments: tiny_bmap! {
                        OS_ASSET => Occurrences::OnceOrMore
                    },
                    validator: Some(LibSite::with(0, alu_id_transition))
                },
                name: fname!("transfer"),
            }
        },
        reserved: none!(),
    }
}

#[derive(Default)]
pub struct PermissionedFungibleAsset;

impl IssuerWrapper for PermissionedFungibleAsset {
    type Wrapper<S: ContractStateRead> = PfaWrapper<S>;

    fn schema() -> Schema { pfa_schema() }

    fn types() -> TypeSystem { StandardTypes::with(rgb_contract_stl()).type_system() }

    fn scripts() -> Scripts {
        let alu_lib_genesis = pfa_lib_genesis();
        let alu_id_genesis = alu_lib_genesis.id();

        let alu_lib_transition = pfa_lib_transition();
        let alu_id_transition = alu_lib_transition.id();

        Confined::from_checked(bmap! {
            alu_id_genesis => alu_lib_genesis,
            alu_id_transition => alu_lib_transition,
        })
    }
}

#[derive(Clone, Eq, PartialEq, Debug, From)]
pub struct PfaWrapper<S: ContractStateRead>(ContractData<S>);

impl<S: ContractStateRead> SchemaWrapper<S> for PfaWrapper<S> {
    fn with(data: ContractData<S>) -> Self {
        if data.schema.schema_id() != PFA_SCHEMA_ID {
            panic!("the provided schema is not PFA");
        }
        Self(data)
    }
}

impl<S: ContractStateRead> PfaWrapper<S> {
    pub fn spec(&self) -> AssetSpec {
        let strict_val = &self
            .0
            .global("spec")
            .next()
            .expect("PFA requires global state `spec` to have at least one item");
        AssetSpec::from_strict_val_unchecked(strict_val)
    }

    pub fn contract_terms(&self) -> ContractTerms {
        let strict_val = &self
            .0
            .global("terms")
            .next()
            .expect("PFA requires global state `terms` to have at least one item");
        ContractTerms::from_strict_val_unchecked(strict_val)
    }

    pub fn total_issued_supply(&self) -> Amount {
        self.0
            .global("issuedSupply")
            .map(|amount| Amount::from_strict_val_unchecked(&amount))
            .sum()
    }

    pub fn allocations<'c>(
        &'c self,
        filter: impl AssignmentsFilter + 'c,
    ) -> impl Iterator<Item = FungibleAllocation> + 'c {
        self.0.fungible_raw(OS_ASSET, filter).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn schema_id() {
        let schema_id = pfa_schema().schema_id();
        eprintln!("{:#04x?}", schema_id.to_byte_array());
        assert_eq!(PFA_SCHEMA_ID, schema_id);
    }
}
