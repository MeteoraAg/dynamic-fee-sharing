import { LiteSVM, TransactionMetadata } from "litesvm";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  createProgram,
  createToken,
  deriveFeeVaultAuthorityAddress,
  deriveTokenVaultAddress,
  DynamicFeeSharingProgram,
  getFeeVault,
  getOrCreateAtA,
  InitializeFeeVaultParameters,
  mintToken,
  TOKEN_DECIMALS,
  UserShare,
} from "./common";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { BN } from "bn.js";
import {
  AccountLayout,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { expect } from "chai";

import DynamicFeeSharingIDL from "../target/idl/dynamic_fee_sharing.json";

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

  it("Full flow", async () => {
    const users: UserShare[] = [
      {
        address: user.publicKey,
        share: new BN(1000),
      },
    ];

    const params: InitializeFeeVaultParameters = {
      padding: [],
      users,
    };

    await fullFlow(
      svm,
      admin,
      funder,
      user,
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
  user: Keypair,
  vaultOwner: PublicKey,
  tokenMint: PublicKey,
  params: InitializeFeeVaultParameters
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
      owner: vaultOwner,
      payer: admin.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .transaction();

  tx.recentBlockhash = svm.latestBlockhash();
  tx.sign(admin, feeVault);

  const sendRes = svm.sendTransaction(tx);

  if (sendRes instanceof TransactionMetadata) {
    const feeVaultState = getFeeVault(svm, feeVault.publicKey);
    expect(feeVaultState.owner.toString()).eq(vaultOwner.toString());
    expect(feeVaultState.tokenMint.toString()).eq(tokenMint.toString());
    expect(feeVaultState.tokenVault.toString()).eq(tokenVault.toString());
    const totalShare = params.users.reduce((a, b) => a.add(b.share), new BN(0));
    expect(feeVaultState.totalShare.toNumber()).eq(totalShare.toNumber());
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
      feeVault: feeVault.publicKey,
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
    const feeVaultState = getFeeVault(svm, feeVault.publicKey);
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

  const userIndex = 0;
  const userTokenVault = getOrCreateAtA(svm, user, tokenMint, user.publicKey);
  const claimFeeTx = await program.methods
    .claimFee(userIndex)
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
      account.data
    ).amount.toString();
    expect(userTokenBalance.toString()).eq(
      feeVaultState.users[0].feeClaimed.toString()
    );
  } else {
    console.log(claimFeeRes.meta().logs());
  }
}
