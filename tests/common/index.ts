import {
  AnchorProvider,
  BN,
  IdlAccounts,
  IdlTypes,
  Program,
  Wallet,
} from "@anchor-lang/core";
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
import { expect } from "chai";
import { getTokenBalance, sendTransactionOrExpectThrowError } from "./svm";

export type InitializeFeeVaultParameters =
  IdlTypes<DynamicFeeSharing>["initializeFeeVaultParameters"];
export type UserShare = IdlTypes<DynamicFeeSharing>["userShare"];

export type FeeVault = IdlAccounts<DynamicFeeSharing>["feeVault"];

export type DynamicFeeSharingProgram = Program<DynamicFeeSharing>;

export const TOKEN_DECIMALS = 9;
export const RAW_AMOUNT = 1_000_000_000 * 10 ** TOKEN_DECIMALS;
export const DYNAMIC_FEE_SHARING_PROGRAM_ID = new PublicKey(
  DynamicFeeSharingIDL.address,
);
export const U64_MAX = new BN("18446744073709551615");

export function createProgram(): DynamicFeeSharingProgram {
  const wallet = new Wallet(Keypair.generate());
  const provider = new AnchorProvider(
    new Connection(clusterApiUrl("devnet")),
    wallet,
    {},
  );
  const program = new Program<DynamicFeeSharing>(
    DynamicFeeSharingIDL as DynamicFeeSharing,
    provider,
  );
  return program;
}

export function getFeeVault(svm: LiteSVM, feeVault: PublicKey): FeeVault {
  const program = createProgram();
  const account = svm.getAccount(feeVault);
  return program.coder.accounts.decode("feeVault", Buffer.from(account.data));
}

export function deriveFeeVaultAuthorityAddress(): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fee_vault_authority")],
    program.programId,
  )[0];
}

export function deriveTokenVaultAddress(feeVault: PublicKey): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [Buffer.from("token_vault"), feeVault.toBuffer()],
    program.programId,
  )[0];
}

export function deriveFeeVaultPdaAddress(
  base: PublicKey,
  tokenMint: PublicKey,
): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fee_vault"), base.toBuffer(), tokenMint.toBuffer()],
    program.programId,
  )[0];
}

