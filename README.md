# CosmoDex

CosmoDex is a decentralized exchange (DEX) built on the Stellar. It enables users to launch tokens, create liquidity pools, add/remove liquidity, and swap tokens in a fully decentralized and permissionless manner.

## Features

- **Token Launching**: Deploy your own custom tokens with adjustable parameters (name, symbol, decimals, admin).
- **Liquidity Pools**: Create pools for any token pair, enabling decentralized trading and liquidity provision.
- **Add/Remove Liquidity**: Supply tokens to pools to earn LP tokens and a share of trading fees, or withdraw your liquidity at any time.
- **Token Swapping**: Instantly swap between any two tokens in a pool using the constant product AMM formula.
- **Factory Pattern**: Pools are deployed and managed via a factory contract, ensuring uniqueness and discoverability.

## Project Structure

```
cosmo-dex/
├── contracts/
│   ├── poolfactory/   # Factory contract for deploying and tracking pools
│   ├── pool/          # Liquidity pool contract (AMM logic, LP tokens)
│   └── token/         # Custom token contract (mint, transfer, burn, etc.)
├── Cargo.toml         # Workspace configuration
└── README.md
```

### Contracts Overview

#### 1. Token Contract

- Implements a standard token with mint, transfer, approve, burn, and admin management.
- Used for both user-launched tokens and LP tokens.

#### 2. Pool Contract

- Implements a constant product AMM (like Uniswap v2).
- Users can:
  - Add liquidity (must be proportional to current reserves).
  - Remove liquidity (burn LP tokens for underlying assets).
  - Swap tokens (with a 0.3% fee).
- Tracks reserves and issues LP tokens to liquidity providers.

#### 3. PoolFactory Contract

- Deploys new pools for unique token pairs.
- Stores and retrieves pool addresses for token pairs.
- Only allows one pool per token pair.

## How It Works

1. **Launch a Token**
   - Deploy the token contract with your desired parameters.
   - Mint tokens to users as needed.

2. **Create a Pool**
   - Use the PoolFactory to deploy a new pool for a token pair.
   - The pool contract is initialized with the two token addresses and LP token metadata.

3. **Add Liquidity**
   - Approve the pool contract to spend your tokens.
   - Call `add_liquidity` with the amounts to deposit.
   - Receive LP tokens representing your share.

4. **Swap Tokens**
   - Call `swap` on the pool contract, specifying the input token and amount.
   - Receive the output token, minus a small fee.

5. **Remove Liquidity**
   - Burn your LP tokens to withdraw your share of the pool's reserves.

## Development

### Prerequisites

- Rust toolchain
- Soroban CLI
- Stellar local testnet



## Security

- All critical actions (minting, admin changes) require proper authorization.
- Pool creation is unique per token pair to prevent duplicate pools.
- Arithmetic checks and assertions prevent common AMM vulnerabilities.

