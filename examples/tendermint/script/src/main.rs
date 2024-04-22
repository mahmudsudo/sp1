use std::time::Duration;
use tendermint::block::signed_header::SignedHeader;
use tendermint::block::Commit;
use tendermint::block::Header;
use tendermint::validator::Set;
use tokio::runtime::Runtime;

use reqwest::Client;
use sp1_sdk::{utils, ProverClient, PublicValues, SP1Stdin};

use sha2::{Digest, Sha256};
use tendermint_light_client_verifier::options::Options;
use tendermint_light_client_verifier::types::LightBlock;
use tendermint_light_client_verifier::ProdVerifier;
use tendermint_light_client_verifier::Verdict;
use tendermint_light_client_verifier::Verifier;
use util::fetch_commit;

use crate::util::fetch_latest_commit;
use crate::util::fetch_light_block;
use crate::util::fetch_validators;

const TENDERMINT_ELF: &[u8] = include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");
mod util;

async fn get_light_blocks() -> (LightBlock, LightBlock) {
    // Uniquely identify a peer in the network.
    let peer_id: [u8; 20] = [
        0x72, 0x6b, 0xc8, 0xd2, 0x60, 0x38, 0x7c, 0xf5, 0x6e, 0xcf, 0xad, 0x3a, 0x6b, 0xf6, 0xfe,
        0xcd, 0x90, 0x3e, 0x18, 0xa2,
    ];
    const BASE_URL: &str = "https://celestia-mocha-rpc.publicnode.com:443";
    let client = Client::new();
    let url = format!("{}/commit", BASE_URL);
    let latest_commit = fetch_latest_commit(&client, &url).await.unwrap();
    let block: u64 = latest_commit.result.signed_header.header.height.into();
    println!("Latest block: {}", block);
    let light_block_1 = fetch_light_block(block - 20, peer_id, BASE_URL)
        .await
        .expect("Failed to generate light block 1");
    let light_block_2 = fetch_light_block(block, peer_id, BASE_URL)
        .await
        .expect("Failed to generate light block 2");
    (light_block_1, light_block_2)
}

async fn get_commit(block: u64) -> SignedHeader {
    const BASE_URL: &str = "https://rpc.celestia-mocha.com";
    let client = Client::new();
    let url = format!("{}/commit", BASE_URL);
    let latest_commit = fetch_commit(&client, &url, block).await.unwrap();
    latest_commit.result.signed_header
}

async fn get_header(block: u64) -> Header {
    const BASE_URL: &str = "https://rpc.celestia-mocha.com";
    let client = Client::new();
    let url = format!("{}/commit", BASE_URL);
    let latest_commit = fetch_commit(&client, &url, block).await.unwrap();
    latest_commit.result.signed_header.header
}

async fn get_validator_set(block: u64) -> Set {
    const BASE_URL: &str = "https://rpc.celestia-mocha.com";
    let client = Client::new();
    let res = fetch_validators(&client, &format!("{}/validators", BASE_URL), block)
        .await
        .unwrap();
    Set::new(res, None)
}

fn main() {
    // Generate proof.
    utils::setup_logger();

    // Use tokio runtime to get the light blocks.
    let rt = Runtime::new().unwrap();
    // let (light_block_1, light_block_2) = rt.block_on(async { get_light_blocks().await });
    let header = rt.block_on(async { get_header(500000).await });
    // let validator_set = rt.block_on(async { get_validator_set(500000).await });
    // let expected_verdict = verify_blocks(light_block_1.clone(), light_block_2.clone());
    // let serialized = postcard::to_vec(&commit).unwrap();

    let mut stdin = SP1Stdin::new();

    // let encoded = bincode::serialize(&validator_set).unwrap();
    // stdin.write_vec(encoded);
    stdin.write(&header);

    // stdin.write(&light_block_1);
    // stdin.write(&light_block_2);

    // let encoded_1 = serde_cbor::to_vec(&light_block_1).unwrap();
    // let encoded_2 = serde_cbor::to_vec(&light_block_2).unwrap();

    // stdin.write_vec(encoded_1);
    // stdin.write_vec(encoded_2);

    // TODO: normally we could just write the LightBlock, but bincode doesn't work with LightBlock.
    // The following code will panic.
    // let encoded: Vec<u8> = bincode::serialize(&light_block_1).unwrap();
    // let decoded: LightBlock = bincode::deserialize(&encoded[..]).unwrap();

    let client = ProverClient::new();
    let proof = ProverClient::execute(TENDERMINT_ELF, stdin).expect("executing failed");

    // // Verify proof.
    // client
    //     .verify(TENDERMINT_ELF, &proof)
    //     .expect("verification failed");

    // // Verify the public values
    // let mut pv_hasher = Sha256::new();
    // pv_hasher.update(light_block_1.signed_header.header.hash().as_bytes());
    // pv_hasher.update(light_block_2.signed_header.header.hash().as_bytes());
    // pv_hasher.update(&serde_cbor::to_vec(&expected_verdict).unwrap());
    // let expected_pv_digest: &[u8] = &pv_hasher.finalize();

    // let public_values_bytes = proof.proof.shard_proofs[0].public_values.clone();
    // let public_values = PublicValues::from_vec(public_values_bytes);
    // assert_eq!(
    //     public_values.commit_digest_bytes().as_slice(),
    //     expected_pv_digest
    // );

    // // Save proof.
    // proof
    //     .save("proof-with-pis.json")
    //     .expect("saving proof failed");

    // println!("successfully generated and verified proof for the program!")
}

fn verify_blocks(light_block_1: LightBlock, light_block_2: LightBlock) -> Verdict {
    let vp = ProdVerifier::default();
    let opt = Options {
        trust_threshold: Default::default(),
        trusting_period: Duration::from_secs(500),
        clock_drift: Default::default(),
    };
    let verify_time = light_block_2.time() + Duration::from_secs(20);
    vp.verify_update_header(
        light_block_2.as_untrusted_state(),
        light_block_1.as_trusted_state(),
        &opt,
        verify_time.unwrap(),
    )
}
