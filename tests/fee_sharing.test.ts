import { LiteSVM, TransactionMetadata } from "litesvm";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  addUser,
  createProgram,
  createToken,
  deriveFeeVaultAuthorityAddress,
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
  claimUnclaimedFee,
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

describe("Fee vault sharing", () => {
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
      padding: [],
      mutableFlag: false,
      users,
    };

    const feeVault = Keypair.generate();
    const tokenVault = deriveTokenVaultAddress(feeVault.publicKey);
    const feeVaultAuthority = deriveFeeVaultAuthorityAddress();

    const tx = await program.methods
      .initializeFeeVault(params)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        feeVaultAuthority,
        tokenVault,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, feeVault);

    const errorCode = getProgramErrorCodeHexString("InvalidNumberOfUsers");
    expectThrowsErrorCode(svm.sendTransaction(tx), errorCode);
  });

  it("Fail to create with zero user", async () => {
    const users = [];

    const params: InitializeFeeVaultParameters = {
      padding: [],
      mutableFlag: false,
      users,
    };

    const feeVault = Keypair.generate();
    const tokenVault = deriveTokenVaultAddress(feeVault.publicKey);
    const feeVaultAuthority = deriveFeeVaultAuthorityAddress();

    const tx = await program.methods
      .initializeFeeVault(params)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        feeVaultAuthority,
        tokenVault,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, feeVault);

    const errorCode = getProgramErrorCodeHexString("InvalidNumberOfUsers");
    expectThrowsErrorCode(svm.sendTransaction(tx), errorCode);
  });

  it("Fail to create with duplicate user addresses", async () => {
    const generatedUser = generateUsers(svm, 2);
    const users = [
      { address: generatedUser[0].publicKey, share: 1000 },
      { address: generatedUser[0].publicKey, share: 2000 },
    ];

    const params: InitializeFeeVaultParameters = {
      padding: [],
      mutableFlag: false,
      users,
    };

    const feeVault = Keypair.generate();
    const tokenVault = deriveTokenVaultAddress(feeVault.publicKey);
    const feeVaultAuthority = deriveFeeVaultAuthorityAddress();

    const tx = await program.methods
      .initializeFeeVault(params)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        feeVaultAuthority,
        tokenVault,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, feeVault);

    const errorCode = getProgramErrorCodeHexString("InvalidUserAddress");
    expectThrowsErrorCode(svm.sendTransaction(tx), errorCode);
  });

  it("Fail to update operator when fee vault is not mutable", async () => {
    const generatedUser = generateUsers(svm, 5);
    const users = generatedUser.map((item) => ({
      address: item.publicKey,
      share: 1000,
    }));

    const params: InitializeFeeVaultParameters = {
      mutableFlag: false,
      padding: [],
      users,
    };

    const feeVault = Keypair.generate();
    const tokenVault = deriveTokenVaultAddress(feeVault.publicKey);
    const feeVaultAuthority = deriveFeeVaultAuthorityAddress();

    const tx = await program.methods
      .initializeFeeVault(params)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        feeVaultAuthority,
        tokenVault,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, feeVault);
    const initializeFeeVaultRes = svm.sendTransaction(tx);
    expect(initializeFeeVaultRes instanceof TransactionMetadata).to.be.true;

    const errorCode = getProgramErrorCodeHexString("FeeVaultNotMutable");

    const updateOperatorTx = await program.methods
      .updateOperator()
      .accountsPartial({
        feeVault: feeVault.publicKey,
        operator: user.publicKey,
        owner: vaultOwner.publicKey,
      })
      .transaction();
    updateOperatorTx.recentBlockhash = svm.latestBlockhash();
    updateOperatorTx.sign(vaultOwner);
    const updateOperatorRes = svm.sendTransaction(updateOperatorTx);
    expectThrowsErrorCode(updateOperatorRes, errorCode);
  });

  it("Fail to perform admin task when not an admin", async () => {
    const generatedUser = generateUsers(svm, 5);
    const users = generatedUser.map((item) => ({
      address: item.publicKey,
      share: 1000,
    }));

    const params: InitializeFeeVaultParameters = {
      padding: [],
      mutableFlag: true,
      users,
    };

    const feeVault = Keypair.generate();
    const tokenVault = deriveTokenVaultAddress(feeVault.publicKey);
    const feeVaultAuthority = deriveFeeVaultAuthorityAddress();

    const tx = await program.methods
      .initializeFeeVault(params)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        feeVaultAuthority,
        tokenVault,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, feeVault);
    const initializeFeeVaultRes = svm.sendTransaction(tx);
    expect(initializeFeeVaultRes instanceof TransactionMetadata).to.be.true;

    const errorCode = getProgramErrorCodeHexString("InvalidPermission");

    const newUser = Keypair.generate();
    const addTx = await program.methods
      .addUser(500)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        user: newUser.publicKey,
        signer: user.publicKey,
      })
      .transaction();
    addTx.recentBlockhash = svm.latestBlockhash();
    addTx.sign(user);
    const addRes = svm.sendTransaction(addTx);
    expectThrowsErrorCode(addRes, errorCode);

    const updateTx1 = await program.methods
      .updateUserShare(0, 2000)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        user: generatedUser[0].publicKey,
        signer: user.publicKey,
      })
      .transaction();
    updateTx1.recentBlockhash = svm.latestBlockhash();
    updateTx1.sign(user);
    const updateUserShareRes1 = svm.sendTransaction(updateTx1);
    expectThrowsErrorCode(updateUserShareRes1, errorCode);

    await updateOperator({
      svm,
      program,
      feeVault: feeVault.publicKey,
      operator: user.publicKey,
      vaultOwner,
    });

    svm.expireBlockhash();
    // expect update to succeed
    await updateUserShare({
      svm,
      program,
      feeVault: feeVault.publicKey,
      operator: user,
      user: generatedUser[0].publicKey,
      index: 0,
      share: 2000,
    });
  });

  it("Fail to add 6th user (exceeds MAX_USER)", async () => {
    const generatedUser = generateUsers(svm, 5);
    const users = generatedUser.map((item) => ({
      address: item.publicKey,
      share: 1000,
    }));

    const params: InitializeFeeVaultParameters = {
      padding: [],
      mutableFlag: true,
      users,
    };

    const feeVault = Keypair.generate();
    const tokenVault = deriveTokenVaultAddress(feeVault.publicKey);
    const feeVaultAuthority = deriveFeeVaultAuthorityAddress();

    const tx = await program.methods
      .initializeFeeVault(params)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        feeVaultAuthority,
        tokenVault,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();

    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(admin, feeVault);
    const initRes = svm.sendTransaction(tx);
    expect(initRes instanceof TransactionMetadata).to.be.true;

    await updateOperator({
      svm,
      program,
      feeVault: feeVault.publicKey,
      operator: user.publicKey,
      vaultOwner,
    });

    const errorCode = getProgramErrorCodeHexString("InvalidNumberOfUsers");
    const newUser = Keypair.generate();
    const addTx = await program.methods
      .addUser(500)
      .accountsPartial({
        feeVault: feeVault.publicKey,
        user: newUser.publicKey,
        signer: user.publicKey,
      })
      .transaction();
    addTx.recentBlockhash = svm.latestBlockhash();
    addTx.sign(user);
    const addRes = svm.sendTransaction(addTx);
    expectThrowsErrorCode(addRes, errorCode);
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
      padding: [],
      mutableFlag: true,
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
  const feeVault = Keypair.generate();
  const tokenVault = deriveTokenVaultAddress(feeVault.publicKey);
  const feeVaultAuthority = deriveFeeVaultAuthorityAddress();

  console.log("initialize fee vault");
  const tx = await program.methods
    .initializeFeeVault(params)
    .accountsPartial({
      feeVault: feeVault.publicKey,
      feeVaultAuthority,
      tokenVault,
      tokenMint,
      owner: vaultOwner.publicKey,
      payer: admin.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(admin, feeVault);

  const sendRes = svm.sendTransaction(tx);

  if (sendRes instanceof TransactionMetadata) {
    const feeVaultState = getFeeVault(svm, feeVault.publicKey);
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
    feeVault: feeVault.publicKey,
    operator: operator.publicKey,
    vaultOwner,
  });

  console.log("fund fee");
  await fundFee({
    svm,
    program,
    funder,
    fundAmount: new BN(100_000 * 10 ** TOKEN_DECIMALS),
    feeVault: feeVault.publicKey,
    tokenMint,
  });

  console.log("User claim fee");

  for (let i = 0; i < users.length; i++) {
    const user = users[i];
    const userTokenVault = getOrCreateAtA(svm, user, tokenMint, user.publicKey);
    const claimFeeTx = await program.methods
      .claimFee(i)
      .accountsPartial({
        feeVault: feeVault.publicKey,
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
      const feeVaultState = getFeeVault(svm, feeVault.publicKey);
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
  await fundFee({
    svm,
    program,
    funder,
    fundAmount: new BN(100_000 * 10 ** TOKEN_DECIMALS),
    feeVault: feeVault.publicKey,
    tokenMint,
  });

  console.log("update user share");
  await updateUserShare({
    svm,
    program,
    feeVault: feeVault.publicKey,
    operator,
    user: users[0].publicKey,
    index: 0,
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
        feeVault: feeVault.publicKey,
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
  await fundFee({
    svm,
    program,
    funder,
    fundAmount: new BN(100_000 * 10 ** TOKEN_DECIMALS),
    feeVault: feeVault.publicKey,
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
        feeVault: feeVault.publicKey,
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
    feeVault: feeVault.publicKey,
    tokenMint,
  });

  const beforeFeePerShare = getFeeVault(svm, feeVault.publicKey).feePerShare;

  console.log("remove user");
  const userUnclaimedFee = await removeUser({
    svm,
    program,
    feeVault: feeVault.publicKey,
    signer: operator,
    user: users[0].publicKey,
    index: 0,
  });

  const afterFeePerShare = getFeeVault(svm, feeVault.publicKey).feePerShare;

  // fee_per_share should NOT increase
  expect(afterFeePerShare.eq(beforeFeePerShare)).to.be.true;
  // unclaimed fees are recorded in the removed user's fee record account
  const userUnclaimedFeeAccount = svm.getAccount(userUnclaimedFee);
  const removedUserBalance = new BN(
    userUnclaimedFeeAccount.data.slice(8, 16),
    "le",
  );
  expect(removedUserBalance.gtn(0)).to.be.true;

  console.log("claim unclaimed fee");
  svm.expireBlockhash();
  const ownerBalanceBefore = svm.getBalance(vaultOwner.publicKey);
  const userTokenBefore = getTokenBalance(
    svm,
    getOrCreateAtA(svm, users[0], tokenMint, users[0].publicKey),
  );

  const claimRes = await claimUnclaimedFee({
    svm,
    program,
    feeVault: feeVault.publicKey,
    tokenMint,
    user: users[0],
    owner: vaultOwner.publicKey,
  });
  expect(claimRes instanceof TransactionMetadata).to.be.true;

  // removed user should have received their unclaimed tokens
  const userTokenAfter = getTokenBalance(
    svm,
    getOrCreateAtA(svm, users[0], tokenMint, users[0].publicKey),
  );
  expect(userTokenAfter.sub(userTokenBefore).eq(removedUserBalance)).to.be.true;

  // removed user token vault PDA should be closed
  const closedUserUnclaimedFee = svm.getAccount(userUnclaimedFee);
  expect(closedUserUnclaimedFee.lamports).eq(0);

  // owner should have received rent back from removed user token vault
  const ownerBalanceAfter = svm.getBalance(vaultOwner.publicKey);
  expect(ownerBalanceAfter > ownerBalanceBefore).to.be.true;

  console.log("add new user after removing user[0]");
  svm.expireBlockhash();
  const newUser = Keypair.generate();
  svm.airdrop(newUser.publicKey, BigInt(LAMPORTS_PER_SOL));
  await addUser({
    svm,
    program,
    feeVault: feeVault.publicKey,
    operator,
    user: newUser.publicKey,
    share: 1500,
  });

  const feeVaultAfterAdd = getFeeVault(svm, feeVault.publicKey);
  const newUserFee = feeVaultAfterAdd.users.find((user) =>
    user.address.equals(newUser.publicKey),
  );
  expect(newUserFee.share).eq(1500);
  // new user should not earn retroactive fees
  expect(newUserFee.pendingFee.toNumber()).eq(0);
  expect(newUserFee.feeClaimed.toNumber()).eq(0);

  console.log("fund fee after adding new user");
  svm.expireBlockhash();
  await fundFee({
    svm,
    program,
    funder,
    fundAmount: new BN(100_000 * 10 ** TOKEN_DECIMALS),
    feeVault: feeVault.publicKey,
    tokenMint,
  });

  console.log("new user claims fee");
  const newUserIndex = getFeeVault(svm, feeVault.publicKey).users.findIndex(
    (user) => user.address.equals(newUser.publicKey),
  );
  const newUserTokenVault = getOrCreateAtA(
    svm,
    newUser,
    tokenMint,
    newUser.publicKey,
  );
  const beforeNewUserBalance = getTokenBalance(svm, newUserTokenVault);
  const claimNewUserTx = await program.methods
    .claimFee(newUserIndex)
    .accountsPartial({
      feeVault: feeVault.publicKey,
      tokenMint,
      tokenVault,
      userTokenVault: newUserTokenVault,
      user: newUser.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();
  claimNewUserTx.recentBlockhash = svm.latestBlockhash();
  claimNewUserTx.sign(newUser);

  const claimNewUserRes = svm.sendTransaction(claimNewUserTx);
  expect(claimNewUserRes instanceof TransactionMetadata).to.be.true;

  const afterNewUserBalance = getTokenBalance(svm, newUserTokenVault);
  expect(afterNewUserBalance.sub(beforeNewUserBalance).gtn(0)).to.be.true;
}
