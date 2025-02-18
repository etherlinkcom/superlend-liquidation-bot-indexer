use alloy::{
    network::Ethereum,
    primitives::{Address, Bytes},
    providers::Provider,
};
use anyhow::Result;

use crate::utils::contracts::{
    Multicall3::Call3,
    MulticallContract::{self, MulticallContractInstance},
};

const MULTICALL_ADDRESS: &str = "0xcA11bde05977b3631167028862bE2a173976CA11";

pub struct MulticallManager<P: Provider<Ethereum>> {
    multicall_contract: MulticallContractInstance<(), P>,
    calls: Vec<Call3>,
}

impl<P: Provider<Ethereum>> MulticallManager<P> {
    pub async fn new(provider: P) -> Result<Self> {
        let multicall =
            MulticallContract::new(MULTICALL_ADDRESS.parse::<Address>()?, provider);

        Ok(Self {
            multicall_contract: multicall,
            calls: vec![],
        })
    }

    pub fn add_call(&mut self, target: &Address, call_data: &Bytes) {
        self.calls.push(Call3 {
            target: *target,
            callData: call_data.clone(),
            allowFailure: true,
        });
    }

    // pub fn add_current_block_timestamp_call(&mut self) {
    //     self.calls.push(Call3 {
    //         target: MULTICALL_ADDRESS.parse::<Address>().unwrap(),
    //         callData: self
    //             .multicall_contract
    //             .getCurrentBlockTimestamp()
    //             .calldata()
    //             .clone(),
    //         allowFailure: true,
    //     });
    // }

    pub fn clear_calls(&mut self) {
        self.calls.clear();
    }

    pub fn get_calls(&self) -> &Vec<Call3> {
        &self.calls
    }

    pub async fn execute_calls(&self, block_number: u64) -> Result<Vec<Bytes>> {
        let multicall_result = self
            .multicall_contract
            .aggregate3(self.calls.clone())
            .block(block_number.into())
            .call()
            .await?;
        let mut results = vec![];

        for i in 0..multicall_result.returnData.len() {
            results.push(multicall_result.returnData[i].returnData.clone());
        }

        Ok(results)
    }
}
