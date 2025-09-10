import {
  AnchorProvider,
  BN,
  IdlAccounts,
  Program,
  Wallet,
} from "@coral-xyz/anchor";

import { CpAmm } from "./idl/damm_v2";
import CpAmmIDL from "../../idls/damm_v2.json";
import { clusterApiUrl, Connection, Keypair, PublicKey } from "@solana/web3.js";
import { LiteSVM } from "litesvm";
import {
  getAssociatedTokenAddressSync,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { sendTransactionOrExpectThrowError } from "./svm";

export const DAMM_V2_PROGRAM_ID = new PublicKey(CpAmmIDL.address);

export type Pool = IdlAccounts<CpAmm>["pool"];
export type Position = IdlAccounts<CpAmm>["position"];

const MIN_SQRT_PRICE = new BN("4295048016");
const MAX_SQRT_PRICE = new BN("79226673521066979257578248091");
const LIQUIDITY_DELTA = new BN("1844674407800459963300003758876517305");
const INIT_PRICE = new BN("18446744073709551616");

export function createDammV2Program() {
  const wallet = new Wallet(Keypair.generate());
  const provider = new AnchorProvider(
    new Connection(clusterApiUrl("devnet")),
    wallet,
    {}
  );
  const program = new Program<CpAmm>(CpAmmIDL as CpAmm, provider);
  return program;
}

export async function createDammV2Pool(
  svm: LiteSVM,
  creator: Keypair,
  tokenAMint: PublicKey,
  tokenBMint: PublicKey
): Promise<{
  pool: PublicKey;
  position: PublicKey;
  positionNftAccount: PublicKey;
}> {
  const program = createDammV2Program();

  const poolAuthority = deriveDammV2PoolAuthority();
  const pool = deriveDammV2CustomizablePoolAddress(tokenAMint, tokenBMint);

  const positionNftKP = Keypair.generate();
  const position = deriveDammV2PositionAddress(positionNftKP.publicKey);
  const positionNftAccount = deriveDammV2PositionNftAccount(
    positionNftKP.publicKey
  );

  const tokenAVault = deriveDammV2TokenVaultAddress(tokenAMint, pool);
  const tokenBVault = deriveDammV2TokenVaultAddress(tokenBMint, pool);

  const payerTokenA = getAssociatedTokenAddressSync(
    tokenAMint,
    creator.publicKey,
    true,
    TOKEN_PROGRAM_ID
  );
  const payerTokenB = getAssociatedTokenAddressSync(
    tokenBMint,
    creator.publicKey,
    true,
    TOKEN_PROGRAM_ID
  );

  const transaction = await program.methods
    .initializeCustomizablePool({
      poolFees: {
        baseFee: {
          cliffFeeNumerator: new BN(10_000_000),
          numberOfPeriod: 0,
          reductionFactor: new BN(0),
          periodFrequency: new BN(0),
          feeSchedulerMode: 0,
        },
        padding: new Array(3).fill(0),
        dynamicFee: null,
      },
      sqrtMinPrice: MIN_SQRT_PRICE,
      sqrtMaxPrice: MAX_SQRT_PRICE,
      hasAlphaVault: false,
      liquidity: LIQUIDITY_DELTA,
      sqrtPrice: INIT_PRICE,
      activationType: 0,
      collectFeeMode: 1, // collect fee mode: onlyB
      activationPoint: null,
    })
    .accountsPartial({
      creator: creator.publicKey,
      positionNftAccount,
      positionNftMint: positionNftKP.publicKey,
      payer: creator.publicKey,
      poolAuthority,
      pool,
      position,
      tokenAMint,
      tokenBMint,
      tokenAVault,
      tokenBVault,
      payerTokenA,
      payerTokenB,
      token2022Program: TOKEN_2022_PROGRAM_ID,
      tokenAProgram: TOKEN_PROGRAM_ID,
      tokenBProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();
  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.sign(creator, positionNftKP);

  sendTransactionOrExpectThrowError(svm, transaction);

  return { pool, position, positionNftAccount };
}

export function getProgramFromFlagDammV2(flag: number): PublicKey {
  return flag == 0 ? TOKEN_PROGRAM_ID : TOKEN_2022_PROGRAM_ID;
}

export function getDammV2PoolState(svm: LiteSVM, dammV2Pool: PublicKey): Pool {
  const program = createDammV2Program();
  const account = svm.getAccount(dammV2Pool);
  return program.coder.accounts.decode("pool", Buffer.from(account.data));
}

export function deriveDammV2PositionAddress(positionNft: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), positionNft.toBuffer()],
    DAMM_V2_PROGRAM_ID
  )[0];
}

