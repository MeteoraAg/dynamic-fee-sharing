import { BN } from "bn.js";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { config, expect } from "chai";
import { LiteSVM } from "litesvm";
import { generateUsers, getTokenBalance, startSvm } from "./common/svm";
import { createToken, getFeeVault, mintToken } from "./common";
import {
  buildDefaultCurve,
  createConfig,
  CreateConfigParams,
  createVirtualPool,
  getVirtualConfigState,
  getVirtualPoolState,
  swap,
  SwapParams,
} from "./common/dbc";
import {
  claimDbcCreatorTradingFee,
  claimDbcTradingFee,
  createFeeVaultPda,
  withdrawDbcCreatorSurplus,
  withdrawDbcPartnerSurplus,
} from "./common/dfs";

describe("Claim fee and withdraw dbc surplus", () => {
  let svm: LiteSVM;
  let admin: Keypair;
  let feeClaimer: Keypair;
  let user: Keypair;
  let poolCreator: Keypair;
  let virtualPool: PublicKey;
  let vaultOwner: Keypair;
  let quoteMint: PublicKey;
  let virtualPoolConfig: PublicKey;

  beforeEach(async () => {
    svm = startSvm();
    admin = Keypair.generate();
    feeClaimer = Keypair.generate();
    user = Keypair.generate();
    poolCreator = Keypair.generate();
    [admin, feeClaimer, user, poolCreator, vaultOwner] = generateUsers(svm, 5);
    quoteMint = createToken(svm, admin, admin.publicKey, null);
    let instructionParams = buildDefaultCurve();
    const params: CreateConfigParams = {
      payer: feeClaimer,
      leftoverReceiver: feeClaimer.publicKey,
      feeClaimer: feeClaimer.publicKey,
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

    virtualPoolConfig = await createConfig(svm, params);

    virtualPool = await createVirtualPool(svm, {
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
            address: PublicKey.unique(),
            share: 100,
          },
          {
            address: PublicKey.unique(),
            share: 100,
          },
        ],
      }
    );

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    await claimDbcCreatorTradingFee(
      svm,
      poolCreator,
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

  it("claim dbc trading fee", async () => {
    const { feeVault, tokenVault } = await createFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      quoteMint,
      {
        padding: [],
        users: [
          {
            address: PublicKey.unique(),
            share: 100,
          },
          {
            address: PublicKey.unique(),
            share: 100,
          },
        ],
      }
    );

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    await claimDbcTradingFee(
      svm,
      feeClaimer,
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
            address: PublicKey.unique(),
            share: 100,
          },
          {
            address: PublicKey.unique(),
            share: 100,
          },
        ],
      }
    );

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    await withdrawDbcCreatorSurplus(
      svm,
      poolCreator,
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
            address: PublicKey.unique(),
            share: 100,
          },
          {
            address: PublicKey.unique(),
            share: 100,
          },
        ],
      }
    );

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    await withdrawDbcPartnerSurplus(
      svm,
      feeClaimer,
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
