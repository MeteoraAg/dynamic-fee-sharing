import {
  AnchorProvider,
  BN,
  IdlAccounts,
  IdlTypes,
  Program,
  Wallet,
} from "@coral-xyz/anchor";
import { LiteSVM } from "litesvm";

import DynamicFeeSharingIDL from "../../target/idl/dynamic_fee_sharing.json";
import { DynamicFeeSharing } from "../../target/types/dynamic_fee_sharing";
import {
  createAssociatedTokenAccountInstruction,
  createInitializeMint2Instruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  clusterApiUrl,
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";

export type InitializeFeeVaultParameters =
  IdlTypes<DynamicFeeSharing>["initializeFeeVaultParameters"];
export type UserShare = IdlTypes<DynamicFeeSharing>["userShare"];

export type FeeVault = IdlAccounts<DynamicFeeSharing>["feeVault"];

export type DynamicFeeSharingProgram = Program<DynamicFeeSharing>;

export const TOKEN_DECIMALS = 9;
export const RAW_AMOUNT = 1_000_000_000 * 10 ** TOKEN_DECIMALS;

export function createProgram(): DynamicFeeSharingProgram {
  const wallet = new Wallet(Keypair.generate());
  const provider = new AnchorProvider(
    new Connection(clusterApiUrl("devnet")),
    wallet,
    {}
  );
  const program = new Program<DynamicFeeSharing>(
    DynamicFeeSharingIDL as DynamicFeeSharing,
    provider
  );
  return program;
}

export function getFeeVault(svm: LiteSVM, feeVault: PublicKey): FeeVault {
  const program = createProgram()
  const account = svm.getAccount(feeVault)
  return program.coder.accounts.decode("feeVault", Buffer.from(account.data))
}

export function deriveFeeVaultAuthorityAddress(): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fee_vault_authority")],
    program.programId
  )[0];
}

export function deriveTokenVaultAddress(feeVault: PublicKey): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [Buffer.from("token_vault"), feeVault.toBuffer()],
    program.programId
  )[0];
}


export function deriveFeeVaultPdaAddress(base: PublicKey, tokenMint: PublicKey): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fee_vault"), base.toBuffer(), tokenMint.toBuffer()],
    program.programId
  )[0];
}

export function createToken(
  svm: LiteSVM,
  payer: Keypair,
  mintAuthority: PublicKey,
  freezeAuthority?: PublicKey
): PublicKey {
  const mintKeypair = Keypair.generate();
  const rent = svm.getRent();
  const lamports = rent.minimumBalance(BigInt(MINT_SIZE));

  const createAccountIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: mintKeypair.publicKey,
    space: MINT_SIZE,
    lamports: Number(lamports.toString()),
    programId: TOKEN_PROGRAM_ID,
  });

  const initializeMintIx = createInitializeMint2Instruction(
    mintKeypair.publicKey,
    TOKEN_DECIMALS,
    mintAuthority,
    freezeAuthority
  );

  let transaction = new Transaction();
  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.add(createAccountIx, initializeMintIx);
  transaction.sign(payer, mintKeypair);

  svm.sendTransaction(transaction);

  return mintKeypair.publicKey;
}

export function mintToken(
  svm: LiteSVM,
  payer: Keypair,
  mint: PublicKey,
  mintAuthority: Keypair,
  toWallet: PublicKey
) {
  const destination = getOrCreateAtA(svm, payer, mint, toWallet);

  const mintIx = createMintToInstruction(
    mint,
    destination,
    mintAuthority.publicKey,
    RAW_AMOUNT
  );

  let transaction = new Transaction();
  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.add(mintIx);
  transaction.sign(payer, mintAuthority);

  svm.sendTransaction(transaction);
}

export function getOrCreateAtA(
  svm: LiteSVM,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey,
  tokenProgram = TOKEN_PROGRAM_ID
): PublicKey {
  const ataKey = getAssociatedTokenAddressSync(mint, owner, true, tokenProgram);

  const account = svm.getAccount(ataKey);
  if (account === null) {
    const createAtaIx = createAssociatedTokenAccountInstruction(
      payer.publicKey,
      ataKey,
      owner,
      mint,
      tokenProgram
    );
    let transaction = new Transaction();

    transaction.recentBlockhash = svm.latestBlockhash();
    transaction.add(createAtaIx);
    transaction.sign(payer);
    svm.sendTransaction(transaction);
  }

  return ataKey;
}
