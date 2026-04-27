import { LiteSVM } from "litesvm";
import { PublicKey, Keypair, Transaction } from "@solana/web3.js";
import {
  generateUsers,
  sendTransactionOrExpectThrowError,
  startSvm,
} from "./common/svm";
import {
  createToken,
  getOrCreateAtA,
  getProgramErrorCodeHexString,
  mintToken,
} from "./common";
import { createDammV2Pool } from "./common/damm_v2";
import { createFeeVaultPda, reclaimDammV2Position } from "./common/dfs";
import { expect } from "chai";
import {
  AccountLayout,
  AuthorityType,
  createSetAuthorityInstruction,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";

describe("Reclaim damm v2 position", () => {
  let svm: LiteSVM;
  let admin: Keypair;
  let creator: Keypair;
  let vaultOwner: Keypair;
  let tokenAMint: PublicKey;
  let tokenBMint: PublicKey;
  let positionNftAccount: PublicKey;
  let shareHolder: Keypair;
  let user: Keypair;

  beforeEach(async () => {
    svm = startSvm();
    [admin, creator, vaultOwner, shareHolder] = generateUsers(svm, 4);
    tokenAMint = createToken(
      svm,
      admin,
      admin.publicKey,
      null,
      TOKEN_2022_PROGRAM_ID
    );
    tokenBMint = createToken(
      svm,
      admin,
      admin.publicKey,
      null,
      TOKEN_2022_PROGRAM_ID
    );

    mintToken(svm, admin, tokenAMint, admin, creator.publicKey);
    mintToken(svm, admin, tokenBMint, admin, creator.publicKey);

    const createDmmV2PoolRes = await createDammV2Pool(
      svm,
      creator,
      tokenAMint,
      tokenBMint
    );
    positionNftAccount = createDmmV2PoolRes.positionNftAccount;
    user = Keypair.generate();
  });

  async function createVault(): Promise<PublicKey> {
    const { feeVault } = await createFeeVaultPda(
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
    return feeVault;
  }

  function transferPositionNftOwnerToVault(feeVault: PublicKey) {
    const setAuthorityIx = createSetAuthorityInstruction(
      positionNftAccount,
      creator.publicKey,
      AuthorityType.AccountOwner,
      feeVault,
      [],
      TOKEN_2022_PROGRAM_ID
    );
    const tx = new Transaction().add(setAuthorityIx);
    tx.recentBlockhash = svm.latestBlockhash();
    tx.sign(creator);
    sendTransactionOrExpectThrowError(svm, tx);
  }

  it("Reclaim damm v2 position", async () => {
    const feeVault = await createVault();
    transferPositionNftOwnerToVault(feeVault);

    const preAccount = svm.getAccount(positionNftAccount);
    const preOwner = AccountLayout.decode(preAccount.data).owner;
    expect(preOwner.equals(feeVault)).to.be.true;

    await reclaimDammV2Position(
      svm,
      vaultOwner,
      feeVault,
      positionNftAccount,
      user.publicKey
    );

    const postAccount = svm.getAccount(positionNftAccount);
    const postOwner = AccountLayout.decode(postAccount.data).owner;
    expect(postOwner.equals(user.publicKey)).to.be.true;
  });

  it("Fails when caller is not the fee vault owner", async () => {
    const feeVault = await createVault();
    transferPositionNftOwnerToVault(feeVault);

    const errorCode = getProgramErrorCodeHexString("Unauthorized");
    await reclaimDammV2Position(
      svm,
      shareHolder,
      feeVault,
      positionNftAccount,
      user.publicKey,
      errorCode
    );
  });

  it("Fails when position_nft_account is not derived from damm v2", async () => {
    const feeVault = await createVault();

    const randomTokenAccount = getOrCreateAtA(
      svm,
      admin,
      tokenAMint,
      feeVault,
      TOKEN_2022_PROGRAM_ID
    );

    const errorCode = getProgramErrorCodeHexString("InvalidAction");
    await reclaimDammV2Position(
      svm,
      vaultOwner,
      feeVault,
      randomTokenAccount,
      user.publicKey,
      errorCode
    );
  });

  it("Fails when position_nft_account is not owned by the fee vault", async () => {
    const feeVault = await createVault();

    const errorCode = getProgramErrorCodeHexString("Unauthorized");
    await reclaimDammV2Position(
      svm,
      vaultOwner,
      feeVault,
      positionNftAccount,
      user.publicKey,
      errorCode
    );
  });
});
