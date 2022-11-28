import { PublicKey } from "@solana/web3.js";
import { createNft, initialize } from "../utils";

export const ownCreator = async (destination: PublicKey, { keypair, url }) => {
  const { metaplex } = initialize({ keypair, url });

  console.info("Creating NFT with one own unverified creator");
  const nftWithOneUnverifiedCreator = await createNft(metaplex, {
    json: {
      name: "NFT with own unverified creator",
      image:
        "https://www.arweave.net/b_6b9H7Ec_pANaz0394OQI7TRoL0_K1Knlrsk_039rc?ext=png",
    },
    name: "NFT with own unverified creator",
    tokenOwner: destination,
    creators: [{ address: destination, share: 100 }],
  });
  console.info(
    `Created NFT with one own unverified creator: ${nftWithOneUnverifiedCreator.address.toBase58()}`
  );

  console.info("Creating NFT with one own verified creator");
  const nftWithOneVerifiedCreator = await createNft(metaplex, {
    json: {
      name: "NFT with own verified creator",
      image:
        "https://www.arweave.net/b2Ufl38QpZWWE16a6d3q-ZldyeSDJAuJPgRBdh6vSkM?ext=jpeg",
    },
    name: "NFT with own verified creator",
    tokenOwner: destination,
    creators: [{ address: destination, share: 100 }],
  });
  console.info(
    `Created NFT with one own verified creator: ${nftWithOneVerifiedCreator.address.toBase58()}`
  );
};
