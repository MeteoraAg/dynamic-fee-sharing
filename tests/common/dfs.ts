import { AccountMeta, Keypair, PublicKey } from "@solana/web3.js";
import CpAmmIDL from "../../idls/damm_v2.json";
import DynamicBondingCurveIDL from "../../idls/dynamic_bonding_curve.json";
import {
  createProgram,
  deriveFeeVaultAuthorityAddress,
  deriveFeeVaultPdaAddress,
  deriveTokenVaultAddress,
  getOrCreateAtA,
  InitializeFeeVaultParameters,
  U64_MAX,
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

async function fundByClaimingFee(svm: LiteSVM, signer: Keypair, feeVault: PublicKey, tokenVault: PublicKey, remainingAccounts: AccountMeta[], payload: Buffer, sourceProgram: PublicKey) {
  const program = createProgram();

  const tx = await program.methods
    .fundByClaimingFee(payload)
    .accountsPartial({
      feeVault,
      tokenVault,
      signer: signer.publicKey,
      sourceProgram
    })
    .remainingAccounts(
      remainingAccounts
    )
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(signer);

  const result = sendTransactionOrExpectThrowError(svm, tx, true);

  return result
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

  const dammV2PoolState = getDammV2PoolState(svm, dammv2Pool);

  const tokenAAccount = getAssociatedTokenAddressSync(
    dammV2PoolState.tokenAMint,
    owner.publicKey,
    true,
    getProgramFromFlagDammV2(dammV2PoolState.tokenAFlag)
  );

  const remainingAccounts = [
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDammV2PoolAuthority(),
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: dammv2Pool,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: position,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenAAccount,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: dammV2PoolState.tokenAVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: dammV2PoolState.tokenBVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: dammV2PoolState.tokenAMint,
    },

    {
      isSigner: false,
      isWritable: true,
      pubkey: dammV2PoolState.tokenBMint,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: positionNftAccount,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: feeVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: getProgramFromFlagDammV2(dammV2PoolState.tokenAFlag),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: getProgramFromFlagDammV2(dammV2PoolState.tokenBFlag),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDammV2EventAuthority(),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: DAMM_V2_PROGRAM_ID,
    },
  ];

  const claimPositionFeeDisc = CpAmmIDL.instructions.find(instruction => instruction.name === "claim_position_fee").discriminator;
  const payload = Buffer.from(claimPositionFeeDisc)

  await fundByClaimingFee(svm, signer, feeVault, tokenVault, remainingAccounts, payload, DAMM_V2_PROGRAM_ID)

}

export async function claimDammV2Reward(
  svm: LiteSVM,
  signer: Keypair,
  owner: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  dammv2Pool: PublicKey,
  position: PublicKey,
  positionNftAccount: PublicKey,
  rewardIndex: number,
) {

  const dammV2PoolState = getDammV2PoolState(svm, dammv2Pool);

  const remainingAccounts = [
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDammV2PoolAuthority(),
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: dammv2Pool,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: position,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: dammV2PoolState.rewardInfos[rewardIndex].vault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: dammV2PoolState.rewardInfos[rewardIndex].mint,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: positionNftAccount,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: feeVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: getProgramFromFlagDammV2(dammV2PoolState.rewardInfos[rewardIndex].rewardTokenFlag),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDammV2EventAuthority(),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: DAMM_V2_PROGRAM_ID,
    },
  ];

  const claimDammV2RewardDisc = CpAmmIDL.instructions.find(instruction => instruction.name === "claim_reward").discriminator;
  const payload = Buffer.concat([Buffer.from(claimDammV2RewardDisc), Buffer.from([rewardIndex]), Buffer.from([1])])
  await fundByClaimingFee(svm, signer, feeVault, tokenVault, remainingAccounts, payload, DAMM_V2_PROGRAM_ID)

}

export async function claimDbcCreatorTradingFee(
  svm: LiteSVM,
  signer: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  poolConfig: PublicKey,
  virtualPool: PublicKey
) {
  const virtualPoolState = getVirtualPoolState(svm, virtualPool);
  const poolConfigState = getVirtualConfigState(svm, poolConfig);

  const tokenAAccount = getOrCreateAtA(
    svm,
    signer,
    virtualPoolState.baseMint,
    signer.publicKey,
    TOKEN_2022_PROGRAM_ID
  );

  const remainingAccounts = [
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDbcPoolAuthority(),
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPool,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenAAccount,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPoolState.baseVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPoolState.quoteVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: virtualPoolState.baseMint,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: poolConfigState.quoteMint,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: feeVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: TOKEN_2022_PROGRAM_ID,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: TOKEN_PROGRAM_ID,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDbcEventAuthority(),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: DBC_PROGRAM_ID,
    },
  ]
  const claimDbcCreatorTradingFeeDisc = DynamicBondingCurveIDL.instructions.find(instruction => instruction.name === "claim_creator_trading_fee").discriminator;
  const payload = Buffer.concat([Buffer.from(claimDbcCreatorTradingFeeDisc), U64_MAX.toBuffer(), U64_MAX.toBuffer()])
  await fundByClaimingFee(svm, signer, feeVault, tokenVault, remainingAccounts, payload, DBC_PROGRAM_ID);
}

