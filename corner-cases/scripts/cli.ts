import fs from "fs";
import { Keypair, PublicKey } from "@solana/web3.js";
import { Command } from "commander";
import { createNfts } from "./create-nfts";

const program = new Command();

const parseDestinationPublicKey = (value) => new PublicKey(value);

const parseKeypair = (value, previousValue) => {
  console.info({ value });
  const keypairFile = fs.readFileSync(value, "utf-8");
  return Keypair.fromSecretKey(Buffer.from(JSON.parse(keypairFile)));
};

program
  .command("create <destination>")
  .description("Mints a bunch of corner-case NFTs into the destination address")
  .option(
    "-k, --keypair <path>",
    "Filepath or URL to a keypair. Only handles fees.",
    parseKeypair
  )
  .option("-u, --url <url>", "RPC URL", "https://devnet.genesysgo.net/")
  .action(async (destination, { keypair, url }) => {
    const destinationAddress = new PublicKey(destination);

    console.info(
      `Minting corner case NFTs into ${destinationAddress.toBase58()} via ${url}`
    );

    await createNfts(destinationAddress, { keypair, url });
  });

program.parse(process.argv);

console.info(program.opts());