export function createToken(
  svm: LiteSVM,
  payer: Keypair,
  mintAuthority: PublicKey,
  freezeAuthority?: PublicKey,
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
    freezeAuthority,
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
  toWallet: PublicKey,
  amount?: number,
) {
  const destination = getOrCreateAtA(svm, payer, mint, toWallet);

  const mintIx = createMintToInstruction(
    mint,
    destination,
    mintAuthority.publicKey,
    amount ?? RAW_AMOUNT,
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
  tokenProgram = TOKEN_PROGRAM_ID,
): PublicKey {
  const ataKey = getAssociatedTokenAddressSync(mint, owner, true, tokenProgram);

  const account = svm.getAccount(ataKey);
  if (account === null) {
    const createAtaIx = createAssociatedTokenAccountInstruction(
      payer.publicKey,
      ataKey,
      owner,
      mint,
      tokenProgram,
    );
    let transaction = new Transaction();

    transaction.recentBlockhash = svm.latestBlockhash();
    transaction.add(createAtaIx);
    transaction.sign(payer);
    svm.sendTransaction(transaction);
  }

  return ataKey;
}

export const wrapSOLInstruction = (
  from: PublicKey,
  to: PublicKey,
  amount: bigint,
): TransactionInstruction[] => {
  return [
    SystemProgram.transfer({
      fromPubkey: from,
      toPubkey: to,
      lamports: amount,
    }),
    new TransactionInstruction({
      keys: [
        {
          pubkey: to,
          isSigner: false,
          isWritable: true,
        },
      ],
      data: Buffer.from(new Uint8Array([17])),
      programId: TOKEN_PROGRAM_ID,
    }),
  ];
};

export const unwrapSOLInstruction = (
  owner: PublicKey,
  allowOwnerOffCurve = true,
) => {
  const wSolATAAccount = getAssociatedTokenAddressSync(
    NATIVE_MINT,
    owner,
    allowOwnerOffCurve,
  );
  if (wSolATAAccount) {
    const closedWrappedSolInstruction = createCloseAccountInstruction(
      wSolATAAccount,
      owner,
      owner,
      [],
      TOKEN_PROGRAM_ID,
    );
    return closedWrappedSolInstruction;
  }
  return null;
};

export function generateUsers(svm: LiteSVM, numberOfUsers: number) {
  const res = [];
  for (let i = 0; i < numberOfUsers; i++) {
    const user = Keypair.generate();
    svm.airdrop(user.publicKey, BigInt(LAMPORTS_PER_SOL));
    res.push(user);
  }

  return res;
}

export function getProgramErrorCodeHexString(errorMessage: String) {
  const error = DynamicFeeSharingIDL.errors.find(
    (e) =>
      e.name.toLowerCase() === errorMessage.toLowerCase() ||
      e.msg.toLowerCase() === errorMessage.toLowerCase(),
  );

  if (!error) {
    throw new Error(
      `Unknown Dynamic Fee Sharing error message / name: ${errorMessage}`,
    );
  }

  return error.code;
}

export function expectThrowsErrorCode(
  response: TransactionMetadata | FailedTransactionMetadata,
  errorCode: number,
) {
  if (response instanceof FailedTransactionMetadata) {
    const message = response.err().toString();

    if (!message.toString().includes(errorCode.toString())) {
      throw new Error(
        `Unexpected error: ${message}. Expected error: ${errorCode}`,
      );
    }

    return;
  } else {
    throw new Error("Expected an error but didn't get one");
  }
}

export async function fundFee(params: {
  svm: LiteSVM;
  program: DynamicFeeSharingProgram;
  funder: Keypair;
  fundAmount: BN;
  feeVault: PublicKey;
  tokenMint: PublicKey;
}) {
  const { svm, program, funder, fundAmount, feeVault, tokenMint } = params;

  const fundTokenVault = getAssociatedTokenAddressSync(
    tokenMint,
    funder.publicKey,
  );
  const tokenVault = deriveTokenVaultAddress(feeVault);
  const beforeTokenBalance = getTokenBalance(svm, tokenVault);
  const beforeFeeVaultState = getFeeVault(svm, feeVault);

  const tx = await program.methods
    .fundFee(fundAmount)
    .accountsPartial({
      feeVault,
      tokenVault,
      tokenMint,
      fundTokenVault,
      funder: funder.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(funder);

  const res = sendTransactionOrExpectThrowError(svm, tx);
  expect(res instanceof TransactionMetadata).to.be.true;

  const afterTokenBalance = getTokenBalance(svm, tokenVault);
  const afterFeeVaultState = getFeeVault(svm, feeVault);
  expect(afterTokenBalance.sub(beforeTokenBalance).eq(fundAmount)).to.be.true;
  expect(
    afterFeeVaultState.totalFundedFee
      .sub(beforeFeeVaultState.totalFundedFee)
      .eq(fundAmount),
  ).to.be.true;
}

export async function updateUserShare(params: {
  svm: LiteSVM;
  program: DynamicFeeSharingProgram;
  feeVault: PublicKey;
  operator: Keypair;
  userIndex: number;
  share: number;
}) {
  const { svm, program, feeVault, operator, userIndex, share } = params;

  const tx = await program.methods
    .updateUserShare(userIndex, share)
    .accountsPartial({
      feeVault,
      signer: operator.publicKey,
    })
    .transaction();
  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(operator);

  const res = sendTransactionOrExpectThrowError(svm, tx);
  expect(res instanceof TransactionMetadata).to.be.true;

  const feeVaultState = getFeeVault(svm, feeVault);
  expect(feeVaultState.users[userIndex].share).eq(share);
}

export async function removeUser(params: {
  svm: LiteSVM;
  program: DynamicFeeSharingProgram;
  feeVault: PublicKey;
  signer: Keypair;
  userIndex: number;
}) {
  const { svm, program, feeVault, signer, userIndex } = params;

  const tx = await program.methods
    .removeUser(userIndex)
    .accountsPartial({
      feeVault,
      signer: signer.publicKey,
    })
    .transaction();
  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(signer);

  const beforeUsersCount = getFeeVault(svm, feeVault).users.filter(
    (x) => !x.address.equals(PublicKey.default),
  ).length;
  const res = sendTransactionOrExpectThrowError(svm, tx);
  const afterUsersCount = getFeeVault(svm, feeVault).users.filter(
    (x) => !x.address.equals(PublicKey.default),
  ).length;

  expect(res instanceof TransactionMetadata).to.be.true;
  expect(beforeUsersCount - afterUsersCount).eq(1);
}

export async function updateOperator(params: {
  svm: LiteSVM;
  program: DynamicFeeSharingProgram;
  feeVault: PublicKey;
  operator: PublicKey;
  vaultOwner: Keypair;
}) {
  const { svm, program, feeVault, operator, vaultOwner } = params;
  const updateOperatorTx = await program.methods
    .updateOperator()
    .accountsPartial({
      feeVault,
      operator,
      owner: vaultOwner.publicKey,
    })
    .transaction();

  updateOperatorTx.recentBlockhash = svm.latestBlockhash();
  updateOperatorTx.sign(vaultOwner);
  const createOperatorRes = svm.sendTransaction(updateOperatorTx);
  const operatorField = getFeeVault(svm, feeVault).operator;
  expect(createOperatorRes instanceof TransactionMetadata).to.be.true;
  expect(operatorField.equals(operator)).to.be.true;

  return operator;
}
