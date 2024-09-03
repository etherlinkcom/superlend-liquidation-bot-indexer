use futures::stream::Stream;
use std::sync::Arc;
use std::time::Duration;

use crate::BaseRpcClient;

pub struct BlockWatcher {
    rpc_client: Arc<BaseRpcClient>,
}

impl BlockWatcher {
    pub fn new(rpc_client: BaseRpcClient) -> Self {
        Self {
            rpc_client: Arc::new(rpc_client),
        }
    }

    pub async fn watch_blocks(
        &self,
    ) -> impl Stream<Item = Result<u64, Box<dyn std::error::Error>>> {
        let rpc_client = self.rpc_client.clone();
        let mut current_block = rpc_client.get_block_number().await.unwrap();
        let interval = Duration::from_secs(2);
        async_stream::stream! {
            loop {
                let block = rpc_client.get_block_number().await.unwrap();
                if block > current_block {
                    current_block = block;
                    yield Ok(current_block);
                }
                std::thread::sleep(interval);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::pin_mut;
    use futures::TryStreamExt;
    use tracing::info;

    use super::*;

    #[tokio::test]
    async fn test_get_block_number() {
        tracing_subscriber::fmt::init();
        let rpc_client = BaseRpcClient::new("https://node.ghostnet.etherlink.com", 5);

        let block_watcher = BlockWatcher::new(rpc_client);
        let stream = block_watcher.watch_blocks().await;
        pin_mut!(stream);
        let mut count = 0;

        loop {
            let result = stream.try_next().await;
            match result {
                Ok(block) => {
                    info!("Block: {:?}", block);
                    count += 1;
                    if count >= 10 {
                        break;
                    }
                }
                Err(e) => {
                    info!("Error: {:?}", e);
                }
            }
        }
    }
}
