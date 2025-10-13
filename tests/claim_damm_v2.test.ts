import { LiteSVM } from "litesvm";
import { PublicKey, Keypair, Transaction } from "@solana/web3.js";
import {
  generateUsers,
  getTokenBalance,
  sendTransactionOrExpectThrowError,
  startSvm,
  warpToTimestamp,
} from "./common/svm";
import {
  createToken,
  getFeeVault,
  mintToken,
} from "./common";
import { createDammV2Pool, dammV2Swap, initializeAndFundReward } from "./common/damm_v2";
import { claimDammV2Fee, claimDammV2Reward, createFeeVaultPda } from "./common/dfs";
import { BN } from "bn.js";
import { expect } from "chai";
import {
  AuthorityType,
  createSetAuthorityInstruction,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";

describe("Fund by claiming damm v2", () => {
  let svm: LiteSVM;
  let admin: Keypair;
  let creator: Keypair;
  let vaultOwner: Keypair;
  let tokenAMint: PublicKey;
  let tokenBMint: PublicKey;
  let rewardMint: PublicKey;
  let dammV2Pool: PublicKey;
  let positionNftAccount: PublicKey;
  let position: PublicKey;
  let shareHolder: Keypair;

  beforeEach(async () => {
    svm = startSvm();
    [admin, creator, vaultOwner, shareHolder] = generateUsers(svm, 4);
    tokenAMint = createToken(svm, admin, admin.publicKey, null);
    tokenBMint = createToken(svm, admin, admin.publicKey, null);
    rewardMint = createToken(svm, admin, admin.publicKey, null);

    mintToken(svm, admin, tokenAMint, admin, creator.publicKey);
    mintToken(svm, admin, tokenBMint, admin, creator.publicKey);
    mintToken(svm, admin, rewardMint, admin, creator.publicKey);

    // create damm v2 pool
    const createDmmV2PoolRes = await createDammV2Pool(
      svm,
      creator,
      tokenAMint,
      tokenBMint
    );
    dammV2Pool = createDmmV2PoolRes.pool;
    position = createDmmV2PoolRes.position;
    positionNftAccount = createDmmV2PoolRes.positionNftAccount;
  });

  it("Fund by claim damm v2 position fee", async () => {
    const { feeVault, tokenVault } = await createFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      tokenBMint,
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

    const setAuthorityIx = createSetAuthorityInstruction(
      positionNftAccount,
      creator.publicKey,
      AuthorityType.AccountOwner,
      feeVault,
      [],
      TOKEN_2022_PROGRAM_ID
    );
    const assignOwnerTx = new Transaction().add(setAuthorityIx);
    assignOwnerTx.recentBlockhash = svm.latestBlockhash();
    assignOwnerTx.sign(creator);

    sendTransactionOrExpectThrowError(svm, assignOwnerTx);

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    // swap damm v2
    await dammV2Swap(svm, {
      payer: creator,
      pool: dammV2Pool,
      inputTokenMint: tokenAMint,
      outputTokenMint: tokenBMint,
      amountIn: new BN(10000 * 10 ** 6),
      minimumAmountOut: new BN(0),
    });

    await claimDammV2Fee(
      svm,
      shareHolder,
      creator,
      feeVault,
      tokenVault,
      dammV2Pool,
      position,
      positionNftAccount
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


  it("Fund by claiming damm v2 reward", async () => {
    const { feeVault, tokenVault } = await createFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      rewardMint,
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

    const setAuthorityIx = createSetAuthorityInstruction(
      positionNftAccount,
      creator.publicKey,
      AuthorityType.AccountOwner,
      feeVault,
      [],
      TOKEN_2022_PROGRAM_ID
    );
    const assignOwnerTx = new Transaction().add(setAuthorityIx);
    assignOwnerTx.recentBlockhash = svm.latestBlockhash();
    assignOwnerTx.sign(creator);

    sendTransactionOrExpectThrowError(svm, assignOwnerTx);

    let vaultState = getFeeVault(svm, feeVault);

    const preTotalFundedFee = vaultState.totalFundedFee;
    const preFeePerShare = vaultState.feePerShare;

    const preTokenVaultBalance = getTokenBalance(svm, tokenVault);

    const rewardIndex = 0;
    await initializeAndFundReward(svm, creator, dammV2Pool, rewardMint, rewardIndex);

    warpToTimestamp(svm, new BN(12 * 60 * 60));
    
    await claimDammV2Reward(
      svm,
      shareHolder,
      creator,
      feeVault,
      tokenVault,
      dammV2Pool,
      position,
      positionNftAccount,
      rewardIndex
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
