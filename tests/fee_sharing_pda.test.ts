import { LiteSVM, TransactionMetadata } from "litesvm";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  createProgram,
  createToken,
  deriveFeeVaultAuthorityAddress,
  deriveFeeVaultPdaAddress,
  deriveTokenVaultAddress,
  DynamicFeeSharingProgram,
  expectThrowsErrorCode,
  fundFee,
  generateUsers,
  getFeeVault,
  getOrCreateAtA,
  getProgramErrorCodeHexString,
  InitializeFeeVaultParameters,
  mintToken,
  removeUser,
  TOKEN_DECIMALS,
  updateOperator,
  updateUserShare,
} from "./common";
import { BN } from "bn.js";
import {
  AccountLayout,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";

import DynamicFeeSharingIDL from "../target/idl/dynamic_fee_sharing.json";
import { getTokenBalance } from "./common/svm";

describe("Fee vault pda sharing", () => {
  let program: DynamicFeeSharingProgram;
  let svm: LiteSVM;
  let admin: Keypair;
  let funder: Keypair;
  let vaultOwner: Keypair;
  let tokenMint: PublicKey;
  let user: Keypair;

  beforeEach(async () => {
    program = createProgram();
    svm = new LiteSVM();
    svm.addProgramFromFile(
      new PublicKey(DynamicFeeSharingIDL.address),
      "./target/deploy/dynamic_fee_sharing.so",
    );

    admin = Keypair.generate();
    vaultOwner = Keypair.generate();
    funder = Keypair.generate();
    user = Keypair.generate();

    svm.airdrop(admin.publicKey, BigInt(LAMPORTS_PER_SOL));
    svm.airdrop(vaultOwner.publicKey, BigInt(LAMPORTS_PER_SOL));
    svm.airdrop(funder.publicKey, BigInt(LAMPORTS_PER_SOL));
    svm.airdrop(user.publicKey, BigInt(LAMPORTS_PER_SOL));

    tokenMint = createToken(svm, admin, admin.publicKey, null);
    mintToken(svm, admin, tokenMint, admin, funder.publicKey);
  });

  it("Fail to create more than max user", async () => {
    const generatedUser = generateUsers(svm, 6); // 6 users
    const users = generatedUser.map((item) => {
      return {
        address: item.publicKey,
        share: 1000,
      };
    });

    const params: InitializeFeeVaultParameters = {
      mutableFlag: 0,
      padding: [],
      users,
    };

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
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, baseKp);

    const errorCode = getProgramErrorCodeHexString("InvalidNumberOfUsers");
    expectThrowsErrorCode(svm.sendTransaction(tx), errorCode);
  });

  it("Fail to create with zero user", async () => {
    const users = [];

    const params: InitializeFeeVaultParameters = {
      mutableFlag: 0,
      padding: [],
      users,
    };
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
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, baseKp);

    const errorCode = getProgramErrorCodeHexString("InvalidNumberOfUsers");
    expectThrowsErrorCode(svm.sendTransaction(tx), errorCode);
  });

  it("Fail to update user share and remove user when fee vault is not mutable", async () => {
    const generatedUser = generateUsers(svm, 5);
    const users = generatedUser.map((item) => ({
      address: item.publicKey,
      share: 1000,
    }));

    const params: InitializeFeeVaultParameters = {
      mutableFlag: 0,
      padding: [],
      users,
    };

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
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, baseKp);
    const initializeFeeVaultRes = svm.sendTransaction(tx);
    expect(initializeFeeVaultRes instanceof TransactionMetadata).to.be.true;

    await updateOperator({
      svm,
      program,
      feeVault,
      operator: user.publicKey,
      vaultOwner,
    });

    const errorCode = getProgramErrorCodeHexString("InvalidAction");

    const updateTx = await program.methods
      .updateUserShare(0, 2000)
      .accountsPartial({
        feeVault,
        signer: user.publicKey,
      })
      .transaction();
    updateTx.recentBlockhash = svm.latestBlockhash();
    updateTx.sign(user);
    const updateUserShareRes = svm.sendTransaction(updateTx);
    expectThrowsErrorCode(updateUserShareRes, errorCode);

    const removeTx = await program.methods
      .removeUser(0)
      .accountsPartial({
        feeVault,
        signer: user.publicKey,
      })
      .transaction();
    removeTx.recentBlockhash = svm.latestBlockhash();
    removeTx.sign(user);
    const removeUserRes = svm.sendTransaction(removeTx);
    expectThrowsErrorCode(removeUserRes, errorCode);
  });

  it("Full flow", async () => {
    const generatedUser = generateUsers(svm, 5); // 5 users
    const users = generatedUser.map((item) => {
      return {
        address: item.publicKey,
        share: 1000,
      };
    });

    const params: InitializeFeeVaultParameters = {
      mutableFlag: 1,
      padding: [],
      users,
    };

    await fullFlow(
      svm,
      admin,
      funder,
      generatedUser,
      vaultOwner,
      tokenMint,
      user,
      params,
    );
  });
});

