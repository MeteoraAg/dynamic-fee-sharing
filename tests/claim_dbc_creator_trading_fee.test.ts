import { BN } from "bn.js";
import { Keypair, PublicKey } from "@solana/web3.js";
import { expect } from "chai";
import { LiteSVM } from "litesvm";
import { generateUsers, getTokenBalance, startSvm } from "./common/svm";
import {
  createToken,
  getFeeVault,
  mintToken,
} from "./common";
import {
  buildDefaultCurve,
  createConfig,
  CreateConfigParams,
  createVirtualPool,
  getVirtualConfigState,
  getVirtualPoolState,
  swap,
  SwapParams,
  transferCreator,
} from "./common/dbc";
import {
  claimDbcCreatorTradingFee,
  claimDbcPartnerTradingFee,
  createFeeVaultPda,
  withdrawDbcCreatorSurplus,
  withdrawDbcPartnerSurplus,
} from "./common/dfs";

describe("Funding by claiming in DBC", () => {
  let svm: LiteSVM;
  let admin: Keypair;
  let payer: Keypair;
  let user: Keypair;
  let poolCreator: Keypair;
  let vaultOwner: Keypair;
  let quoteMint: PublicKey;
  let shareHolder: Keypair;

  beforeEach(async () => {
    svm = startSvm();
    admin = Keypair.generate();
    payer = Keypair.generate();
    user = Keypair.generate();
    poolCreator = Keypair.generate();
    [admin, payer, user, poolCreator, vaultOwner, shareHolder] = generateUsers(svm, 6);
    quoteMint = createToken(svm, admin, admin.publicKey, null);
  });

  it("claim dbc creator trading fee", async () => {
    const { feeVault, tokenVault } = await createFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      quoteMint,
      {
        padding: [],
        users: [
          {
            address: shareHolder.publicKey,
            share: 100,
          },
          {
            address: PublicKey.unique(),
            share: 100,
          },
        ],
      }
    );

    const { virtualPool, virtualPoolConfig } = await setupPool(
      svm,
      admin,
      user,
      poolCreator,
      payer,
      feeVault,
      quoteMint
    );

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    await claimDbcCreatorTradingFee(
      svm,
      shareHolder,
      feeVault,
      tokenVault,
      virtualPoolConfig,
      virtualPool
    );

    const postTokenVaultBalance = getTokenBalance(svm, tokenVault);
    vaultState = getFeeVault(svm, feeVault);

    const postTotalFundedFee = vaultState.totalFundedFee;
    const postFeePerShare = vaultState.feePerShare;

    expect(postTotalFundedFee.sub(preTotalFundedFee).toString()).eq(
      postTokenVaultBalance.sub(preTokenVaultBalance).toString()
    );
    expect(Number(postFeePerShare.sub(preFeePerShare))).gt(0);
  });

  it("claim dbc partner trading fee", async () => {
    const { feeVault, tokenVault } = await createFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      quoteMint,
      {
        padding: [],
        users: [
          {
            address: shareHolder.publicKey,
            share: 100,
          },
          {
            address: PublicKey.unique(),
            share: 100,
          },
        ],
      }
    );

    const { virtualPool, virtualPoolConfig } = await setupPool(
      svm,
      admin,
      user,
      poolCreator,
      payer,
      feeVault,
      quoteMint
    );

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    await claimDbcPartnerTradingFee(
      svm,
      shareHolder,
      payer,
      feeVault,
      tokenVault,
      virtualPoolConfig,
      virtualPool
    );

    const postTokenVaultBalance = getTokenBalance(svm, tokenVault);
    vaultState = getFeeVault(svm, feeVault);

    const postTotalFundedFee = vaultState.totalFundedFee;
    const postFeePerShare = vaultState.feePerShare;

    expect(postTotalFundedFee.sub(preTotalFundedFee).toString()).eq(
      postTokenVaultBalance.sub(preTokenVaultBalance).toString()
    );
    expect(Number(postFeePerShare.sub(preFeePerShare))).gt(0);
  });

  it("withdraw dbc creator surplus", async () => {
    const { feeVault, tokenVault } = await createFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      quoteMint,
      {
        padding: [],
        users: [
          {
            address: shareHolder.publicKey,
            share: 100,
          },
          {
            address: PublicKey.unique(),
            share: 100,
          },
        ],
      }
    );

    const { virtualPool, virtualPoolConfig } = await setupPool(
      svm,
      admin,
      user,
      poolCreator,
      payer,
      feeVault,
      quoteMint
    );

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    await withdrawDbcCreatorSurplus(
      svm,
      shareHolder,
      feeVault,
      tokenVault,
      virtualPoolConfig,
      virtualPool
    );

    const postTokenVaultBalance = getTokenBalance(svm, tokenVault);
    vaultState = getFeeVault(svm, feeVault);

    const postTotalFundedFee = vaultState.totalFundedFee;
    const postFeePerShare = vaultState.feePerShare;

    expect(postTotalFundedFee.sub(preTotalFundedFee).toString()).eq(
      postTokenVaultBalance.sub(preTokenVaultBalance).toString()
    );
    expect(Number(postFeePerShare.sub(preFeePerShare))).gt(0);
  });

  it("withdraw dbc partner surplus", async () => {
    const { feeVault, tokenVault } = await createFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      quoteMint,
      {
        padding: [],
        users: [
          {
            address: shareHolder.publicKey,
            share: 100,
          },
          {
            address: PublicKey.unique(),
            share: 100,
          },
        ],
      }
    );

    const { virtualPool, virtualPoolConfig } = await setupPool(
      svm,
      admin,
      user,
      poolCreator,
      payer,
      feeVault,
      quoteMint
    );

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    await withdrawDbcPartnerSurplus(
      svm,
      shareHolder,
      feeVault,
      tokenVault,
      virtualPoolConfig,
      virtualPool
    );

    const postTokenVaultBalance = getTokenBalance(svm, tokenVault);
    vaultState = getFeeVault(svm, feeVault);

    const postTotalFundedFee = vaultState.totalFundedFee;
    const postFeePerShare = vaultState.feePerShare;

    expect(postTotalFundedFee.sub(preTotalFundedFee).toString()).eq(
      postTokenVaultBalance.sub(preTokenVaultBalance).toString()
    );
    expect(Number(postFeePerShare.sub(preFeePerShare))).gt(0);
  });
});

