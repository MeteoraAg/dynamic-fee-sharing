import { LiteSVM, TransactionMetadata } from "litesvm";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  addUser,
  createProgram,
  createToken,
  deriveFeeVaultPdaAddress,
  deriveTokenVaultAddress,
  DynamicFeeSharingProgram,
  expectThrowsErrorCode,
  fundFee,
  generateUsers,
  getUserFees,
  getFeeVault,
  getUserUnclaimedFee,
  getOrCreateAtA,
  getProgramErrorCodeHexString,
  initializeFeeVaultPda,
  mintToken,
  claimUnclaimedFee,
  removeUser,
  TOKEN_DECIMALS,
  updateOperator,
  updateUserShare,
} from "./common";
import { BN } from "bn.js";
import { AccountLayout, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { expect } from "chai";

import DynamicFeeSharingIDL from "../target/idl/dynamic_fee_sharing.json";
import { getTokenBalance } from "./common/svm";

const fundAmount = new BN(100_000 * 10 ** TOKEN_DECIMALS);

describe("Fee vault pda sharing", () => {
  let program: DynamicFeeSharingProgram;
  let svm: LiteSVM;
  let admin: Keypair;
  let funder: Keypair;
  let vaultOwner: Keypair;
  let baseKp: Keypair;
  let tokenMint: PublicKey;
  let user: Keypair;
  let feeVault: PublicKey;
  let tokenVault: PublicKey;

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
    baseKp = Keypair.generate();

    svm.airdrop(admin.publicKey, BigInt(LAMPORTS_PER_SOL));
    svm.airdrop(vaultOwner.publicKey, BigInt(LAMPORTS_PER_SOL));
    svm.airdrop(funder.publicKey, BigInt(LAMPORTS_PER_SOL));
    svm.airdrop(user.publicKey, BigInt(LAMPORTS_PER_SOL));

    tokenMint = createToken(svm, admin, admin.publicKey, null);
    mintToken(svm, admin, tokenMint, admin, funder.publicKey);

    feeVault = deriveFeeVaultPdaAddress(baseKp.publicKey, tokenMint);
    tokenVault = deriveTokenVaultAddress(feeVault);
  });

  describe("Validation", () => {
    it("Fail to create more than max user", async () => {
      const users = generateUsers(svm, 6).map((item) => ({
        address: item.publicKey,
        share: 1000,
      }));

      const res = await initializeFeeVaultPda({
        svm,
        program,
        base: baseKp,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin,
        vaultParams: { padding: [], mutableFlag: false, users },
      });
      expectThrowsErrorCode(
        res,
        getProgramErrorCodeHexString("InvalidNumberOfUsers"),
      );
    });

    it("Fail to create with zero user", async () => {
      const res = await initializeFeeVaultPda({
        svm,
        program,
        base: baseKp,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin,
        vaultParams: { padding: [], mutableFlag: false, users: [] },
      });
      expectThrowsErrorCode(
        res,
        getProgramErrorCodeHexString("InvalidNumberOfUsers"),
      );
    });

    it("Fail to create with duplicate user addresses", async () => {
      const generatedUser = generateUsers(svm, 2);
      const users = [
        { address: generatedUser[0].publicKey, share: 1000 },
        { address: generatedUser[0].publicKey, share: 2000 },
      ];

      const res = await initializeFeeVaultPda({
        svm,
        program,
        base: baseKp,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin,
        vaultParams: { padding: [], mutableFlag: false, users },
      });
      expectThrowsErrorCode(
        res,
        getProgramErrorCodeHexString("InvalidUserAddress"),
      );
    });

    it("Fail to update operator when fee vault is not mutable", async () => {
      const users = generateUsers(svm, 5).map((item) => ({
        address: item.publicKey,
        share: 1000,
      }));

      const initializeFeeVaultRes = await initializeFeeVaultPda({
        svm,
        program,
        base: baseKp,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin,
        vaultParams: { mutableFlag: false, padding: [], users },
      });
      expect(initializeFeeVaultRes instanceof TransactionMetadata).to.be.true;

      const errorCode = getProgramErrorCodeHexString("FeeVaultNotMutable");

      const updateOperatorTx = await program.methods
        .updateOperator()
        .accountsPartial({
          feeVault,
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

      const initializeFeeVaultRes = await initializeFeeVaultPda({
        svm,
        program,
        base: baseKp,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin,
        vaultParams: { padding: [], mutableFlag: true, users },
      });
      expect(initializeFeeVaultRes instanceof TransactionMetadata).to.be.true;

      const errorCode = getProgramErrorCodeHexString("InvalidPermission");

      const newUser = Keypair.generate();
      const addTx = await program.methods
        .addUser(500)
        .accountsPartial({
          feeVault,
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
          feeVault,
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
        feeVault,
        operator: user.publicKey,
        vaultOwner,
      });

      svm.expireBlockhash();
      // expect update to succeed
      await updateUserShare({
        svm,
        program,
        feeVault,
        operator: user,
        user: generatedUser[0].publicKey,
        index: 0,
        share: 2000,
      });
    });
  });

  describe("initialized mutable fee vault", () => {
    let generatedUser: Keypair[];

    beforeEach(async () => {
      generatedUser = generateUsers(svm, 5);
      const initRes = await initializeFeeVaultPda({
        svm,
        program,
        base: baseKp,
        tokenMint,
        owner: vaultOwner.publicKey,
        payer: admin,
        vaultParams: {
          padding: [],
          mutableFlag: true,
          users: generatedUser.map((item) => ({
            address: item.publicKey,
            share: 1000,
          })),
        },
      });
      expect(initRes instanceof TransactionMetadata).to.be.true;

      await updateOperator({
        svm,
        program,
        feeVault,
        operator: user.publicKey,
        vaultOwner,
      });
    });

    it("Successfully add and remove dynamic users (realloc)", async () => {
      expect(getUserFees(svm, feeVault).length).eq(5);

      // Add 3 dynamic users (6, 7, 8)
      const dynamicUsers: Keypair[] = [];
      for (let i = 0; i < 3; i++) {
        const newUser = Keypair.generate();
        svm.airdrop(newUser.publicKey, BigInt(LAMPORTS_PER_SOL));
        await addUser({
          svm,
          program,
          feeVault,
          operator: user,
          user: newUser.publicKey,
          share: 500 + i * 100,
        });
        dynamicUsers.push(newUser);

        const allUsers = getUserFees(svm, feeVault);
        expect(allUsers.length).eq(6 + i);
        const lastUser = allUsers[allUsers.length - 1];
        expect(lastUser.address.equals(newUser.publicKey)).to.be.true;
        expect(lastUser.share).eq(500 + i * 100);
      }

      expect(getUserFees(svm, feeVault).length).eq(8);

      // Fund fee and verify all 8 users (fixed + dynamic) can claim
      svm.expireBlockhash();
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      const allUserKeys = [...generatedUser, ...dynamicUsers];
      const claimDeltas: InstanceType<typeof BN>[] = [];
      for (let i = 0; i < allUserKeys.length; i++) {
        const claimer = allUserKeys[i];
        const userTokenVault = getOrCreateAtA(
          svm,
          claimer,
          tokenMint,
          claimer.publicKey,
        );
        const beforeBalance = getTokenBalance(svm, userTokenVault);

        const claimTx = await program.methods
          .claimFee(i)
          .accountsPartial({
            feeVault,
            tokenMint,
            tokenVault,
            userTokenVault,
            user: claimer.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .transaction();
        claimTx.recentBlockhash = svm.latestBlockhash();
        claimTx.sign(claimer);

        const claimRes = svm.sendTransaction(claimTx);
        expect(claimRes instanceof TransactionMetadata).to.be.true;

        const afterBalance = getTokenBalance(svm, userTokenVault);
        claimDeltas.push(afterBalance.sub(beforeBalance));
      }

      // All users should have received fees
      expect(claimDeltas.every((d) => d.gtn(0))).to.be.true;
      // Fixed users (equal share=1000) should get the same amount
      expect(claimDeltas.slice(0, 5).every((d) => d.eq(claimDeltas[0]))).to.be
        .true;

      // Remove 2 dynamic users (index 7, then 6)
      for (let i = 0; i < 2; i++) {
        const userToRemove = dynamicUsers[dynamicUsers.length - 1 - i];
        const removeIndex = 7 - i;

        svm.expireBlockhash();
        await removeUser({
          svm,
          program,
          feeVault,
          signer: user,
          user: userToRemove.publicKey,
          index: removeIndex,
        });

        const allUsers = getUserFees(svm, feeVault);
        expect(allUsers.length).eq(7 - i);
        expect(allUsers.every((u) => !u.address.equals(userToRemove.publicKey)))
          .to.be.true;
      }

      expect(getUserFees(svm, feeVault).length).eq(6);

      // Fund again and verify remaining 6 users can claim
      svm.expireBlockhash();
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      const remainingUserKeys = [...generatedUser, dynamicUsers[0]];
      for (let i = 0; i < remainingUserKeys.length; i++) {
        const claimer = remainingUserKeys[i];
        const userTokenVault = getOrCreateAtA(
          svm,
          claimer,
          tokenMint,
          claimer.publicKey,
        );
        const beforeBalance = getTokenBalance(svm, userTokenVault);

        const claimTx = await program.methods
          .claimFee(i)
          .accountsPartial({
            feeVault,
            tokenMint,
            tokenVault,
            userTokenVault,
            user: claimer.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .transaction();
        claimTx.recentBlockhash = svm.latestBlockhash();
        claimTx.sign(claimer);

        const claimRes = svm.sendTransaction(claimTx);
        expect(claimRes instanceof TransactionMetadata).to.be.true;

        const afterBalance = getTokenBalance(svm, userTokenVault);
        expect(afterBalance.sub(beforeBalance).gtn(0)).to.be.true;
      }
    });

    it("Remove, re-add and remove user again accumulates unclaimed fees", async () => {
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      const removedUser = generatedUser[0];

      svm.expireBlockhash();
      const userUnclaimedFee = await removeUser({
        svm,
        program,
        feeVault,
        signer: user,
        user: removedUser.publicKey,
        index: 0,
      });

      const firstUnclaimed = getUserUnclaimedFee(
        svm,
        userUnclaimedFee,
      ).unclaimedFee;
      expect(firstUnclaimed.gtn(0)).to.be.true;

      // re-add the same user without claiming the unclaimed fees
      svm.expireBlockhash();
      await addUser({
        svm,
        program,
        feeVault,
        operator: user,
        user: removedUser.publicKey,
        share: 1000,
      });

      svm.expireBlockhash();
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      const reAddedIndex = getFeeVault(svm, feeVault).users.findIndex((u) =>
        u.address.equals(removedUser.publicKey),
      );
      svm.expireBlockhash();
      await removeUser({
        svm,
        program,
        feeVault,
        signer: user,
        user: removedUser.publicKey,
        index: reAddedIndex,
      });

      // second removal accumulates into the existing PDA
      const totalUnclaimed = getUserUnclaimedFee(
        svm,
        userUnclaimedFee,
      ).unclaimedFee;
      expect(totalUnclaimed.gt(firstUnclaimed)).to.be.true;

      // accumulated amount is claimable in full
      svm.expireBlockhash();
      const userTokenVault = getOrCreateAtA(
        svm,
        removedUser,
        tokenMint,
        removedUser.publicKey,
      );
      const beforeBalance = getTokenBalance(svm, userTokenVault);
      const claimRes = await claimUnclaimedFee({
        svm,
        program,
        feeVault,
        tokenMint,
        user: removedUser,
        operator: user.publicKey,
      });
      expect(claimRes instanceof TransactionMetadata).to.be.true;
      const afterBalance = getTokenBalance(svm, userTokenVault);
      expect(afterBalance.sub(beforeBalance).eq(totalUnclaimed)).to.be.true;
    });

    it("Remove, claim unclaimed, re-add and remove user again recreates the PDA", async () => {
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      const removedUser = generatedUser[0];

      svm.expireBlockhash();
      const userUnclaimedFee = await removeUser({
        svm,
        program,
        feeVault,
        signer: user,
        user: removedUser.publicKey,
        index: 0,
      });

      const firstUnclaimed = getUserUnclaimedFee(
        svm,
        userUnclaimedFee,
      ).unclaimedFee;
      expect(firstUnclaimed.gtn(0)).to.be.true;

      // claiming the unclaimed fees closes the PDA
      svm.expireBlockhash();
      const claimRes = await claimUnclaimedFee({
        svm,
        program,
        feeVault,
        tokenMint,
        user: removedUser,
        operator: user.publicKey,
      });
      expect(claimRes instanceof TransactionMetadata).to.be.true;
      expect(svm.getAccount(userUnclaimedFee).lamports).eq(0);

      // re-add the same user and accrue new fees
      svm.expireBlockhash();
      await addUser({
        svm,
        program,
        feeVault,
        operator: user,
        user: removedUser.publicKey,
        share: 1000,
      });

      svm.expireBlockhash();
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      const reAddedIndex = getFeeVault(svm, feeVault).users.findIndex((u) =>
        u.address.equals(removedUser.publicKey),
      );
      svm.expireBlockhash();
      await removeUser({
        svm,
        program,
        feeVault,
        signer: user,
        user: removedUser.publicKey,
        index: reAddedIndex,
      });

      // second removal recreates the closed PDA
      expect(svm.getAccount(userUnclaimedFee).lamports > 0).to.be.true;
      const secondUnclaimed = getUserUnclaimedFee(
        svm,
        userUnclaimedFee,
      ).unclaimedFee;
      expect(secondUnclaimed.eq(firstUnclaimed)).to.be.true;

      svm.expireBlockhash();
      const userTokenVault = getOrCreateAtA(
        svm,
        removedUser,
        tokenMint,
        removedUser.publicKey,
      );
      const beforeBalance = getTokenBalance(svm, userTokenVault);
      const claimRes2 = await claimUnclaimedFee({
        svm,
        program,
        feeVault,
        tokenMint,
        user: removedUser,
        operator: user.publicKey,
      });
      expect(claimRes2 instanceof TransactionMetadata).to.be.true;
      const afterBalance = getTokenBalance(svm, userTokenVault);
      expect(afterBalance.sub(beforeBalance).eq(secondUnclaimed)).to.be.true;
      expect(svm.getAccount(userUnclaimedFee).lamports).eq(0);
    });

    it("Full flow", async () => {
      const operator = user;

      const feeVaultState = getFeeVault(svm, feeVault);
      expect(feeVaultState.owner.toString()).eq(
        vaultOwner.publicKey.toString(),
      );
      expect(feeVaultState.tokenMint.toString()).eq(tokenMint.toString());
      expect(feeVaultState.tokenVault.toString()).eq(tokenVault.toString());
      expect(feeVaultState.totalShare).eq(1000 * generatedUser.length);
      expect(feeVaultState.totalFundedFee.toNumber()).eq(0);

      const totalUsers = feeVaultState.users.filter(
        (item) => !item.address.equals(PublicKey.default),
      ).length;
      expect(totalUsers).eq(generatedUser.length);

      console.log("fund fee");
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      console.log("User claim fee");

      for (let i = 0; i < generatedUser.length; i++) {
        const claimUser = generatedUser[i];
        const userTokenVault = getOrCreateAtA(
          svm,
          claimUser,
          tokenMint,
          claimUser.publicKey,
        );
        const claimFeeTx = await program.methods
          .claimFee(i)
          .accountsPartial({
            feeVault,
            tokenMint,
            tokenVault,
            userTokenVault,
            user: claimUser.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .transaction();

        claimFeeTx.recentBlockhash = svm.latestBlockhash();
        claimFeeTx.sign(claimUser);

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
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      console.log("update user share");
      await updateUserShare({
        svm,
        program,
        feeVault,
        operator,
        user: generatedUser[0].publicKey,
        index: 0,
        share: 2000,
      });

      console.log("user claim fee that was funded before share update");
      const tokenBalanceDeltasBefore = [];
      for (let i = 0; i < generatedUser.length; i++) {
        const claimUser = generatedUser[i];
        const userTokenVault = getOrCreateAtA(
          svm,
          claimUser,
          tokenMint,
          claimUser.publicKey,
        );
        const beforeUserBalance = getTokenBalance(svm, userTokenVault);
        const claimFeeTx = await program.methods
          .claimFee(i)
          .accountsPartial({
            feeVault,
            tokenMint,
            tokenVault,
            userTokenVault,
            user: claimUser.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .transaction();

        claimFeeTx.recentBlockhash = svm.latestBlockhash();
        claimFeeTx.sign(claimUser);

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
        fundAmount,
        feeVault,
        tokenMint,
      });

      console.log("user claim fee that was funded after share update");
      const tokenBalanceDeltasAfter = [];
      for (let i = 0; i < generatedUser.length; i++) {
        const claimUser = generatedUser[i];
        const userTokenVault = getOrCreateAtA(
          svm,
          claimUser,
          tokenMint,
          claimUser.publicKey,
        );
        const beforeUserBalance = getTokenBalance(svm, userTokenVault);
        const claimFeeTx = await program.methods
          .claimFee(i)
          .accountsPartial({
            feeVault,
            tokenMint,
            tokenVault,
            userTokenVault,
            user: claimUser.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .transaction();

        claimFeeTx.recentBlockhash = svm.latestBlockhash();
        claimFeeTx.sign(claimUser);

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
          .every(
            (delta) => delta.gtn(0) && delta.eq(tokenBalanceDeltasAfter[1]),
          ) && tokenBalanceDeltasAfter[0].gt(tokenBalanceDeltasAfter[1]),
      ).to.be.true;

      console.log("fund fee before remove user");
      svm.expireBlockhash();
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      const beforeFeePerShare = getFeeVault(svm, feeVault).feePerShare;

      console.log("remove user");
      const userUnclaimedFee = await removeUser({
        svm,
        program,
        feeVault,
        signer: operator,
        user: generatedUser[0].publicKey,
        index: 0,
      });

      const afterFeePerShare = getFeeVault(svm, feeVault).feePerShare;

      // fee_per_share should NOT increase
      expect(afterFeePerShare.eq(beforeFeePerShare)).to.be.true;
      // unclaimed fees are recorded in the removed user's fee record account
      const removedUserBalance = getUserUnclaimedFee(
        svm,
        userUnclaimedFee,
      ).unclaimedFee;
      expect(removedUserBalance.gtn(0)).to.be.true;

      console.log("claim unclaimed fee");
      svm.expireBlockhash();
      const operatorBalanceBefore = svm.getBalance(operator.publicKey);
      const userTokenBefore = getTokenBalance(
        svm,
        getOrCreateAtA(
          svm,
          generatedUser[0],
          tokenMint,
          generatedUser[0].publicKey,
        ),
      );

      const claimRes = await claimUnclaimedFee({
        svm,
        program,
        feeVault,
        tokenMint,
        user: generatedUser[0],
        operator: operator.publicKey,
      });
      expect(claimRes instanceof TransactionMetadata).to.be.true;

      // removed user should have received their unclaimed tokens
      const userTokenAfter = getTokenBalance(
        svm,
        getOrCreateAtA(
          svm,
          generatedUser[0],
          tokenMint,
          generatedUser[0].publicKey,
        ),
      );
      expect(userTokenAfter.sub(userTokenBefore).eq(removedUserBalance)).to.be
        .true;

      // removed user's unclaimed fee PDA should be closed
      const closedUserUnclaimedFee = svm.getAccount(userUnclaimedFee);
      expect(closedUserUnclaimedFee.lamports).eq(0);

      // operator should have received rent back from the closed unclaimed fee PDA
      const operatorBalanceAfter = svm.getBalance(operator.publicKey);
      expect(operatorBalanceAfter > operatorBalanceBefore).to.be.true;

      console.log("add new user after removing user[0]");
      svm.expireBlockhash();
      const newUser = Keypair.generate();
      svm.airdrop(newUser.publicKey, BigInt(LAMPORTS_PER_SOL));
      await addUser({
        svm,
        program,
        feeVault,
        operator,
        user: newUser.publicKey,
        share: 1500,
      });

      const feeVaultAfterAdd = getFeeVault(svm, feeVault);
      const newUserFee = feeVaultAfterAdd.users.find((u) =>
        u.address.equals(newUser.publicKey),
      );
      expect(newUserFee.share).eq(1500);
      // new user should not earn retroactive fees
      expect(newUserFee.pendingFee.toNumber()).eq(0);
      expect(newUserFee.feeClaimed.toNumber()).eq(0);
      expect(newUserFee.feePerShareCheckpoint.eq(feeVaultAfterAdd.feePerShare))
        .to.be.true;

      console.log("fund fee after adding new user");
      svm.expireBlockhash();
      await fundFee({
        svm,
        program,
        funder,
        fundAmount,
        feeVault,
        tokenMint,
      });

      console.log("new user claims fee");
      const newUserIndex = getFeeVault(svm, feeVault).users.findIndex((u) =>
        u.address.equals(newUser.publicKey),
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
          feeVault,
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
    });
  });
});
