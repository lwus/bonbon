import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";

export const dustAddress = async (
  destinationAddress: PublicKey,
  numberOfTransactions: number,
  { keypair, url }: { keypair: Keypair; url: string }
) => {
  const connection = new Connection(url, "confirmed");

  const currentLamports = await connection.getBalance(destinationAddress);
  const rentMinimum = await connection.getMinimumBalanceForRentExemption(0);

  const amount =
    (currentLamports <= rentMinimum ? rentMinimum : 0) +
    numberOfTransactions * 5000;

  if (currentLamports >= amount) {
    throw new Error("Destination already has SOL!");
  }

  const transaction = new Transaction().add(
    SystemProgram.transfer({
      fromPubkey: keypair.publicKey,
      toPubkey: destinationAddress,
      lamports: amount,
    })
  );

  await sendAndConfirmTransaction(connection, transaction, [keypair]);

  console.info(
    `Successfully dusted ${amount} into ${destinationAddress.toBase58()}`
  );
};
