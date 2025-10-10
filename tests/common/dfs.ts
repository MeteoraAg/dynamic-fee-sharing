import { Keypair, PublicKey } from "@solana/web3.js";
import {
  createProgram,
  deriveFeeVaultAuthorityAddress,
  deriveFeeVaultPdaAddress,
  deriveTokenVaultAddress,
  getOrCreateAtA,
  getProgramErrorCodeHexString,
  InitializeFeeVaultParameters,
} from ".";
import { LiteSVM } from "litesvm";
import {
  getAssociatedTokenAddressSync,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  DAMM_V2_PROGRAM_ID,
  deriveDammV2EventAuthority,
  deriveDammV2PoolAuthority,
  getDammV2PoolState,
  getProgramFromFlagDammV2,
} from "./damm_v2";
import { sendTransactionOrExpectThrowError } from "./svm";
import {
  DBC_PROGRAM_ID,
  deriveDbcEventAuthority,
  deriveDbcPoolAuthority,
  getVirtualConfigState,
  getVirtualPoolState,
} from "./dbc";

export async function createFeeVaultPda(
  svm: LiteSVM,
  admin: Keypair,
  vaultOwner: PublicKey,
  tokenMint: PublicKey,
  params: InitializeFeeVaultParameters
): Promise<{
  feeVault: PublicKey;
  tokenVault: PublicKey;
}> {
  const program = createProgram();
  const baseKp = Keypair.generate();
  const feeVault = deriveFeeVaultPdaAddress(baseKp.publicKey, tokenMint);
  const tokenVault = deriveTokenVaultAddress(feeVault);
  const feeVaultAuthority = deriveFeeVaultAuthorityAddress();
  const tx = await program.methods
    .initializeFeeVaultPda(params)
    .accountsPartial({
      feeVault,
      base: baseKp.publicKey,
      feeVaultAuthority,
      tokenVault,
      tokenMint,
      owner: vaultOwner,
      payer: admin.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(admin, baseKp);

  sendTransactionOrExpectThrowError(svm, tx, false);

  return { feeVault, tokenVault };
}

export async function claimDammV2FeeExpectThrowError(
  svm: LiteSVM,
  signer: Keypair,
  owner: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  dammv2Pool: PublicKey,
  position: PublicKey,
  positionNftAccount: PublicKey,
) {
  const program = createProgram();
  const dammV2PoolState = getDammV2PoolState(svm, dammv2Pool);

  const tokenAAccount = getAssociatedTokenAddressSync(
    dammV2PoolState.tokenAMint,
    owner.publicKey,
    true,
    getProgramFromFlagDammV2(dammV2PoolState.tokenAFlag)
  );

  const tx = await program.methods
    .fundingByClaimDammv2Fee()
    .accountsPartial({
      feeVault,
      pool: dammv2Pool,
      position,
      positionNftAccount,
      tokenAAccount,
      tokenBAccount: tokenVault,
      tokenAVault: dammV2PoolState.tokenAVault,
      tokenBVault: dammV2PoolState.tokenBVault,
      tokenAMint: dammV2PoolState.tokenAMint,
      tokenBMint: dammV2PoolState.tokenBMint,
      tokenAProgram: getProgramFromFlagDammV2(dammV2PoolState.tokenAFlag),
      tokenBProgram: getProgramFromFlagDammV2(dammV2PoolState.tokenBFlag),
      dammv2EventAuthority: deriveDammV2EventAuthority(),
      dammv2PoolAuthority: deriveDammV2PoolAuthority(),
      dammv2Program: DAMM_V2_PROGRAM_ID,
      signer: signer.publicKey
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(signer);

  const errorCode = getProgramErrorCodeHexString("InvalidSigner");

  sendTransactionOrExpectThrowError(svm, tx, true, errorCode);
}

export async function claimDammV2Fee(
  svm: LiteSVM,
  signer: Keypair,
  owner: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  dammv2Pool: PublicKey,
  position: PublicKey,
  positionNftAccount: PublicKey,
) {
  const program = createProgram();
  const dammV2PoolState = getDammV2PoolState(svm, dammv2Pool);

  const tokenAAccount = getAssociatedTokenAddressSync(
    dammV2PoolState.tokenAMint,
    owner.publicKey,
    true,
    getProgramFromFlagDammV2(dammV2PoolState.tokenAFlag)
  );

  const tx = await program.methods
    .fundingByClaimDammv2Fee()
    .accountsPartial({
      feeVault,
      pool: dammv2Pool,
      position,
      positionNftAccount,
      tokenAAccount,
      tokenBAccount: tokenVault,
      tokenAVault: dammV2PoolState.tokenAVault,
      tokenBVault: dammV2PoolState.tokenBVault,
      tokenAMint: dammV2PoolState.tokenAMint,
      tokenBMint: dammV2PoolState.tokenBMint,
      tokenAProgram: getProgramFromFlagDammV2(dammV2PoolState.tokenAFlag),
      tokenBProgram: getProgramFromFlagDammV2(dammV2PoolState.tokenBFlag),
      dammv2EventAuthority: deriveDammV2EventAuthority(),
      dammv2PoolAuthority: deriveDammV2PoolAuthority(),
      dammv2Program: DAMM_V2_PROGRAM_ID,
      signer: signer.publicKey
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(signer);

  const result = sendTransactionOrExpectThrowError(svm, tx, true);

  return result
}

export async function claimDbcCreatorTradingFee(
  svm: LiteSVM,
  signer: Keypair,
  creator: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  poolConfig: PublicKey,
  virtualPool: PublicKey
) {
  const program = createProgram();
  const virtualPoolState = getVirtualPoolState(svm, virtualPool);
  const poolConfigState = getVirtualConfigState(svm, poolConfig);

  const tokenAAccount = getOrCreateAtA(
    svm,
    creator,
    virtualPoolState.baseMint,
    creator.publicKey,
    TOKEN_2022_PROGRAM_ID
  );

  const tx = await program.methods
    .fundingByClaimDbcCreatorTradingFee()
    .accountsPartial({
      feeVault,
      config: poolConfig,
      pool: virtualPool,
      tokenAAccount,
      tokenBAccount: tokenVault,
      baseVault: virtualPoolState.baseVault,
      quoteVault: virtualPoolState.quoteVault,
      baseMint: virtualPoolState.baseMint,
      quoteMint: poolConfigState.quoteMint,
      tokenBaseProgram: TOKEN_2022_PROGRAM_ID,
      tokenQuoteProgram: TOKEN_PROGRAM_ID,
      dbcEventAuthority: deriveDbcEventAuthority(),
      dbcPoolAuthority: deriveDbcPoolAuthority(),
      dbcProgram: DBC_PROGRAM_ID,
      signer: signer.publicKey
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(signer);

  sendTransactionOrExpectThrowError(svm, tx);
}

export async function claimDbcTradingFee(
  svm: LiteSVM,
  signer: Keypair,
  feeClaimer: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  poolConfig: PublicKey,
  virtualPool: PublicKey
) {
  const program = createProgram();
  const virtualPoolState = getVirtualPoolState(svm, virtualPool);
  const poolConfigState = getVirtualConfigState(svm, poolConfig);

  const tokenAAccount = getOrCreateAtA(
    svm,
    feeClaimer,
    virtualPoolState.baseMint,
    feeClaimer.publicKey,
    TOKEN_2022_PROGRAM_ID
  );

  const tx = await program.methods
    .fundingByClaimDbcPartnerTradingFee()
    .accountsPartial({
      feeVault,
      config: poolConfig,
      pool: virtualPool,
      tokenAAccount,
      tokenBAccount: tokenVault,
      baseVault: virtualPoolState.baseVault,
      quoteVault: virtualPoolState.quoteVault,
      baseMint: virtualPoolState.baseMint,
      quoteMint: poolConfigState.quoteMint,
      tokenBaseProgram: TOKEN_2022_PROGRAM_ID,
      tokenQuoteProgram: TOKEN_PROGRAM_ID,
      dbcEventAuthority: deriveDbcEventAuthority(),
      dbcPoolAuthority: deriveDbcPoolAuthority(),
      dbcProgram: DBC_PROGRAM_ID,
      signer: signer.publicKey
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(signer);

  sendTransactionOrExpectThrowError(svm, tx);
}

export async function withdrawDbcCreatorSurplus(
  svm: LiteSVM,
  signer: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  poolConfig: PublicKey,
  virtualPool: PublicKey
) {
  const program = createProgram();
  const virtualPoolState = getVirtualPoolState(svm, virtualPool);
  const poolConfigState = getVirtualConfigState(svm, poolConfig);

  const tx = await program.methods
    .fundingByClaimDbcCreatorSurplus()
    .accountsPartial({
      feeVault,
      config: poolConfig,
      pool: virtualPool,
      tokenQuoteAccount: tokenVault,
      quoteVault: virtualPoolState.quoteVault,
      quoteMint: poolConfigState.quoteMint,
      tokenQuoteProgram: TOKEN_PROGRAM_ID,
      dbcEventAuthority: deriveDbcEventAuthority(),
      dbcPoolAuthority: deriveDbcPoolAuthority(),
      dbcProgram: DBC_PROGRAM_ID,
      signer: signer.publicKey
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(signer);

  sendTransactionOrExpectThrowError(svm, tx, true);
}

export async function withdrawDbcPartnerSurplus(
  svm: LiteSVM,
  signer: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  poolConfig: PublicKey,
  virtualPool: PublicKey
) {
  const program = createProgram();
  const virtualPoolState = getVirtualPoolState(svm, virtualPool);
  const poolConfigState = getVirtualConfigState(svm, poolConfig);

  const tx = await program.methods
    .fundingByClaimDbcPartnerSurplus()
    .accountsPartial({
      feeVault,
      config: poolConfig,
      pool: virtualPool,
      tokenQuoteAccount: tokenVault,
      quoteVault: virtualPoolState.quoteVault,
      quoteMint: poolConfigState.quoteMint,
      tokenQuoteProgram: TOKEN_PROGRAM_ID,
      dbcEventAuthority: deriveDbcEventAuthority(),
      dbcPoolAuthority: deriveDbcPoolAuthority(),
      dbcProgram: DBC_PROGRAM_ID,
      signer: signer.publicKey
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(signer);

  sendTransactionOrExpectThrowError(svm, tx, true);
}
