import { Metaplex } from "@metaplex-foundation/js";
import { clusterApiUrl, Connection } from "@solana/web3.js";
import { initialize } from "./utils";

// Attempt to clone from mainnet
const mainnetConnection = new Connection(clusterApiUrl("mainnet-beta"));
const mainnetMetaplex = new Metaplex(mainnetConnection);

/*
 * Does a best effort job to clone an NFT. This cannot clone the `verification` status, however.
 */
export const cloneNft = async (
  sourceMintAddress,
  destinationAddress,
  { keypair, url }
) => {
  const sourceNft = await mainnetMetaplex
    .nfts()
    .findByMint({ mintAddress: sourceMintAddress })
    .run();

  const { metaplex } = initialize({ keypair, url });
  const { nft } = await metaplex
    .nfts()
    .create({
      ...sourceNft,
      collection: sourceNft.collection?.address,
      tokenOwner: destinationAddress,
    })
    .run();

  console.info(
    `Successfully cloned ${sourceMintAddress} into ${nft.address.toBase58()}`
  );
};
