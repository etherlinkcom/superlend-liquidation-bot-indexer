use alloy::sol;

// Aave Pool Contract
sol!(
    #[allow(missing_docs)]
    #[sol(rpc, extra_methods)]
    #[derive(Debug)]
    AavePoolContract,
    "abis/aave_pool.json"
);

// Aave Pool Data Provider Contract
sol!(
    #[allow(missing_docs)]
    #[sol(rpc, extra_methods)]
    #[derive(Debug)]
    AavePoolDataProviderContract,
    "abis/aave_pool_data_provider.json"
);

// --------- Multicall ---------
sol!(
    #[allow(missing_docs)]
    #[sol(rpc, extra_methods)]
    #[derive(Debug)]
    MulticallContract,
    "abis/multicall.json"
);
