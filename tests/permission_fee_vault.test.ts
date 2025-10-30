import { LiteSVM } from "litesvm";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  createProgram,
  createToken,
  DynamicFeeSharingProgram,
  generateUsers,
  getFeeVault,
  InitializeFeeVaultParameters,
  mintToken,
  TOKEN_DECIMALS,
} from "./common";
import { BN } from "bn.js";

import DynamicFeeSharingIDL from "../target/idl/dynamic_fee_sharing.json";
import {
  closePermissionFeeVault,
  createPermissionFeeVault,
  fundFee,
} from "./common/dfs";
import { getTokenBalance } from "./common/svm";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { expect } from "chai";

describe("Permission fee vault", () => {
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

    await fullFlow(svm, admin, funder, vaultOwner.publicKey, tokenMint, params);
  });
});

async function fullFlow(
  svm: LiteSVM,
  admin: Keypair,
  funder: Keypair,
  vaultOwner: PublicKey,
  tokenMint: PublicKey,
  params: InitializeFeeVaultParameters
) {
  const { feeVault } = await createPermissionFeeVault(
    svm,
    admin,
    vaultOwner,
    tokenMint,
    params
  );

  const fundAmount = new BN(100_000 * 10 ** TOKEN_DECIMALS);

  console.log("fund fee vault");

  await fundFee(svm, funder, feeVault, fundAmount);

  console.log("close fee vault");
  const feeVaultState = getFeeVault(svm, feeVault);
  const preVaultBalance = getTokenBalance(svm, feeVaultState.tokenVault);
  const adminPreBalance = getTokenBalance(
    svm,
    getAssociatedTokenAddressSync(feeVaultState.tokenMint, admin.publicKey)
  );

  await closePermissionFeeVault(svm, admin, feeVault);

  const adminPostBalance = getTokenBalance(
    svm,
    getAssociatedTokenAddressSync(feeVaultState.tokenMint, admin.publicKey)
  );
  expect(adminPostBalance.sub(adminPreBalance).toString()).eq(
    preVaultBalance.toString()
  );
}
