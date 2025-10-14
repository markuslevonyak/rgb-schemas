// RGB schemas
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2025 by
//     Stefano Pellegrini <stefano.pellegrini@bitfinex.com>
//
// Copyright (C) 2025 LNP/BP Standards Association. All rights reserved.
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

//! Inflatable Fungible Assets (IFA) schema.
//! (!) Not safe to use in a production environment!

use aluvm::isa::Instr;
use aluvm::library::{Lib, LibSite};
use amplify::confinement::Confined;
use rgbstd::contract::{
    AssignmentsFilter, ContractData, FungibleAllocation, IssuerWrapper, RightsAllocation,
    SchemaWrapper,
};
use rgbstd::persistence::{ContractStateRead, MemContract};
use rgbstd::schema::{
    AssignmentDetails, FungibleType, GenesisSchema, GlobalStateSchema, Occurrences,
    OwnedStateSchema, Schema, TransitionSchema,
};
use rgbstd::stl::{rgb_contract_stl, AssetSpec, ContractTerms, RejectListUrl, StandardTypes};
use rgbstd::validation::Scripts;
use rgbstd::vm::RgbIsa;
use rgbstd::{rgbasm, Amount, GlobalDetails, MetaDetails, SchemaId, TransitionDetails};
use strict_types::TypeSystem;

use crate::{
    ERRNO_INFLATION_EXCEEDS_ALLOWANCE, ERRNO_INFLATION_MISMATCH, ERRNO_ISSUED_MISMATCH,
    ERRNO_NON_EQUAL_IN_OUT, ERRNO_REPLACE_HIDDEN_BURN, ERRNO_REPLACE_NO_INPUT, GS_ISSUED_SUPPLY,
    GS_MAX_SUPPLY, GS_NOMINAL, GS_REJECT_LIST_URL, GS_TERMS, MS_ALLOWED_INFLATION, OS_ASSET,
    OS_INFLATION, OS_REPLACE, TS_BURN, TS_INFLATION, TS_REPLACE, TS_TRANSFER,
};

pub const IFA_SCHEMA_ID: SchemaId = SchemaId::from_array([
    0x82, 0x65, 0x7f, 0x89, 0x08, 0x2f, 0x06, 0x27, 0x64, 0xdc, 0x04, 0x7c, 0xbb, 0xff, 0xad, 0x94,
    0x2a, 0x82, 0x30, 0xc0, 0x41, 0xbc, 0xa3, 0x16, 0x43, 0x05, 0xba, 0x24, 0xc5, 0x95, 0xb4, 0x60,
]);

pub(crate) fn ifa_lib_genesis() -> Lib {
    #[allow(clippy::diverging_sub_expression)]
    let code = rgbasm! {
        // Set common offsets
        put     a8[1],0;
        put     a16[0],0;

        // Check reported issued supply against sum of asset allocations in output
        put     a8[0],ERRNO_ISSUED_MISMATCH;  // set errno
        ldg     GS_ISSUED_SUPPLY,a8[1],s16[0];  // read issued supply global state
        extr    s16[0],a64[0],a16[0];  // and store it in a64[0]
        sas     OS_ASSET;  // check sum of assets assignments in output equals a64[0]
        test;

        // Check that sum of inflation rights = max supply - issued supply
        put     a8[0],ERRNO_INFLATION_MISMATCH;  // set errno
        ldg     GS_MAX_SUPPLY,a8[1],s16[1];  // read max supply global state
        extr    s16[1],a64[1],a16[0];  // and store it in a64[1]
        sub.uc  a64[1],a64[0];  // issued supply is still in a64[0], result overwrites a64[0]
        test;  // fails if result is <0
        sas     OS_INFLATION;  // check sum of inflation rights in output equals a64[0]
        test;

        ret;
    };
    Lib::assemble::<Instr<RgbIsa<MemContract>>>(&code)
        .expect("wrong inflatable asset genesis valdiation script")
}

