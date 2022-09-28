import { Metaplex } from "@metaplex-foundation/js";
import type {
  CreateNftInput,
  UploadMetadataInput,
} from "@metaplex-foundation/js";

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
