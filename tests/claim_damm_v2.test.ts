import { LiteSVM } from "litesvm";
import { PublicKey, Keypair, Transaction } from "@solana/web3.js";
import {
  generateUsers,
  getTokenBalance,
  sendTransactionOrExpectThrowError,
  startSvm,
} from "./common/svm";
import {
  createToken,
  deriveFeeVaultAuthorityAddress,
  expectThrowsErrorCode,
  getFeeVault,
  getProgramErrorCodeHexString,
  mintToken,
} from "./common";
import { createDammV2Pool, dammV2Swap } from "./common/damm_v2";
import { claimDammV2Fee, createFeeVaultPda } from "./common/dfs";
import { BN } from "bn.js";
import { expect } from "chai";
import {
  AuthorityType,
  createSetAuthorityInstruction,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";

describe.only("Claim damm v2 fee", () => {
  let svm: LiteSVM;
  let admin: Keypair;
  let creator: Keypair;
  let vaultOwner: Keypair;
  let tokenAMint: PublicKey;
  let tokenBMint: PublicKey;
  let dammV2Pool: PublicKey;
  let positionNftAccount: PublicKey;
  let position: PublicKey;
  let shareHolder: Keypair;

  beforeEach(async () => {
    svm = startSvm();
    [admin, creator, vaultOwner, shareHolder] = generateUsers(svm, 4);
    tokenAMint = createToken(svm, admin, admin.publicKey, null);
    tokenBMint = createToken(svm, admin, admin.publicKey, null);

    mintToken(svm, admin, tokenAMint, admin, creator.publicKey);
    mintToken(svm, admin, tokenBMint, admin, creator.publicKey);

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

  it("fullflow claim damm v2", async () => {
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


    // await claimDammV2FeeExpectThrowError(
    //   svm,
    //   creator,
    //   creator,
    //   feeVault,
    //   tokenVault,
    //   dammV2Pool,
    //   position,
    //   positionNftAccount
    // )

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
});