pub(crate) fn ifa_lib_transfer() -> Lib {
    let code = rgbasm! {
        // Checking that the sum of inputs is equal to the sum of outputs
        put     a8[0],ERRNO_NON_EQUAL_IN_OUT;  // set errno
        svs     OS_ASSET;  // verify sum
        test;  // check it didn't fail
        svs     OS_INFLATION;  // verify sum
        test;  // check it didn't fail

        // Replace rights validation
        cnp     OS_REPLACE,a16[0];  // count input replace rights
        cns     OS_REPLACE,a16[1];  // count output replace rights
        // Check if input count is 0
        put     a16[2],0;  // store 0 in a16[2]
        eq.n    a16[0],a16[2];  // check if input_count == 0
        // TODO: fix comment
        jif     40;  // jump to 0x28 if input_count == 0
        // Input count > 0, check that output count >= input count
        put     a8[0],ERRNO_REPLACE_HIDDEN_BURN;  // set errno
        lt.u    a16[1],a16[0];  // output_count < input_count
        inv     st0;  // output_count >= input_count
        test;  // fail if output_count < input_count
        ret;  // return execution flow
        // 0x28: Input count is 0, output count must also be 0
        put     a8[0],ERRNO_REPLACE_NO_INPUT;  // set errno
        eq.n    a16[1],a16[0];  // check if output_count == input_count
        test;  // fail if output_count != input_count (=0)
        ret;  // return execution flow
    };
    Lib::assemble::<Instr<RgbIsa<MemContract>>>(&code).expect("wrong transfer validation script")
}

pub(crate) fn ifa_lib_inflation() -> Lib {
    #[allow(clippy::diverging_sub_expression)]
    let code = rgbasm! {
        // Set common offsets
        put     a8[1],0;
        put     a16[0],0;

        // Check reported issued supply equals sum of asset allocations in output
        put     a8[0],ERRNO_ISSUED_MISMATCH;  // set errno
        ldg     GS_ISSUED_SUPPLY,a8[1],s16[0];  // read issued supply global state
        extr    s16[0],a64[0],a16[0];  // and store it in a64[0]
        sas     OS_ASSET;  // check sum of asset allocations in output equals issued_supply
        test;
        cpy     a64[0],a64[1];  // store issued supply in a64[1] for later

        // Check reported allowed inflation equals sum of inflation rights in output
        put     a8[0],ERRNO_INFLATION_MISMATCH;  // set errno
        ldm     MS_ALLOWED_INFLATION,s16[0];  // read allowed inflation global state
        extr    s16[0],a64[0],a16[0];  // and store it in a64[0]
        sas     OS_INFLATION;  // check sum of inflation rights in output equals a64[0]
        test;

        // Check that input inflation rights equals issued supply + allowed inflation
        put     a8[0],ERRNO_INFLATION_EXCEEDS_ALLOWANCE;
        add.uc  a64[1],a64[0];  // result is stored in a64[0]
        test;  // fails in case of an overflow
        sps     OS_INFLATION;  // check sum of inflation rights in input equals a64[0]
        test;

        ret;
    };
    Lib::assemble::<Instr<RgbIsa<MemContract>>>(&code).expect("wrong inflation validation script")
}

fn ifa_standard_types() -> StandardTypes { StandardTypes::with(rgb_contract_stl()) }

