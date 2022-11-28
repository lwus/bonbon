# Corner-cases

This CLI handles programmatic creation of "corner-case" NFTs that are useful in the context of validating a successful integration of Solana NFTs into a wallet / product.

## Commands

### `create`

Runs through all cases and mints NFTs accordingly.

```
 yarn tsx scripts/cli.ts create <destination-wallet-address> \
   --keypair <path-to-keypair-file> \
   --url https://ssc-dao.genesysgo.net/
```

Some cases are enumerated and can be run separately.

```
yarn tsx scripts/cli.ts case happyCase <destination-wallet-address> \
  --keypair <path-to-keypair-file> \
  --url https://ssc-dao.genesysgo.net/
```

### `dust`

Sends the minimal amount of SOL needed to facilitate the specified amount of transactions

```
yarn tsx scripts/cli.ts dust <destination-wallet-address> 5
```

### `clone`

Clones an NFT from `mainnet-beta` (except for any verification flags)

```
yarn tsx scripts/cli.ts clone <token-mint-address> <destination-wallet-address>
```
