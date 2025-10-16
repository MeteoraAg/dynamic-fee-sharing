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
  generateUsers,
  getFeeVault,
  getOrCreateAtA,
  getProgramErrorCodeHexString,
  InitializeFeeVaultParameters,
  mintToken,
  TOKEN_DECIMALS,
} from "./common";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { BN } from "bn.js";
import {
  AccountLayout,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { expect } from "chai";

import DynamicFeeSharingIDL from "../target/idl/dynamic_fee_sharing.json";

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
      "./target/deploy/dynamic_fee_sharing.so"
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

    const errorCode = getProgramErrorCodeHexString("ExceededUser");
    expectThrowsErrorCode(svm.sendTransaction(tx), errorCode);
  });

  it("Fail to create with zero user", async () => {
    const users = [];

    const params: InitializeFeeVaultParameters = {
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

    const errorCode = getProgramErrorCodeHexString("ExceededUser");
    expectThrowsErrorCode(svm.sendTransaction(tx), errorCode);
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
      users,
    };

    await fullFlow(
      svm,
      admin,
      funder,
      generatedUser,
      vaultOwner.publicKey,
      tokenMint,
      params
    );
  });
});

async function fullFlow(
  svm: LiteSVM,
  admin: Keypair,
  funder: Keypair,
  users: Keypair[],
  vaultOwner: PublicKey,
  tokenMint: PublicKey,
  params: InitializeFeeVaultParameters
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
      owner: vaultOwner,
      payer: admin.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(admin, baseKp);

  const sendRes = svm.sendTransaction(tx);

  if (sendRes instanceof TransactionMetadata) {
    const feeVaultState = getFeeVault(svm, feeVault);
    expect(feeVaultState.owner.toString()).eq(vaultOwner.toString());
    expect(feeVaultState.tokenMint.toString()).eq(tokenMint.toString());
    expect(feeVaultState.tokenVault.toString()).eq(tokenVault.toString());
    const totalShare = params.users.reduce(
      (a, b) => a.add(new BN(b.share)),
      new BN(0)
    );
    expect(feeVaultState.totalShare).eq(totalShare.toNumber());
    expect(feeVaultState.totalFundedFee.toNumber()).eq(0);

    const totalUsers = feeVaultState.users.filter(
      (item) => !item.address.equals(PublicKey.default)
    ).length;
    expect(totalUsers).eq(params.users.length);
  } else {
    console.log(sendRes.meta().logs());
  }

  console.log("fund fee");

  const fundTokenVault = getAssociatedTokenAddressSync(
    tokenMint,
    funder.publicKey
  );
  const fundAmount = new BN(100_000 * 10 ** TOKEN_DECIMALS);
  const fundFeeTx = await program.methods
    .fundFee(fundAmount)
    .accountsPartial({
      feeVault,
      tokenVault,
      tokenMint,
      fundTokenVault,
      funder: funder.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();

  fundFeeTx.recentBlockhash = svm.latestBlockhash();
  fundFeeTx.sign(funder);

  const fundFeeRes = svm.sendTransaction(fundFeeTx);

  if (fundFeeRes instanceof TransactionMetadata) {
    const feeVaultState = getFeeVault(svm, feeVault);
    const account = svm.getAccount(tokenVault);
    const tokenVaultBalance = AccountLayout.decode(
      account.data
    ).amount.toString();
    expect(tokenVaultBalance).eq(fundAmount.toString());
    expect(feeVaultState.totalFundedFee.toString()).eq(fundAmount.toString());
  } else {
    console.log(fundFeeRes.meta().logs());
  }

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
        account.data
      ).amount.toString();
      expect(userTokenBalance.toString()).eq(
        feeVaultState.users[i].feeClaimed.toString()
      );
    } else {
      console.log(claimFeeRes.meta().logs());
    }
  }
}
