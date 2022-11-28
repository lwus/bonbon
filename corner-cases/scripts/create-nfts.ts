import fs from "fs";
import path from "path";
import { Keypair, PublicKey } from "@solana/web3.js";
import { createCollectionNft, createNft, initialize } from "./utils";
import { ownCreator } from "./cases/own-creator";

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

const collectionAddress = new PublicKey(
  "4xQvgQiN8aFeFp6bvmBd1zMTUMkPMxkgegwUeRDupHoQ"
);

export const createNfts = async (
  destination: PublicKey,
  { keypair, url }: { keypair: Keypair; url: string }
) => {
  const { metaplex } = initialize({ keypair, url });

  const nftPromises: Array<() => Promise<any>> = [];

  nftPromises.push(async () => {
    console.info("Creating NFT without a picture");
    const nftWithoutAPicture = await createNft(metaplex, {
      json: { name: "NFT without a picture" },
      name: "NFT without a picture",
      tokenOwner: destination,
    });
    console.info(
      `Created NFT without a picture: ${nftWithoutAPicture.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
    console.info("Creating NFT with unverified collection");
    const nftWithUnverifiedCollection = await createNft(metaplex, {
      json: {
        name: "NFT with unverified collection",
        image:
          "https://www.arweave.net/x3iWfSzm39cdAQ0e_-O9EX4ne-T3go752wjBXGmGea4?ext=png",
      },
      name: "NFT with unverified collection",
      collection: collectionAddress,
      tokenOwner: destination,
    });
    console.info(
      `Created NFT with unverified collection: ${nftWithUnverifiedCollection.address.toBase58()}`
    );

    console.info("Creating NFT with verified collection");
    const nftWithVerifiedCollection = await createNft(metaplex, {
      json: {
        name: "NFT with verified collection",
        image:
          "https://www.arweave.net/N9p7kt8EuHriN_B-teb_JH5WKWEsZ1gB9HbZGUTO6D8?ext=png",
      },
      name: "NFT with verified collection",
      collection: collectionAddress,
      collectionAuthority: keypair,
      tokenOwner: destination,
    });
    console.info(
      `Created NFT with verified collection: ${nftWithVerifiedCollection.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
    // Verified / unverified creators
    console.info("Creating NFT with one unverified creator");
    const nftWithOneUnverifiedCreator = await createNft(metaplex, {
      json: {
        name: "NFT with unverified creator (1)",
        image: "https://onlyhands.s3.amazonaws.com/assets/3235.png",
      },
      name: "NFT with unverified creator (1)",
      tokenOwner: destination,
      creators: [{ address: Keypair.generate().publicKey, share: 100 }],
    });
    console.info(
      `Created NFT one unverified creator: ${nftWithOneUnverifiedCreator.address.toBase58()}`
    );

    console.info("Creating NFT with one verified creator");
    const verifiedCreator = Keypair.generate();
    const nftWithOneVerifiedCreator = await createNft(metaplex, {
      json: {
        name: "NFT with verified creator (1)",
        image:
          "https://arweave.net/W-4HTmHtNVpS_Mhr1k0KNJpqz4qFaTv_APKpMQ1lq1Q?ext=jpeg",
      },
      name: "NFT with verified creator (1)",
      tokenOwner: destination,
      creators: [{ address: verifiedCreator.publicKey, share: 100 }],
    });
    await metaplex
      .nfts()
      .verifyCreator({
        mintAddress: nftWithOneVerifiedCreator.address,
        creator: verifiedCreator,
      })
      .run();
    console.info(
      `Created NFT one verified creator: ${nftWithOneVerifiedCreator.address.toBase58()}`
    );

    await ownCreator(destination, { keypair, url });
  });

  nftPromises.push(async () => {
    console.info("Creating NFT with Candy Machine creator");
    // Not _exactly_ correct, since a Candy Machine creator address will be a PDA.
    // This convention is not 1:1 with Candy Machine though, so this should be a good approximation.
    const candyMachineCreator = Keypair.generate();
    const nftWithCandyMachineCreator = await createNft(metaplex, {
      json: {
        name: "NFT with Candy Machine creator",
        image:
          "https://bafybeid3v3nmdttovnoppbeb4vsjtm7u7zzvuew7nmeuqkoqfo6vn57at4.ipfs.nftstorage.link/8695.png?ext=png",
      },
      name: "NFT with Candy Machine creator",
      tokenOwner: destination,
      creators: [
        { address: candyMachineCreator.publicKey, share: 0 },
        { address: Keypair.generate().publicKey, share: 90 },
        { address: Keypair.generate().publicKey, share: 10 },
      ],
    });
    await metaplex
      .nfts()
      .verifyCreator({
        mintAddress: nftWithCandyMachineCreator.address,
        creator: candyMachineCreator,
      })
      .run();
    console.info(
      `Created NFT with Candy Machine creator: ${nftWithCandyMachineCreator.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
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
    console.info(
      `Created NFT with missing name / URI: ${nftWithMissingName.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
    console.info("Creating NFT with incredibly long description");
    const nftWithLongDescription = await createNft(metaplex, {
      json: {
        name: "NFT with extra-long description",
        description: fs.readFileSync(
          path.join(__dirname, "./lorem-ipsum.txt"),
          "utf-8"
        ),
        image:
          "https://bafybeig7hq2rxjg4dj7geghwzab4xtkg53hderviyrbdvssxlsk2lqpwtu.ipfs.nftstorage.link/32.jpg?ext=jpg",
      },
      name: "NFT with extra-long description",
      tokenOwner: destination,
    });
    console.info(
      `Created NFT with incredibly long description: ${nftWithLongDescription.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
    console.info("Creating NFT with mismatched names");
    const nftWithMismatch = await createNft(metaplex, {
      json: {
        name: "This name shouldn't be displayed",
        image:
          "https://arweave.net/cj43BQW4DNCrwkY8XNauhmaeZMPy4hWNOivHYXyMJ1M",
      },
      name: "This name should be displayed",
      tokenOwner: destination,
    });
    console.info(
      `Created NFT with mismatched names: ${nftWithMismatch.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
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
    console.info(
      `Created NFT with GLB animation_url: ${nftWithGLBAnimationURL.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
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
    console.info(
      `Created NFT with GIF animation URL: ${nftWithGIFAnimationURL.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
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
    console.info(
      `Created NFT with mp4 animation URL: ${nftWithMP4AnimationURL.address.toBase58()}`
    );
  });

  nftPromises.push(async () => {
    console.info("Creating immutable NFT");
    const immutableNFT = await createNft(metaplex, {
      json: {
        name: "Immutable NFT",
        image:
          "https://bafybeiba2rva764svactzqkq6fvejtsryxhdbfpzdhxuzl5co3yk4yutgy.ipfs.nftstorage.link/363.png",
      },
      name: "Immutable NFT",
      isMutable: false,
      tokenOwner: destination,
    });
    console.info(`Created immutable NFT: ${immutableNFT.address.toBase58()}`);
  });

  nftPromises.push(async () => {
    console.info("Creating Master Edition NFT with 5 supply");
    const masterEditionNFT5Supply = await createNft(metaplex, {
      json: {
        name: "Master Edition NFT w/ 5 supply",
        image:
          "https://www.arweave.net/oOQ10CB1Ng2m3QycPYRio2qQkxM1U1oCFaHn7V4tbV4?ext=jpeg",
      },
      name: "Master Edition NFT w/ 5 supply",
      maxSupply: 5,
    });
    console.info(
      `Created Master Edition NFT with 5 supply: ${masterEditionNFT5Supply.address.toBase58()}`
    );
    const { nft: limitedEditionNFT } = await metaplex
      .nfts()
      .printNewEdition({
        originalMint: masterEditionNFT5Supply.address,
        newOwner: destination,
      })
      .run();
    console.info(
      `Created limited edition NFT: ${limitedEditionNFT.address.toBase58()}`
    );
  });

  const awaitPromises: Array<Promise<any>> = [];
  for (const nftPromise of nftPromises) {
    await sleep(1000);

    awaitPromises.push(nftPromise());
  }

  const results = await Promise.allSettled(awaitPromises);
  console.info({
    results: JSON.stringify(results, null, 2),
  });
};
