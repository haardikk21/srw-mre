use fuels::{
    accounts::predicate::Predicate,
    prelude::*,
    programs::script_calls::ScriptCallHandler,
    types::{coin_type::*, input::*, output::Output, unresolved_bytes::UnresolvedBytes},
};

// Load abi from json
abigen!(Predicate(
    name = "SocialRecoveryPredicate",
    abi = "out/debug/social-recovery-wallet-mre-abi.json"
));

const ASSET_ONE: AssetId = AssetId::new([0u8; 32]);
const PREDICATE_BIN: &str = "out/debug/social-recovery-wallet-mre.bin";

async fn get_wallets() -> (Vec<WalletUnlocked>) {
    // Launch a local network and deploy the contract
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new_multiple_assets(
            2,
            vec![AssetConfig {
                id: ASSET_ONE,
                num_coins: 1,
                coin_amount: 1_000_000_000,
            }],
        ),
        None,
        None,
    )
    .await
    .unwrap();

    wallets
}

#[tokio::test]
async fn can_get_contract_id() {
    let wallets = get_wallets().await;
    let my_wallet = wallets[0].clone();

    let provider = my_wallet.provider().clone().unwrap();

    let configs =
        SocialRecoveryPredicateConfigurables::new().with_OWNER(my_wallet.address().into());
    let predicate = Predicate::load_from(PREDICATE_BIN)
        .unwrap()
        .with_configurables(configs);

    let transfer_tx = my_wallet
        .transfer(
            predicate.address().into(),
            10,
            ASSET_ONE,
            Default::default(),
        )
        .await
        .unwrap();

    // Now you have an instance of your contract you can use to test each function
    // my_wallet is a `WalletUnlocked` type
    // `ASSET_ONE` is an `AssetId` type

    // Self-transferring 1 token

    // This is the 'default' inputs and outputs of a simple asset transfer
    let amount = 1;
    let mut transfer_inputs = my_wallet
        .get_asset_inputs_for_amount(ASSET_ONE, amount)
        .await
        .unwrap();
    let mut transfer_outputs =
        my_wallet.get_asset_outputs_for_amount(my_wallet.address().into(), ASSET_ONE, amount);

    // Intercept it
    // First, we modify the inputs
    let mut new_transfer_inputs = vec![];
    for input in &transfer_inputs {
        match input {
            // For each ResourceSigned
            Input::ResourceSigned { resource } => match resource {
                CoinType::Coin(coin) => {
                    // if input is originally from my wallet,
                    // we replace it with an input from predicate
                    if coin.owner == my_wallet.address().into() {
                        // Get predicate coin resource for this asset
                        let predicate_coin = &provider
                            .get_spendable_resources(ResourceFilter {
                                from: predicate.address().clone(),
                                asset_id: coin.asset_id,
                                amount: amount,
                                ..Default::default()
                            })
                            .await
                            .unwrap();

                        // Create the ResourcePredicate and add it
                        let new_input = predicate_coin[0].clone();
                        let predicate_code_input = Input::ResourcePredicate {
                            resource: new_input.clone(),
                            code: predicate.code().clone(),
                            data: UnresolvedBytes::default(),
                        };
                        new_transfer_inputs.push(predicate_code_input);

                    // If input is originally not mine,
                    // keep it as-is
                    } else {
                        new_transfer_inputs.push(input.clone());
                    }
                }
                _ => panic!("Coin type not supported"),
            },
            _ => panic!("Input type not supported"),
        }
    }

    // Now, we modify the outputs as well
    let mut new_transfer_outputs = vec![];
    for output in &transfer_outputs {
        match output {
            Output::Coin {
                to: _,
                amount: _,
                asset_id: _,
            } => {
                // Leave output coins as-is
                new_transfer_outputs.push(output.clone());
            }
            Output::Change {
                to: _,
                amount,
                asset_id,
            } => {
                // Update all `Change` outputs to be `Change` back into predicate address
                let change_to_predicate = Output::Change {
                    to: predicate.address().clone().into(),
                    amount: amount.clone(),
                    asset_id: asset_id.clone(),
                };
                new_transfer_outputs.push(change_to_predicate);
            }
            _ => panic!("Output type not supported"),
        }
    }

    // Now, build the transaction
    let script_call = ScriptCallHandler::<WalletUnlocked, ()>::new(
        vec![],
        UnresolvedBytes::default(),
        my_wallet.clone(),
        provider.clone(),
        Default::default(),
    )
    .with_inputs(new_transfer_inputs)
    .with_outputs(new_transfer_outputs);

    // Finally, execute the script
    script_call
        .call()
        .await
        .expect("Failed to execute script call");
}
