import { LiteSVM } from "litesvm";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  createToken,
  generateUsers,
  getDynamicFeeVault,
  getDynamicFeeVaultUsers,
  InitializeFeeVaultParameters,
  mintToken,
  TOKEN_DECIMALS,
} from "./common";
import {
  createDynamicFeeVault,
  createDynamicFeeVaultPda,
  fundFee,
  claimFee,
} from "./common/dfs";
import { getTokenBalance } from "./common/svm";
import BN from "bn.js";
import { expect } from "chai";

import DynamicFeeSharingIDL from "../target/idl/dynamic_fee_sharing.json";

describe("DynamicFeeVault", () => {
  let svm: LiteSVM;
  let admin: Keypair;
  let funder: Keypair;
  let vaultOwner: Keypair;
  let token0Mint: PublicKey;

  beforeEach(async () => {
    svm = new LiteSVM();
    svm.addProgramFromFile(
      new PublicKey(DynamicFeeSharingIDL.address),
      "./target/deploy/dynamic_fee_sharing.so"
    );

    admin = Keypair.generate();
    vaultOwner = Keypair.generate();
    funder = Keypair.generate();

    svm.airdrop(admin.publicKey, BigInt(LAMPORTS_PER_SOL));
    svm.airdrop(vaultOwner.publicKey, BigInt(LAMPORTS_PER_SOL));
    svm.airdrop(funder.publicKey, BigInt(LAMPORTS_PER_SOL));

    token0Mint = createToken(svm, admin, admin.publicKey, null);
    mintToken(svm, admin, token0Mint, admin, funder.publicKey);
  });

  it("Non-PDA variant", async () => {
    const users = generateUsers(svm, 6); // 6 users, beyond the fixed FeeVault cap of 5
    const params = makeParams(users);

    const { feeVault, token0Vault } = await createDynamicFeeVault(
      svm,
      admin,
      vaultOwner.publicKey,
      token0Mint,
      params
    );

    await fullFlow(svm, funder, users, vaultOwner.publicKey, token0Mint, {
      feeVault,
      token0Vault,
      params,
    });
  });

  it("PDA variant", async () => {
    const users = generateUsers(svm, 6);
    const params = makeParams(users);

    const { feeVault, token0Vault } = await createDynamicFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      token0Mint,
      params
    );

    await fullFlow(svm, funder, users, vaultOwner.publicKey, token0Mint, {
      feeVault,
      token0Vault,
      params,
    });
  });
});

function makeParams(users: Keypair[]): InitializeFeeVaultParameters {
  return {
    padding: [],
    users: users.map((u) => ({ address: u.publicKey, share: 1000 })),
  };
}

async function fullFlow(
  svm: LiteSVM,
  funder: Keypair,
  users: Keypair[],
  vaultOwner: PublicKey,
  token0Mint: PublicKey,
  vault: {
    feeVault: PublicKey;
    token0Vault: PublicKey;
    params: InitializeFeeVaultParameters;
  }
) {
  const { feeVault, token0Vault, params } = vault;

  const header = getDynamicFeeVault(svm, feeVault);
  const totalShare = params.users.reduce(
    (a, b) => a.add(new BN(b.share)),
    new BN(0)
  );

  expect(header.fixed.owner.toString()).eq(vaultOwner.toString());
  expect(header.fixed.token0Mint.toString()).eq(token0Mint.toString());
  expect(header.fixed.token0Vault.toString()).eq(token0Vault.toString());
  expect(header.fixed.totalShare).eq(totalShare.toNumber());
  expect(header.fixed.totalFundedFeeToken0.toNumber()).eq(0);
  expect(getDynamicFeeVaultUsers(svm, feeVault).length).eq(params.users.length);

  const fundAmount = new BN(100_000 * 10 ** TOKEN_DECIMALS);
  await fundFee(svm, funder, feeVault, token0Vault, token0Mint, fundAmount);

  expect(getTokenBalance(svm, token0Vault).toString()).eq(
    fundAmount.toString()
  );
  expect(
    getDynamicFeeVault(svm, feeVault).fixed.totalFundedFeeToken0.toString()
  ).eq(fundAmount.toString());

  for (let i = 0; i < users.length; i++) {
    const userTokenVault = await claimFee(
      svm,
      users[i],
      feeVault,
      token0Vault,
      token0Mint,
      i
    );

    const onChainUser = getDynamicFeeVaultUsers(svm, feeVault)[i];
    expect(getTokenBalance(svm, userTokenVault).toString()).eq(
      onChainUser.feeClaimed.toString()
    );
  }
}
