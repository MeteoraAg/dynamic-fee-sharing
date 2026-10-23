import { TransactionMetadata } from "litesvm";
import { LiteSVM } from "./common/litesvmCompat";
import {
  PublicKey,
  Keypair,
  LAMPORTS_PER_SOL,
  Transaction,
} from "@solana/web3.js";
import {
  createProgram,
  createToken,
  deriveFeeVaultAuthorityAddress,
  deriveTokenVaultAddress,
  expectThrowsErrorCode,
  getFeeVault,
  getOrCreateAtA,
  getProgramErrorCodeHexString,
  InitializeFeeVaultParameters,
  mintToken,
  TOKEN_DECIMALS,
} from "./common";
import { getTokenBalance } from "./common/svm";
import { BN } from "@anchor-lang/core";
import {
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";

import DynamicFeeSharingIDL from "../target/idl/dynamic_fee_sharing.json";

describe("Update user address", () => {
  const program = createProgram();
  let svm: LiteSVM;
  let admin: Keypair;
  let funder: Keypair;
  let tokenMint: PublicKey;
  let users: Keypair[];
  let feeVault: PublicKey;
  let tokenVault: PublicKey;

  function send(tx: Transaction, signers: Keypair[]) {
    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(...signers);
    const result = svm.sendTransaction(tx);
    svm.expireBlockhash();
    return result;
  }

  async function fundFee(amount: BN) {
    const tx = await program.methods
      .fundFee(amount)
      .accountsPartial({
        feeVault,
        tokenVault,
        tokenMint,
        fundTokenVault: getAssociatedTokenAddressSync(
          tokenMint,
          funder.publicKey
        ),
        funder: funder.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();
    expect(send(tx, [funder])).instanceOf(TransactionMetadata);
  }

  async function claimFee(index: number, signer: Keypair) {
    const userTokenVault = getOrCreateAtA(
      svm,
      signer,
      tokenMint,
      signer.publicKey
    );
    const tx = await program.methods
      .claimFee(index)
      .accountsPartial({
        feeVault,
        tokenMint,
        tokenVault,
        userTokenVault,
        user: signer.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();
    return { result: send(tx, [signer]), userTokenVault };
  }

  async function updateUserAddress(
    index: number,
    signer: Keypair,
    newAddress: PublicKey
  ) {
    const tx = await program.methods
      .updateUserAddress(index, newAddress)
      .accountsPartial({ feeVault, user: signer.publicKey })
      .transaction();
    return send(tx, [signer]);
  }

  beforeEach(async () => {
    svm = new LiteSVM();
    svm.addProgramFromFile(
      new PublicKey(DynamicFeeSharingIDL.address),
      "./target/deploy/dynamic_fee_sharing.so"
    );

    admin = Keypair.generate();
    funder = Keypair.generate();
    users = [Keypair.generate(), Keypair.generate()];
    for (const kp of [admin, funder, ...users]) {
      svm.airdrop(kp.publicKey, BigInt(LAMPORTS_PER_SOL));
    }

    tokenMint = createToken(svm, admin, admin.publicKey, null);
    mintToken(svm, admin, tokenMint, admin, funder.publicKey);

    const params: InitializeFeeVaultParameters = {
      padding: new Array(8).fill(new BN(0)), // [u64; 8]
      users: users.map((user) => ({ address: user.publicKey, share: 1000 })),
    };
    const feeVaultKp = Keypair.generate();
    feeVault = feeVaultKp.publicKey;
    tokenVault = deriveTokenVaultAddress(feeVault);
    const tx = await program.methods
      .initializeFeeVault(params)
      .accountsPartial({
        feeVault,
        feeVaultAuthority: deriveFeeVaultAuthorityAddress(),
        tokenVault,
        tokenMint,
        owner: admin.publicKey,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();
    expect(send(tx, [admin, feeVaultKp])).instanceOf(TransactionMetadata);
  });

  it("moves the share and the pending fee to the new address", async () => {
    const newOwner = Keypair.generate();
    svm.airdrop(newOwner.publicKey, BigInt(LAMPORTS_PER_SOL));
    const fundAmount = new BN(1_000 * 10 ** TOKEN_DECIMALS);
    await fundFee(fundAmount);

    expect(await updateUserAddress(0, users[0], newOwner.publicKey)).instanceOf(
      TransactionMetadata
    );

    const user = getFeeVault(svm, feeVault).users[0];
    expect(user.address.equals(newOwner.publicKey)).to.be.true;
    expect(user.share).eq(1000);
    expect(user.feeClaimed.isZero()).to.be.true;

    // the old address cannot claim anymore
    const invalidUserAddress =
      getProgramErrorCodeHexString("InvalidUserAddress");
    expectThrowsErrorCode(
      (await claimFee(0, users[0])).result,
      invalidUserAddress
    );

    // the new address claims the fee funded before the update: half of it
    const { result, userTokenVault } = await claimFee(0, newOwner);
    expect(result).instanceOf(TransactionMetadata);
    expect(getTokenBalance(svm, userTokenVault).toString()).eq(
      fundAmount.divn(2).toString()
    );

    // the other user is untouched
    expect(
      getFeeVault(svm, feeVault).users[1].address.equals(users[1].publicKey)
    ).to.be.true;
  });

  it("rejects a signer that is not the user at the index", async () => {
    const invalidUserAddress =
      getProgramErrorCodeHexString("InvalidUserAddress");
    // another share holder cannot move someone else's slot
    expectThrowsErrorCode(
      await updateUserAddress(0, users[1], Keypair.generate().publicKey),
      invalidUserAddress
    );
    expect(
      getFeeVault(svm, feeVault).users[0].address.equals(users[0].publicKey)
    ).to.be.true;
  });

  it("rejects the default or the same address, and an invalid index", async () => {
    const invalidUserAddress =
      getProgramErrorCodeHexString("InvalidUserAddress");
    expectThrowsErrorCode(
      await updateUserAddress(0, users[0], PublicKey.default),
      invalidUserAddress
    );
    expectThrowsErrorCode(
      await updateUserAddress(0, users[0], users[0].publicKey),
      invalidUserAddress
    );
    expectThrowsErrorCode(
      await updateUserAddress(255, users[0], Keypair.generate().publicKey),
      getProgramErrorCodeHexString("InvalidUserIndex")
    );
  });
});