fn ifa_schema() -> Schema {
    let types = ifa_standard_types();

    let alu_id_transfer = ifa_lib_transfer().id();

    Schema {
        ffv: zero!(),
        name: tn!("InflatableFungibleAsset"),
        meta_types: tiny_bmap! {
            MS_ALLOWED_INFLATION => MetaDetails {
                sem_id: types.get("RGBContract.Amount"),
                name: fname!("allowedInflation"),
            }
        },
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
                global_state_schema: GlobalStateSchema::many(types.get("RGBContract.Amount")),
                name: fname!("issuedSupply"),
            },
            GS_MAX_SUPPLY => GlobalDetails {
                global_state_schema: GlobalStateSchema::once(types.get("RGBContract.Amount")),
                name: fname!("maxSupply"),
            },
            GS_REJECT_LIST_URL => GlobalDetails {
                global_state_schema: GlobalStateSchema::once(types.get("RGBContract.RejectListUrl")),
                name: fname!("rejectListUrl"),
            },
        },
        owned_types: tiny_bmap! {
            OS_ASSET => AssignmentDetails {
                owned_state_schema: OwnedStateSchema::Fungible(FungibleType::Unsigned64Bit),
                name: fname!("assetOwner"),
                default_transition: TS_TRANSFER,
            },
            OS_INFLATION => AssignmentDetails {
                owned_state_schema: OwnedStateSchema::Fungible(FungibleType::Unsigned64Bit),
                name: fname!("inflationAllowance"),
                default_transition: TS_TRANSFER
            },
            OS_REPLACE => AssignmentDetails {
                owned_state_schema: OwnedStateSchema::Declarative,
                name: fname!("replaceRight"),
                default_transition: TS_TRANSFER,
            }
        },
        genesis: GenesisSchema {
            metadata: none!(),
            globals: tiny_bmap! {
                GS_NOMINAL => Occurrences::Once,
                GS_TERMS => Occurrences::Once,
                GS_ISSUED_SUPPLY => Occurrences::Once,
                GS_MAX_SUPPLY => Occurrences::Once,
                GS_REJECT_LIST_URL => Occurrences::NoneOrOnce,
            },
            assignments: tiny_bmap! {
                OS_ASSET => Occurrences::NoneOrMore,
                OS_INFLATION => Occurrences::NoneOrMore,
                OS_REPLACE => Occurrences::NoneOrMore,
            },
            validator: Some(LibSite::with(0, ifa_lib_genesis().id())),
        },
        transitions: tiny_bmap! {
            TS_TRANSFER => TransitionDetails {
                transition_schema: TransitionSchema {
                    metadata: none!(),
                    globals: none!(),
                    inputs: tiny_bmap! {
                        OS_ASSET => Occurrences::NoneOrMore,
                        OS_INFLATION => Occurrences::NoneOrMore,
                        OS_REPLACE => Occurrences::NoneOrMore
                    },
                    assignments: tiny_bmap! {
                        OS_ASSET => Occurrences::NoneOrMore,
                        OS_INFLATION => Occurrences::NoneOrMore,
                        OS_REPLACE => Occurrences::NoneOrMore
                    },
                    validator: Some(LibSite::with(0, alu_id_transfer))
                },
                name: fname!("transfer"),
            },
            TS_INFLATION => TransitionDetails {
                transition_schema: TransitionSchema {
                    metadata: tiny_bset![MS_ALLOWED_INFLATION],
                    globals: tiny_bmap! {
                        GS_ISSUED_SUPPLY => Occurrences::Once,
                    },
                    inputs: tiny_bmap! {
                        OS_INFLATION => Occurrences::OnceOrMore
                    },
                    assignments: tiny_bmap! {
                        OS_ASSET => Occurrences::OnceOrMore,
                        OS_INFLATION => Occurrences::NoneOrMore
                    },
                    validator: Some(LibSite::with(0, ifa_lib_inflation().id()))
                },
                name: fname!("inflate"),
            },
            TS_BURN => TransitionDetails {
                transition_schema: TransitionSchema {
                    metadata: none!(),
                    globals: none!(),
                    inputs: tiny_bmap! {
                        OS_ASSET => Occurrences::NoneOrMore,
                        OS_REPLACE => Occurrences::NoneOrMore,
                        OS_INFLATION => Occurrences::NoneOrMore,
                    },
                    assignments: none!(),
                    validator: None
                },
                name: fname!("burn"),
            },
            TS_REPLACE => TransitionDetails {
                transition_schema: TransitionSchema {
                    metadata: none!(),
                    globals: none!(),
                    inputs: tiny_bmap! {
                        OS_ASSET => Occurrences::OnceOrMore,
                        OS_REPLACE => Occurrences::OnceOrMore,
                    },
                    assignments: tiny_bmap! {
                        OS_ASSET => Occurrences::OnceOrMore,
                        OS_REPLACE => Occurrences::OnceOrMore,
                    },
                    validator: Some(LibSite::with(0, alu_id_transfer))
                },
                name: fname!("replace"),
            },
        },
        default_assignment: Some(OS_ASSET),
    }
}

