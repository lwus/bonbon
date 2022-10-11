import {
  bundlrStorage,
  keypairIdentity,
  Metaplex,
} from "@metaplex-foundation/js";
import type {
  CreateNftInput,
  UploadMetadataInput,
} from "@metaplex-foundation/js";
import { Connection } from "@solana/web3.js";

export const createNft = async (
  mx: Metaplex,
  input: Partial<CreateNftInput & { json: UploadMetadataInput }> = {}
) => {
  const { uri } = await mx
    .nfts()
    .uploadMetadata(input.json ?? {})
    .run();

  const { nft } = await mx
    .nfts()
    .create({
      uri,
      name: "My NFT",
      sellerFeeBasisPoints: 200,
      ...input,
    })
    .run();

  return nft;
};

export const createCollectionNft = (
  mx: Metaplex,
  input: Partial<CreateNftInput & { json: UploadMetadataInput }> = {}
) => createNft(mx, { ...input, isCollection: true });

export const initialize = ({ keypair, url }) => {
  const connection = new Connection(url, "finalized");
  const storage = url.includes("devnet")
    ? bundlrStorage({
        address: "https://devnet.bundlr.network",
      })
    : bundlrStorage();

  const metaplex = Metaplex.make(connection)
    .use(keypairIdentity(keypair))
    .use(storage);

  return { metaplex };
};
