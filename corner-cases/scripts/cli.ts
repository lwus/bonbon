import fs from "fs";
import { Keypair, PublicKey } from "@solana/web3.js";
import { Command } from "commander";
import { createNfts } from "./create-nfts";
import { cloneNft } from "./clone-nft";
import { dustAddress } from "./dust-address";
import cases from "./cases";

const program = new Command();

const parseKeypair = (value) => {
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

program
  .command("clone <mintAddress> <destination>")
  .description("Clones an NFT at <mintAddress> into <destination> wallet")
  .option(
    "-k, --keypair <path>",
    "Filepath or URL to a keypair. Only handles fees.",
    parseKeypair
  )
  .option("-u, --url <url>", "RPC URL", "https://devnet.genesysgo.net/")
  .action(async (mintAddress, destination, { keypair, url }) => {
    const sourceMintAddress = new PublicKey(mintAddress);
    const destinationAddress = new PublicKey(destination);

    console.info(
      `Cloning ${sourceMintAddress} into ${destinationAddress.toBase58()} via ${url}`
    );

    await cloneNft(sourceMintAddress, destinationAddress, { keypair, url });
  });

program
  .command("dust <destination> <numberOfTransactions>")
  .description(
    "Dusts an account at <destination> with enough SOL for <numberOfTransactions>"
  )
  .option(
    "-k, --keypair <path>",
    "Filepath or URL to a keypair. Only handles fees.",
    parseKeypair
  )
  .option("-u, --url <url>", "RPC URL", "https://devnet.genesysgo.net/")
  .action(async (destination, numberOfTransactions, { keypair, url }) => {
    const destinationAddress = new PublicKey(destination);

    console.info(
      `Dusting ${destinationAddress.toBase58()} with enough SOL for ${numberOfTransactions} transaction(s) via ${url}`
    );

    await dustAddress(destinationAddress, numberOfTransactions, {
      keypair,
      url,
    });
  });

program
  .command("case <caseId> <destination>")
  .description("Create a specific case and mint into <destination>")
  .option(
    "-k, --keypair <path>",
    "Filepath or URL to a keypair. Only handles fees.",
    parseKeypair
  )
  .option("-u, --url <url>", "RPC URL", "https://devnet.genesysgo.net/")
  .action(async (caseId, destination, { keypair, url }) => {
    const destinationAddress = new PublicKey(destination);

    const foundCase = cases[caseId];
    if (foundCase) {
      console.info(
        `Executing case ${caseId} for address ${destinationAddress.toBase58()}`
      );

      await foundCase(destinationAddress, { keypair, url });
    }
  });

program.parse(process.argv);
