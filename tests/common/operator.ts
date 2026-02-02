import {
  AnchorProvider,
  BN,
  IdlAccounts,
  IdlTypes,
  Program,
  Wallet,
} from "@coral-xyz/anchor";
import {
  FailedTransactionMetadata,
  LiteSVM,
  TransactionMetadata,
} from "litesvm";

import DynamicFeeSharingIDL from "../../target/idl/dynamic_fee_sharing.json";
import { DynamicFeeSharing } from "../../target/types/dynamic_fee_sharing";
import {
  createAssociatedTokenAccountInstruction,
  createCloseAccountInstruction,
  createInitializeMint2Instruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  MINT_SIZE,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  clusterApiUrl,
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { DynamicFeeSharingProgram } from ".";
import { expect } from "chai";

export function deriveOperatorAddress(
  whitelistedAddress: PublicKey,
  programId: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("operator"), whitelistedAddress.toBuffer()],
    programId
  )[0];
}

export enum OperatorPermission {
  UpdateUserShare, // 0
}

export function encodePermissions(permissions: OperatorPermission[]): BN {
  return permissions.reduce((acc, perm) => {
    return acc.or(new BN(1).shln(perm));
  }, new BN(0));
}

export async function createOperatorAccount(params: {
  svm: LiteSVM;
  program: DynamicFeeSharingProgram;
  feeVault: PublicKey;
  whitelistedUser: PublicKey;
  vaultOwner: Keypair;
  permissions: OperatorPermission[];
}) {
  const { svm, program, feeVault, whitelistedUser, vaultOwner, permissions } =
    params;
  const operator = deriveOperatorAddress(whitelistedUser, program.programId);
  const createOperatorTx = await program.methods
    .createOperatorAccount(encodePermissions(permissions))
    .accountsPartial({
      feeVault,
      operator,
      whitelistedAddress: whitelistedUser,
      owner: vaultOwner.publicKey,
    })
    .transaction();

  createOperatorTx.recentBlockhash = svm.latestBlockhash();
  createOperatorTx.sign(vaultOwner);
  const createOperatorRes = svm.sendTransaction(createOperatorTx);

  expect(createOperatorRes instanceof TransactionMetadata).to.be.true;

  return operator;
}
