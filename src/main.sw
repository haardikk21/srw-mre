predicate;

use std::constants::ZERO_B256;
use std::tx::*;
use std::inputs::*;
use std::ecr::*;
use std::b512::*;

configurable {
    OWNER: Address = Address::from(ZERO_B256),
}

fn main() -> bool {
    let inputs_count = input_count().as_u64();
    let tx_hash = tx_id();

    let mut i = 0;
    while i < inputs_count {
        match input_type(i) {
            // If there is a Coin Input
            Input::Coin(_) => {
                let predicate_length = input_predicate_length(i);
                // That is a ResourcePredicate (has predicate code)
                if predicate_length.is_some() && predicate_length.unwrap() > 0 {
                    // Get the witness index for this input
                    let witness_index = input_witness_index(i).unwrap();

                    // Get the witness data from this txn for this witness
                    let witness_data: B512 = tx_witness_data(witness_index.as_u64());

                    // Extract address of witness
                    // To my understanding, witness would be `my_wallet` based on
                    // the above test harness
                    let recovered_address = ec_recover_address(witness_data, tx_hash);

                    // If recovered address = OWNER
                    if recovered_address.is_ok() && recovered_address.unwrap() == OWNER {
                        // All good
                        return true;
                    }
                }
            },
            _ => (),
        };

        i += 1;
    }

    false
}