async function fullFlow(
  svm: LiteSVM,
  admin: Keypair,
  funder: Keypair,
  users: Keypair[],
  vaultOwner: Keypair,
  tokenMint: PublicKey,
  operator: Keypair,
  params: InitializeFeeVaultParameters,
) {
  const program = createProgram();
  const baseKp = Keypair.generate();
  const feeVault = deriveFeeVaultPdaAddress(baseKp.publicKey, tokenMint);
  const tokenVault = deriveTokenVaultAddress(feeVault);
  const feeVaultAuthority = deriveFeeVaultAuthorityAddress();

  console.log("initialize fee vault");
  const tx = await program.methods
    .initializeFeeVaultPda(params)
    .accountsPartial({
      feeVault,
      base: baseKp.publicKey,
      feeVaultAuthority,
      tokenVault,
      tokenMint,
      owner: vaultOwner.publicKey,
      payer: admin.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(admin, baseKp);

  const sendRes = svm.sendTransaction(tx);

  if (sendRes instanceof TransactionMetadata) {
    const feeVaultState = getFeeVault(svm, feeVault);
    expect(feeVaultState.owner.toString()).eq(vaultOwner.publicKey.toString());
    expect(feeVaultState.tokenMint.toString()).eq(tokenMint.toString());
    expect(feeVaultState.tokenVault.toString()).eq(tokenVault.toString());
    const totalShare = params.users.reduce(
      (a, b) => a.add(new BN(b.share)),
      new BN(0),
    );
    expect(feeVaultState.totalShare).eq(totalShare.toNumber());
    expect(feeVaultState.totalFundedFee.toNumber()).eq(0);

    const totalUsers = feeVaultState.users.filter(
      (item) => !item.address.equals(PublicKey.default),
    ).length;
    expect(totalUsers).eq(params.users.length);
  } else {
    console.log(sendRes.meta().logs());
  }

  console.log("create vault operator account");
  await updateOperator({
    svm,
    program,
    feeVault,
    operator: operator.publicKey,
    vaultOwner,
  });

  console.log("fund fee");

  fundFee({
    svm,
    program,
    funder,
    fundAmount: new BN(100_000 * 10 ** TOKEN_DECIMALS),
    feeVault,
    tokenMint,
  });

  console.log("User claim fee");

  for (let i = 0; i < users.length; i++) {
    const user = users[i];
    const userTokenVault = getOrCreateAtA(svm, user, tokenMint, user.publicKey);
    const claimFeeTx = await program.methods
      .claimFee(i)
      .accountsPartial({
        feeVault,
        tokenMint,
        tokenVault,
        userTokenVault,
        user: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    claimFeeTx.recentBlockhash = svm.latestBlockhash();
    claimFeeTx.sign(user);

    const claimFeeRes = svm.sendTransaction(claimFeeTx);

    if (claimFeeRes instanceof TransactionMetadata) {
      const feeVaultState = getFeeVault(svm, feeVault);
      const account = svm.getAccount(userTokenVault);
      const userTokenBalance = AccountLayout.decode(
        account.data,
      ).amount.toString();
      expect(userTokenBalance.toString()).eq(
        feeVaultState.users[i].feeClaimed.toString(),
      );
    } else {
      console.log(claimFeeRes.meta().logs());
    }
  }

  console.log("fund fee before share update");
  svm.expireBlockhash();
  fundFee({
    svm,
    program,
    funder,
    fundAmount: new BN(100_000 * 10 ** TOKEN_DECIMALS),
    feeVault,
    tokenMint,
  });

  console.log("update user share");
  await updateUserShare({
    svm,
    program,
    feeVault,
    operator,
    userIndex: 0,
    share: 2000,
  });

  console.log("user claim fee that was funded before share update");
  const tokenBalanceDeltasBefore = [];
  for (let i = 0; i < users.length; i++) {
    const user = users[i];
    const userTokenVault = getOrCreateAtA(svm, user, tokenMint, user.publicKey);
    const beforeUserBalance = getTokenBalance(svm, userTokenVault);
    const claimFeeTx = await program.methods
      .claimFee(i)
      .accountsPartial({
        feeVault,
        tokenMint,
        tokenVault,
        userTokenVault,
        user: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    claimFeeTx.recentBlockhash = svm.latestBlockhash();
    claimFeeTx.sign(user);

    const claimFeeRes = svm.sendTransaction(claimFeeTx);
    expect(claimFeeRes instanceof TransactionMetadata).to.be.true;
    const afterUserBalance = getTokenBalance(svm, userTokenVault);
    tokenBalanceDeltasBefore.push(afterUserBalance.sub(beforeUserBalance));
  }

  // all users should have the same token balance delta since the fee was funded before share was updated
  expect(
    tokenBalanceDeltasBefore.every(
      (delta) => delta.gtn(0) && delta.eq(tokenBalanceDeltasBefore[0]),
    ),
  ).to.be.true;

  console.log("fund fee after share update");
  svm.expireBlockhash();
  fundFee({
    svm,
    program,
    funder,
    fundAmount: new BN(100_000 * 10 ** TOKEN_DECIMALS),
    feeVault,
    tokenMint,
  });

  console.log("user claim fee that was funded after share update");
  const tokenBalanceDeltasAfter = [];
  for (let i = 0; i < users.length; i++) {
    const user = users[i];
    const userTokenVault = getOrCreateAtA(svm, user, tokenMint, user.publicKey);
    const beforeUserBalance = getTokenBalance(svm, userTokenVault);
    const claimFeeTx = await program.methods
      .claimFee(i)
      .accountsPartial({
        feeVault,
        tokenMint,
        tokenVault,
        userTokenVault,
        user: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    claimFeeTx.recentBlockhash = svm.latestBlockhash();
    claimFeeTx.sign(user);

    const claimFeeRes = svm.sendTransaction(claimFeeTx);
    expect(claimFeeRes instanceof TransactionMetadata).to.be.true;
    const afterUserBalance = getTokenBalance(svm, userTokenVault);
    tokenBalanceDeltasAfter.push(afterUserBalance.sub(beforeUserBalance));
  }

  // user 0 should have a higher token balance delta compared to the other users
  // all others should have the same token balance delta
  expect(
    tokenBalanceDeltasAfter
      .slice(1)
      .every((delta) => delta.gtn(0) && delta.eq(tokenBalanceDeltasAfter[1])) &&
      tokenBalanceDeltasAfter[0].gt(tokenBalanceDeltasAfter[1]),
  ).to.be.true;

  console.log("fund fee before remove user");
  svm.expireBlockhash();
  await fundFee({
    svm,
    program,
    funder,
    fundAmount: new BN(100_000 * 10 ** TOKEN_DECIMALS),
    feeVault,
    tokenMint,
  });

  const beforeFeePerShare = getFeeVault(svm, feeVault).feePerShare;

  console.log("remove user");
  await removeUser({
    svm,
    program,
    feeVault,
    signer: operator,
    userIndex: 0,
  });

  const afterFeePerShare = getFeeVault(svm, feeVault).feePerShare;

  // fee_per_share should increase because removed user's unclaimed fees are redistributed
  expect(afterFeePerShare.gt(beforeFeePerShare)).to.be.true;
}
