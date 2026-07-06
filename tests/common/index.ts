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
import { readFileSync } from "fs";
import { join } from "path";
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

export type InitializeFeeVaultParameters =
  IdlTypes<DynamicFeeSharing>["initializeFeeVaultParameters"];
export type UserShare = IdlTypes<DynamicFeeSharing>["userShare"];

export type FeeVault = IdlAccounts<DynamicFeeSharing>["feeVault"];

export type DynamicFeeVault = IdlAccounts<DynamicFeeSharing>["dynamicFeeVault"];

export type CreateWhitelistedActionParameters =
  IdlTypes<DynamicFeeSharing>["createWhitelistedActionParameters"];

export type DynamicFeeSharingProgram = Program<DynamicFeeSharing>;

export function deriveWhitelistedActionAddress(
  sourceProgram: PublicKey,
  discriminator: number[] | Buffer
): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("whitelisted_action"),
      sourceProgram.toBuffer(),
      Buffer.from(discriminator),
    ],
    program.programId
  )[0];
}

// admin allowlisted under the `local` feature (see instructions/auth.rs); `pnpm test` builds with it
export function loadLocalnetAdmin(svm: LiteSVM): Keypair {
  const secret = JSON.parse(
    readFileSync(
      join(
        __dirname,
        "../../keys/localnet/admin-bossj3JvwiNK7pvjr149DqdtJxf2gdygbcmEPTkb2F1.json"
      ),
      "utf-8"
    )
  );
  const admin = Keypair.fromSecretKey(Uint8Array.from(secret));
  svm.airdrop(admin.publicKey, BigInt(LAMPORTS_PER_SOL));
  return admin;
}

export const TOKEN_DECIMALS = 9;
export const RAW_AMOUNT = 1_000_000_000 * 10 ** TOKEN_DECIMALS;
export const DYNAMIC_FEE_SHARING_PROGRAM_ID = new PublicKey(
  DynamicFeeSharingIDL.address
);
export const U64_MAX = new BN("18446744073709551615");

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
  const program = createProgram();
  const account = svm.getAccount(feeVault);
  return program.coder.accounts.decode("feeVault", Buffer.from(account.data));
}

// Decodes the fixed header of a DynamicFeeVault. The growable UserFee tail lives
// past the header and is read separately via getDynamicFeeVaultUsers.
export function getDynamicFeeVault(
  svm: LiteSVM,
  feeVault: PublicKey
): DynamicFeeVault {
  const program = createProgram();
  const account = svm.getAccount(feeVault);
  return program.coder.accounts.decode(
    "dynamicFeeVault",
    Buffer.from(account.data)
  );
}

// Layout: [8 discriminator][DynamicFeeVault header (320)][DynamicUserFee; N (128 each)].
const DYNAMIC_FEE_VAULT_TAIL_OFFSET = 8 + 320;
const USER_FEE_SIZE = 128;

export function getDynamicFeeVaultUsers(
  svm: LiteSVM,
  feeVault: PublicKey
): { address: PublicKey; share: number; feeClaimed: BN }[] {
  const account = svm.getAccount(feeVault);
  const data = Buffer.from(account.data);
  const users = [];
  for (
    let off = DYNAMIC_FEE_VAULT_TAIL_OFFSET;
    off + USER_FEE_SIZE <= data.length;
    off += USER_FEE_SIZE
  ) {
    users.push({
      address: new PublicKey(data.subarray(off, off + 32)),
      share: data.readUInt32LE(off + 32),
      // DynamicUserFee.fee_claimed_token_0: u64 at offset 40
      feeClaimed: new BN(data.subarray(off + 40, off + 48), "le"),
    });
  }
  return users;
}

// absent token1Mint = disabled slot 1, seeded as PublicKey.default (all zeros)
export function deriveDynamicFeeVaultPdaAddress(
  base: PublicKey,
  token0Mint: PublicKey,
  token1Mint?: PublicKey
): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("dynamic_fee_vault"),
      base.toBuffer(),
      token0Mint.toBuffer(),
      (token1Mint ?? PublicKey.default).toBuffer(),
    ],
    program.programId
  )[0];
}

export function deriveFeeVaultAuthorityAddress(): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fee_vault_authority")],
    program.programId
  )[0];
}

// slot-1 token vault is keyed by mint; slot 0 keeps [prefix, fee_vault] for FeeVault parity
export function deriveTokenVault1Address(
  feeVault: PublicKey,
  token1Mint: PublicKey
): PublicKey {
  const program = createProgram();
  return PublicKey.findProgramAddressSync(
    [Buffer.from("token_vault"), feeVault.toBuffer(), token1Mint.toBuffer()],
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

export function deriveFeeVaultPdaAddress(
  base: PublicKey,
  tokenMint: PublicKey
): PublicKey {
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
  toWallet: PublicKey,
  amount?: number
) {
  const destination = getOrCreateAtA(svm, payer, mint, toWallet);

  const mintIx = createMintToInstruction(
    mint,
    destination,
    mintAuthority.publicKey,
    amount ?? RAW_AMOUNT
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

export const wrapSOLInstruction = (
  from: PublicKey,
  to: PublicKey,
  amount: bigint
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
  allowOwnerOffCurve = true
) => {
  const wSolATAAccount = getAssociatedTokenAddressSync(
    NATIVE_MINT,
    owner,
    allowOwnerOffCurve
  );
  if (wSolATAAccount) {
    const closedWrappedSolInstruction = createCloseAccountInstruction(
      wSolATAAccount,
      owner,
      owner,
      [],
      TOKEN_PROGRAM_ID
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
      e.msg.toLowerCase() === errorMessage.toLowerCase()
  );

  if (!error) {
    throw new Error(
      `Unknown Dynamic Fee Sharing error message / name: ${errorMessage}`
    );
  }

  return error.code;
}

export function expectThrowsErrorCode(
  response: TransactionMetadata | FailedTransactionMetadata,
  errorCode: number
) {
  if (response instanceof FailedTransactionMetadata) {
    const message = response.err().toString();

    if (!message.toString().includes(errorCode.toString())) {
      throw new Error(
        `Unexpected error: ${message}. Expected error: ${errorCode}`
      );
    }

    return;
  } else {
    throw new Error("Expected an error but didn't get one");
  }
}