#[derive(Default)]
pub struct InflatableFungibleAsset;

impl IssuerWrapper for InflatableFungibleAsset {
    type Wrapper<S: ContractStateRead> = IfaWrapper<S>;

    fn schema() -> Schema { ifa_schema() }

    fn types() -> TypeSystem { ifa_standard_types().type_system(ifa_schema()) }

    fn scripts() -> Scripts {
        let alu_lib_genesis = ifa_lib_genesis();
        let alu_id_genesis = alu_lib_genesis.id();

        let alu_lib_transfer = ifa_lib_transfer();
        let alu_id_transfer = alu_lib_transfer.id();

        let alu_lib_inflation = ifa_lib_inflation();
        let alu_id_inflation = alu_lib_inflation.id();

        Confined::from_checked(bmap! {
            alu_id_genesis => alu_lib_genesis,
            alu_id_transfer => alu_lib_transfer,
            alu_id_inflation => alu_lib_inflation,
        })
    }
}
#[derive(Clone, Eq, PartialEq, Debug, From)]
pub struct IfaWrapper<S: ContractStateRead>(ContractData<S>);

impl<S: ContractStateRead> SchemaWrapper<S> for IfaWrapper<S> {
    fn with(data: ContractData<S>) -> Self {
        if data.schema.schema_id() != IFA_SCHEMA_ID {
            panic!("the provided schema is not IFA");
        }
        Self(data)
    }
}

impl<S: ContractStateRead> IfaWrapper<S> {
    pub fn spec(&self) -> AssetSpec {
        let strict_val = &self
            .0
            .global("spec")
            .next()
            .expect("IFA requires global state `spec` to have at least one item");
        AssetSpec::from_strict_val_unchecked(strict_val)
    }

    pub fn contract_terms(&self) -> ContractTerms {
        let strict_val = &self
            .0
            .global("terms")
            .next()
            .expect("IFA requires global state `terms` to have at least one item");
        ContractTerms::from_strict_val_unchecked(strict_val)
    }

    pub fn reject_list_url(&self) -> Option<RejectListUrl> {
        self.0
            .global("rejectListUrl")
            .next()
            .map(|strict_val| RejectListUrl::from_strict_val_unchecked(&strict_val))
    }

    fn issued_supply(&self) -> impl Iterator<Item = Amount> + '_ {
        self.0
            .global("issuedSupply")
            .map(|amount| Amount::from_strict_val_unchecked(&amount))
    }

    pub fn total_issued_supply(&self) -> Amount { self.issued_supply().sum() }

    pub fn issuance_amounts(&self) -> Vec<Amount> { self.issued_supply().collect::<Vec<_>>() }

    pub fn max_supply(&self) -> Amount {
        self.0
            .global("maxSupply")
            .map(|amount| Amount::from_strict_val_unchecked(&amount))
            .sum()
    }

    pub fn allocations<'c>(
        &'c self,
        filter: impl AssignmentsFilter + 'c,
    ) -> impl Iterator<Item = FungibleAllocation> + 'c {
        self.0.fungible_raw(OS_ASSET, filter).unwrap()
    }

    pub fn inflation_allocations<'c>(
        &'c self,
        filter: impl AssignmentsFilter + 'c,
    ) -> impl Iterator<Item = FungibleAllocation> + 'c {
        self.0.fungible_raw(OS_INFLATION, filter).unwrap()
    }

    pub fn replace_rights<'c>(
        &'c self,
        filter: impl AssignmentsFilter + 'c,
    ) -> impl Iterator<Item = RightsAllocation> + 'c {
        self.0.rights_raw(OS_REPLACE, filter).unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::ifa::ifa_schema;
    use crate::IFA_SCHEMA_ID;

    #[test]
    fn schema_id() {
        let schema_id = ifa_schema().schema_id();
        eprintln!("{:#04x?}", schema_id.to_byte_array());
        assert_eq!(IFA_SCHEMA_ID, schema_id);
    }
}
