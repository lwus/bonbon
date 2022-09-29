import fs from "fs";
import path from "path";
import {
  bundlrStorage,
  keypairIdentity,
  Metaplex,
} from "@metaplex-foundation/js";
// import { nftStorage } from "@metaplex-foundation/js-plugin-nft-storage";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { createCollectionNft, createNft } from "./utils";

const initialize = ({ keypair, url }) => {
  const connection = new Connection(url);
  const metaplex = Metaplex.make(connection)
    .use(keypairIdentity(keypair))
    .use(
      bundlrStorage({
        address: "https://devnet.bundlr.network",
      })
      // nftStorage({
      //   token:
      //     // TODO(jon): Parameterize this
      // })
    );
  return { metaplex };
};

export const createNfts = async (
  destination: PublicKey,
  { keypair, url }: { keypair: Keypair; url: string }
) => {
  const { metaplex } = initialize({ keypair, url });

  // Collection-related NFTs

  console.info("Creating collection NFT");
  const collectionNFT = await createCollectionNft(metaplex, {
    json: {
      name: "My first Collection NFT",
      description:
        "This is an NFT that represents an entire collection of NFTs!",
    },
    name: "My first Collection NFT",
  });
  console.info(`Created collection NFT: ${collectionNFT.address.toBase58()}`);

  console.info("Creating NFT with unverified collection");
  const nftWithUnverifiedCollection = await createNft(metaplex, {
    json: { name: "NFT with unverified collection" },
    name: "NFT with unverified collection",
    collection: collectionNFT.address,
    tokenOwner: destination,
  });
  console.info(
    `Created NFT: ${nftWithUnverifiedCollection.address.toBase58()}`
  );

  console.info("Creating NFT with verified collection");
  const nftWithVerifiedCollection = await createNft(metaplex, {
    json: { name: "NFT with verified collection" },
    name: "NFT with verified collection",
    collection: collectionNFT.address,
    tokenOwner: destination,
  });
  await metaplex.nfts().verifyCollection({
    collectionMintAddress: collectionNFT.address,
    mintAddress: nftWithVerifiedCollection.address,
  });
  console.info(`Created NFT: ${nftWithVerifiedCollection.address.toBase58()}`);

  // Verified / unverified creators
  console.info("Creating NFT with one unverified creator");
  const nftWithOneUnverifiedCreator = await createNft(metaplex, {
    json: { name: "NFT with unverified creator (1)" },
    name: "NFT with unverified creator (1)",
    tokenOwner: destination,
    creators: [{ address: Keypair.generate().publicKey, share: 100 }],
  });
  console.info(
    `Created NFT: ${nftWithOneUnverifiedCreator.address.toBase58()}`
  );

  console.info("Creating NFT with one verified creator");
  const verifiedCreator = Keypair.generate();
  const nftWithOneVerifiedCreator = await createNft(metaplex, {
    json: { name: "NFT with verified creator (1)" },
    name: "NFT with verified creator (1)",
    tokenOwner: destination,
    creators: [{ address: verifiedCreator.publicKey, share: 100 }],
  });
  await metaplex.nfts().verifyCreator({
    mintAddress: nftWithOneVerifiedCreator.address,
    creator: verifiedCreator,
  });
  console.info(`Created NFT: ${nftWithOneVerifiedCreator.address.toBase58()}`);

  console.info("Creating NFT with Candy Machine creator");
  // Not _exactly_ correct, since a Candy Machine creator address will be a PDA.
  // This convention is not 1:1 with Candy Machine though, so this should be a good approximation.
  const candyMachineCreator = Keypair.generate();
  const nftWithCandyMachineCreator = await createNft(metaplex, {
    json: { name: "NFT with Candy Machine creator" },
    name: "NFT with Candy Machine creator",
    tokenOwner: destination,
    creators: [
      { address: verifiedCreator.publicKey, share: 0 },
      { address: Keypair.generate().publicKey, share: 90 },
      { address: Keypair.generate().publicKey, share: 10 },
    ],
  });
  await metaplex.nfts().verifyCreator({
    mintAddress: nftWithCandyMachineCreator.address,
    creator: candyMachineCreator,
  });
  console.info(`Created NFT: ${nftWithCandyMachineCreator.address.toBase58()}`);

  // Missing on-chain fields like name, uri
  console.info("Creating NFT with missing name / URI");
  const { nft: nftWithMissingName } = await metaplex
    .nfts()
    .create({
      uri: "",
      name: "",
      sellerFeeBasisPoints: 200,
    })
    .run();
  console.info(`Created NFT: ${nftWithMissingName.address.toBase58()}`);

  console.info("Creating NFT with incredibly long description");
  const nftWithLongDescription = await createNft(metaplex, {
    json: {
      name: "NFT with extra-long description",
      description: fs.readFileSync(
        path.join(__dirname, "./lorem-ipsum.txt"),
        "utf-8"
      ),
    },
    name: "NFT with extra-long description",
    tokenOwner: destination,
  });
  console.info(`Created NFT: ${nftWithLongDescription.address.toBase58()}`);

  console.info("Creating NFT with mismatched names");
  const nftWithMismatch = await createNft(metaplex, {
    json: {
      name: "This name shouldn't be displayed",
    },
    name: "This name should be displayed",
    tokenOwner: destination,
  });
  console.info(`Created NFT: ${nftWithMismatch.address.toBase58()}`);

  /*
   * Various `animation_url` formats
   */
  console.info("Creating NFT with GLB animation_url");
  const nftWithGLBAnimationURL = await createNft(metaplex, {
    json: {
      name: "NFT with GLB animation",
      image:
        "https://www.arweave.net/WmHroGZbameA0uITaEzlCoOKIGAjmVpkBfvAy5mrcLI",
      animation_url:
        "https://www.arweave.net/GfyWp6ktfhcfs09oXQ-r1OaBPMi0fm4g4l-1jRhPrt8?ext=glb",
      properties: {
        category: "vr",
      },
    },
    name: "NFT with GLB animation",
    tokenOwner: destination,
  });
  console.info(`Created NFT: ${nftWithGLBAnimationURL.address.toBase58()}`);

  console.info("Creating NFT with GIF animation URL");
  const nftWithGIFAnimationURL = await createNft(metaplex, {
    json: {
      name: "NFT with GIF animation",
      image:
        "https://arweave.net/C1BVcCCZX5NXSK9Z5gv3qaivKVXI7AXGRk0kFoHAzWY?ext=gif",
      properties: {
        category: "image",
        files: [
          {
            type: "image/gif",
            uri: "https://arweave.net/C1BVcCCZX5NXSK9Z5gv3qaivKVXI7AXGRk0kFoHAzWY?ext=gif",
          },
        ],
      },
    },
    name: "NFT with GIF animation",
    tokenOwner: destination,
  });
  console.info(`Created NFT: ${nftWithGIFAnimationURL.address.toBase58()}`);

  console.info("Creating NFT with mp4 animation URL");
  const nftWithMP4AnimationURL = await createNft(metaplex, {
    json: {
      name: "NFT with mp4 animation",
      animation_url:
        "https://arweave.net/6tTaYxu5epcV0Yh50uKsGZ4IFPEZ2ZmFoRlfyD1xbts?ext=mp4",
      properties: {
        category: "video",
        files: [
          {
            type: "video/mp4",
            uri: "https://arweave.net/6tTaYxu5epcV0Yh50uKsGZ4IFPEZ2ZmFoRlfyD1xbts?ext=mp4",
          },
        ],
      },
    },
    name: "NFT with mp4 animation",
    tokenOwner: destination,
  });
  console.info(`Created NFT: ${nftWithMP4AnimationURL.address.toBase58()}`);

  console.info("Creating immutable NFT");
  const immutableNFT = await createNft(metaplex, {
    json: {
      name: "Immutable NFT",
    },
    name: "Immutable NFT",
    isMutable: false,
    tokenOwner: destination,
  });
  console.info(`Created NFT: ${immutableNFT.address.toBase58()}`);

  console.info("Creating Master Edition NFT with 5 supply");
  const masterEditionNFT5Supply = await createNft(metaplex, {
    json: {
      name: "Master Edition NFT w/ 5 supply",
    },
    name: "Master Edition NFT w/ 5 supply",
    maxSupply: 5,
  });
  console.info(`Created NFT: ${masterEditionNFT5Supply.address.toBase58()}`);
  const { nft: limitedEditionNFT } = await metaplex
    .nfts()
    .printNewEdition({
      originalMint: masterEditionNFT5Supply.address,
      newOwner: destination,
    })
    .run();
  console.info(`Created NFT: ${limitedEditionNFT.address.toBase58()}`);
};