export async function claimDbcPartnerTradingFee(
  svm: LiteSVM,
  signer: Keypair,
  feeClaimer: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  poolConfig: PublicKey,
  virtualPool: PublicKey
) {
  const virtualPoolState = getVirtualPoolState(svm, virtualPool);
  const poolConfigState = getVirtualConfigState(svm, poolConfig);

  const tokenAAccount = getOrCreateAtA(
    svm,
    feeClaimer,
    virtualPoolState.baseMint,
    feeClaimer.publicKey,
    TOKEN_2022_PROGRAM_ID
  );

  const remainingAccounts = [
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDbcPoolAuthority(),
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: poolConfig,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPool,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenAAccount,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenVault,
    },

    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPoolState.baseVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPoolState.quoteVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: virtualPoolState.baseMint,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: poolConfigState.quoteMint,
    },

    {
      isSigner: false,
      isWritable: true,
      pubkey: feeVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: TOKEN_2022_PROGRAM_ID,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: TOKEN_PROGRAM_ID,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDbcEventAuthority(),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: DBC_PROGRAM_ID,
    },
  ]
  const claimDbcPartnerTradingFeeDisc = DynamicBondingCurveIDL.instructions.find(instruction => instruction.name === "claim_trading_fee").discriminator;
  const payload = Buffer.concat([Buffer.from(claimDbcPartnerTradingFeeDisc), U64_MAX.toBuffer(), U64_MAX.toBuffer()])
  await fundByClaimingFee(svm, signer, feeVault, tokenVault, remainingAccounts, payload, DBC_PROGRAM_ID);
}

export async function withdrawDbcCreatorSurplus(
  svm: LiteSVM,
  signer: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  poolConfig: PublicKey,
  virtualPool: PublicKey
) {
  const virtualPoolState = getVirtualPoolState(svm, virtualPool);
  const poolConfigState = getVirtualConfigState(svm, poolConfig);

  const remainingAccounts = [
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDbcPoolAuthority(),
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: poolConfig,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPool,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPoolState.quoteVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: poolConfigState.quoteMint,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: feeVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: TOKEN_PROGRAM_ID,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDbcEventAuthority(),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: DBC_PROGRAM_ID,
    },
  ]
  const creatorWithdrawSurplusDisc = DynamicBondingCurveIDL.instructions.find(instruction => instruction.name === "creator_withdraw_surplus").discriminator;
  const payload = Buffer.from(creatorWithdrawSurplusDisc)
  await fundByClaimingFee(svm, signer, feeVault, tokenVault, remainingAccounts, payload, DBC_PROGRAM_ID);
}

export async function withdrawDbcPartnerSurplus(
  svm: LiteSVM,
  signer: Keypair,
  feeVault: PublicKey,
  tokenVault: PublicKey,
  poolConfig: PublicKey,
  virtualPool: PublicKey
) {
  const virtualPoolState = getVirtualPoolState(svm, virtualPool);
  const poolConfigState = getVirtualConfigState(svm, poolConfig);

  const remainingAccounts = [
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDbcPoolAuthority(),
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: poolConfig,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPool,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: tokenVault,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: virtualPoolState.quoteVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: poolConfigState.quoteMint,
    },
    {
      isSigner: false,
      isWritable: true,
      pubkey: feeVault,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: TOKEN_PROGRAM_ID,
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: deriveDbcEventAuthority(),
    },
    {
      isSigner: false,
      isWritable: false,
      pubkey: DBC_PROGRAM_ID,
    },
  ]
  const partnerWithdrawSurplusDisc = DynamicBondingCurveIDL.instructions.find(instruction => instruction.name === "partner_withdraw_surplus").discriminator;
  const payload = Buffer.from(partnerWithdrawSurplusDisc)
  await fundByClaimingFee(svm, signer, feeVault, tokenVault, remainingAccounts, payload, DBC_PROGRAM_ID);
}