export function deriveDammV2PositionNftAccount(
  positionNftMint: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position_nft_account"), positionNftMint.toBuffer()],
    DAMM_V2_PROGRAM_ID
  )[0];
}

export function deriveDammV2CustomizablePoolAddress(
  tokenAMint: PublicKey,
  tokenBMint: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("cpool"),
      getFirstKey(tokenAMint, tokenBMint),
      getSecondKey(tokenAMint, tokenBMint),
    ],
    DAMM_V2_PROGRAM_ID
  )[0];
}

export function deriveDammV2TokenVaultAddress(
  tokenMint: PublicKey,
  pool: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("token_vault"), tokenMint.toBuffer(), pool.toBuffer()],
    DAMM_V2_PROGRAM_ID
  )[0];
}

export function deriveDammV2EventAuthority() {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("__event_authority")],
    DAMM_V2_PROGRAM_ID
  )[0];
}

export function deriveDammV2PoolAuthority(): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("pool_authority")],
    DAMM_V2_PROGRAM_ID
  )[0];
}

function getSecondKey(key1: PublicKey, key2: PublicKey) {
  const buf1 = key1.toBuffer();
  const buf2 = key2.toBuffer();
  // Buf1 > buf2
  if (Buffer.compare(buf1, buf2) === 1) {
    return buf2;
  }
  return buf1;
}

function getFirstKey(key1: PublicKey, key2: PublicKey) {
  const buf1 = key1.toBuffer();
  const buf2 = key2.toBuffer();
  // Buf1 > buf2
  if (Buffer.compare(buf1, buf2) === 1) {
    return buf1;
  }
  return buf2;
}

export type SwapParams = {
  payer: Keypair;
  pool: PublicKey;
  inputTokenMint: PublicKey;
  outputTokenMint: PublicKey;
  amountIn: BN;
  minimumAmountOut: BN;
};

export async function dammV2Swap(svm: LiteSVM, params: SwapParams) {
  const {
    payer,
    pool,
    inputTokenMint,
    outputTokenMint,
    amountIn,
    minimumAmountOut,
  } = params;

  const program = createDammV2Program();

  const poolState = getDammV2PoolState(svm, pool);

  const tokenAProgram = svm.getAccount(poolState.tokenAMint).owner;

  const tokenBProgram = svm.getAccount(poolState.tokenBMint).owner;
  const inputTokenAccount = getAssociatedTokenAddressSync(
    inputTokenMint,
    payer.publicKey,
    true,
    tokenAProgram
  );
  const outputTokenAccount = getAssociatedTokenAddressSync(
    outputTokenMint,
    payer.publicKey,
    true,
    tokenBProgram
  );
  const tokenAVault = poolState.tokenAVault;
  const tokenBVault = poolState.tokenBVault;
  const tokenAMint = poolState.tokenAMint;
  const tokenBMint = poolState.tokenBMint;

  const transaction = await program.methods
    .swap({
      amountIn,
      minimumAmountOut,
    })
    .accountsPartial({
      poolAuthority: deriveDammV2PoolAuthority(),
      pool,
      payer: payer.publicKey,
      inputTokenAccount,
      outputTokenAccount,
      tokenAVault,
      tokenBVault,
      tokenAProgram,
      tokenBProgram,
      tokenAMint,
      tokenBMint,
      referralTokenAccount: null,
    })
    .transaction();

  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.sign(payer);

  sendTransactionOrExpectThrowError(svm, transaction);
}
