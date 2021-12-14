use workspaces::prelude::*;
use workspaces::{Contract, DevNetwork, Worker};

// // helper that deploys a specific contract to mainnet
// pub async fn deploy(worker: Worker<impl Network>, contract_file: String) -> anyhow::Result<Contract> {
//     let file = std::fs::read(contract_file)?;
//     worker.deploy(file).await
// }

// helper that deploys a specific contract
// NOTE: `dev_deploy` is only available on `DevNetwork`s such sandbox and testnet.
pub async fn dev_deploy(
    worker: Worker<impl DevNetwork>,
    contract_file: &str,
) -> anyhow::Result<Contract> {
    let file = std::fs::read(contract_file)?;
    worker.dev_deploy(file).await
}
