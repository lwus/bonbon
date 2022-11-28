import { PublicKey } from "@solana/web3.js";
import { createNft, createCollectionNft, initialize } from "../utils";

const collectionAddress = new PublicKey(
  "4xQvgQiN8aFeFp6bvmBd1zMTUMkPMxkgegwUeRDupHoQ"
);

/*
 * Does a best effort job to clone an NFT. This cannot clone the `verification` status, however.
 */
export const happyCase = async (
  destinationAddress: PublicKey,
  { keypair, url }
) => {
  const { metaplex } = initialize({ keypair, url });

  const happyCaseNFT = await createNft(metaplex, {
    json: {
      name: "Happy Case!",
      description:
        "As the Orca ecosystem has grown, new creatures have been sighted emerging from deep within. 10,000 unique Orcanauts are now roaming free! Just like our podmates, each one of these little explorers is unique… and it’s looking for a forever friend with whom to navigate the deep sea of DeFi.",
      image:
        "https://www.arweave.net/N9p7kt8EuHriN_B-teb_JH5WKWEsZ1gB9HbZGUTO6D8?ext=png",
    },
    name: "Happy Case!",
    collection: collectionAddress,
    collectionAuthority: keypair,
    tokenOwner: destinationAddress,
    creators: [
      {
        address: new PublicKey("7Yj6vvhdBV4FDkcpFAbpbGEFB8J1LCpxdgZ1FWeVuPhu"),
        share: 100,
      },
    ],
  });

  console.info(
    `Successfully created ${happyCaseNFT.address.toBase58()} in ${destinationAddress.toBase58()}`
  );
};
