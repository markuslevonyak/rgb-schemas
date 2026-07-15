use std::str::FromStr;

use rgbstd::containers::{FileContent, Kit};
use rgbstd::persistence::Stock;
use rgbstd::stl::{ContractTerms, RicardianContract};
use rgbstd::txout::BlindSeal;
use rgbstd::{GenesisSeal, Txid};

/// Fixed issuance timestamp so example output is reproducible.
pub const CREATED_AT: i64 = 1713261744;

pub const BENEFICIARY_TXID: &str =
    "14295d5bb1a191cdb6286dc0944df938421e3dfcbf0811353ccac4100c2068c5";

#[allow(dead_code)]
pub fn default_terms() -> ContractTerms {
    ContractTerms {
        text: RicardianContract::default(),
        media: None,
    }
}

pub fn stock_with_kit(schema_rgb: &str) -> Stock {
    let mut stock = Stock::in_memory();
    let kit = Kit::load_file(schema_rgb).unwrap().validate().unwrap();
    stock.import_kit(kit).expect("invalid issuer kit");
    stock
}

pub fn genesis_seal(txid: &str, vout: u32, blinding: u64) -> GenesisSeal {
    GenesisSeal::from(BlindSeal::with_blinding(Txid::from_str(txid).unwrap(), vout, blinding))
}
