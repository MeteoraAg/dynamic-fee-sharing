import { Keypair, PublicKey } from "@solana/web3.js";
import {
  createProgram,
  deriveFeeVaultAuthorityAddress,
  deriveFeeVaultPdaAddress,
  deriveTokenVaultAddress,
  InitializeFeeVaultParameters,
} from ".";
import { LiteSVM, TransactionMetadata } from "litesvm";
import {
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";
import {
  DAMM_V2_PROGRAM_ID,
  deriveDammV2EventAuthority,
  deriveDammV2PoolAuthority,
  getDammV2PoolState,
  getProgramFromFlagDammV2,
} from "./damm_v2";
import { sendTransactionOrExpectThrowError } from "./svm";

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

export async function claimDammV2Fee(
  svm: LiteSVM,
  owner: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  dammv2Pool: PublicKey,
  position: PublicKey,
  positionNftAccount: PublicKey
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
    .claimDammv2Fee()
    .accountsPartial({
      feeVault,
      owner: owner.publicKey,
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
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(owner);

  sendTransactionOrExpectThrowError(svm, tx, false);
}