async function setupPool(
  svm: LiteSVM,
  admin: Keypair,
  user: Keypair,
  poolCreator: Keypair,
  payer: Keypair,
  feeVault: PublicKey,
  quoteMint: PublicKey
) {
  let instructionParams = buildDefaultCurve();
  const params: CreateConfigParams = {
    payer,
    leftoverReceiver: feeVault,
    feeClaimer: feeVault,
    quoteMint,
    instructionParams,
  };
  mintToken(
    svm,
    admin,
    quoteMint,
    admin,
    user.publicKey,
    instructionParams.migrationQuoteThreshold.mul(new BN(2)).toNumber()
  );

  const virtualPoolConfig = await createConfig(svm, params);

  const virtualPool = await createVirtualPool(svm, {
    payer: poolCreator,
    poolCreator: poolCreator,
    quoteMint,
    config: virtualPoolConfig,
    instructionParams: {
      name: "test token spl",
      symbol: "TEST",
      uri: "abc.com",
    },
  });

  // transfer pool creator
    await transferCreator(svm, virtualPool, poolCreator, feeVault);

  let virtualPoolState = getVirtualPoolState(svm, virtualPool);
  let configState = getVirtualConfigState(svm, virtualPoolConfig);
  const amountIn = configState.migrationQuoteThreshold
    .mul(new BN(6))
    .div(new BN(5));
  // swap
  const swapParams: SwapParams = {
    config: virtualPoolConfig,
    payer: user,
    pool: virtualPool,
    inputTokenMint: quoteMint,
    outputTokenMint: virtualPoolState.baseMint,
    amountIn,
    minimumAmountOut: new BN(0),
    referralTokenAccount: null,
  };
  await swap(svm, swapParams);

  return { virtualPool, virtualPoolConfig };
}
