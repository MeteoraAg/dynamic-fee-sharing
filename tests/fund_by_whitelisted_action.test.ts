import { LiteSVM } from "litesvm";
import { PublicKey, Keypair, Transaction } from "@solana/web3.js";
import CpAmmIDL from "../idls/damm_v2.json";
import { BN } from "bn.js";
import { expect } from "chai";
import {
  AuthorityType,
  createSetAuthorityInstruction,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import {
  generateUsers,
  getTokenBalance,
  sendTransactionOrExpectThrowError,
  startSvm,
} from "./common/svm";
import {
  createToken,
  getDynamicFeeVault,
  getOrCreateAtA,
  loadLocalnetAdmin,
  mintToken,
} from "./common";
import {
  createDammV2Pool,
  dammV2Swap,
  DAMM_V2_PROGRAM_ID,
  getDammV2PoolState,
} from "./common/damm_v2";
import {
  createDynamicFeeVaultPda,
  createWhitelistedAction,
  fundByWhitelistedActionDammV2,
  claimFee,
} from "./common/dfs";

const SINGLE_TOKEN_SENTINEL = 255;

describe("Fund by whitelisted action (damm v2)", () => {
  let svm: LiteSVM;
  let admin: Keypair;
  let creator: Keypair;
  let vaultOwner: Keypair;
  let shareHolder: Keypair;
  let tokenAMint: PublicKey;
  let tokenBMint: PublicKey;
  let dammV2Pool: PublicKey;
  let position: PublicKey;
  let positionNftAccount: PublicKey;
  let poolTokenAMint: PublicKey;
  let poolTokenBMint: PublicKey;

  const claimPositionFeeDisc = CpAmmIDL.instructions.find(
    (instruction) => instruction.name === "claim_position_fee"
  ).discriminator;

  const users = () => ({
    padding: [],
    users: [
      { address: shareHolder.publicKey, share: 100 },
      { address: PublicKey.unique(), share: 100 },
    ],
  });

  const assignPositionToVault = (feeVault: PublicKey) => {
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
  };

  const accrueFeesBothDirections = async () => {
    await dammV2Swap(svm, {
      payer: creator,
      pool: dammV2Pool,
      inputTokenMint: poolTokenAMint,
      outputTokenMint: poolTokenBMint,
      amountIn: new BN(10000 * 10 ** 6),
      minimumAmountOut: new BN(0),
    });
    await dammV2Swap(svm, {
      payer: creator,
      pool: dammV2Pool,
      inputTokenMint: poolTokenBMint,
      outputTokenMint: poolTokenAMint,
      amountIn: new BN(10000 * 10 ** 6),
      minimumAmountOut: new BN(0),
    });
  };

  // collectFeeMode 0 = fees in both tokens, 1 = only token B
  const setupPool = async (collectFeeMode: number) => {
    const res = await createDammV2Pool(
      svm,
      creator,
      tokenAMint,
      tokenBMint,
      collectFeeMode
    );
    dammV2Pool = res.pool;
    position = res.position;
    positionNftAccount = res.positionNftAccount;

    // pool sorts the mints; token_0 == pool token A, token_1 == pool token B
    const poolState = getDammV2PoolState(svm, dammV2Pool);
    poolTokenAMint = poolState.tokenAMint;
    poolTokenBMint = poolState.tokenBMint;
  };

  beforeEach(async () => {
    svm = startSvm();
    [creator, vaultOwner, shareHolder] = generateUsers(svm, 3);
    admin = loadLocalnetAdmin(svm);

    tokenAMint = createToken(svm, admin, admin.publicKey, null);
    tokenBMint = createToken(svm, admin, admin.publicKey, null);
    mintToken(svm, admin, tokenAMint, admin, creator.publicKey);
    mintToken(svm, admin, tokenBMint, admin, creator.publicKey);
  });

  it("funds both tokens of a two-token vault", async () => {
    await setupPool(0);

    const { feeVault, token0Vault, token1Vault } =
      await createDynamicFeeVaultPda(
        svm,
        admin,
        vaultOwner.publicKey,
        poolTokenAMint,
        users(),
        poolTokenBMint
      );

    assignPositionToVault(feeVault);

    // token A -> vault token_0 (index 3), token B -> vault token_1 (index 4)
    const whitelistedAction = await createWhitelistedAction(
      svm,
      admin,
      DAMM_V2_PROGRAM_ID,
      claimPositionFeeDisc,
      3,
      4
    );

    await accrueFeesBothDirections();

    await fundByWhitelistedActionDammV2(
      svm,
      shareHolder,
      feeVault,
      token0Vault,
      token1Vault,
      whitelistedAction,
      dammV2Pool,
      position,
      positionNftAccount,
      token0Vault,
      token1Vault
    );

    const vault = getDynamicFeeVault(svm, feeVault);
    expect(vault.fixed.totalFundedFeeToken0.gt(new BN(0))).eq(true);
    expect(vault.fixed.totalFundedFeeToken1.gt(new BN(0))).eq(true);
    expect(vault.fixed.feePerShareToken0.gt(new BN(0))).eq(true);
    expect(vault.fixed.feePerShareToken1.gt(new BN(0))).eq(true);

    // a share holder can claim the funded token_0
    const userTokenVault = await claimFee(
      svm,
      shareHolder,
      feeVault,
      token0Vault,
      poolTokenAMint,
      0
    );
    expect(Number(getTokenBalance(svm, userTokenVault))).gt(0);
  });

  it("funds only token_0 of a single-token vault (token_1 sentinel)", async () => {
    await setupPool(1);

    const { feeVault, token0Vault } = await createDynamicFeeVaultPda(
      svm,
      admin,
      vaultOwner.publicKey,
      poolTokenBMint,
      users()
    );

    assignPositionToVault(feeVault);

    // single-token action: token B -> vault token_0 (index 4), token A -> owner account
    const whitelistedAction = await createWhitelistedAction(
      svm,
      admin,
      DAMM_V2_PROGRAM_ID,
      claimPositionFeeDisc,
      4,
      SINGLE_TOKEN_SENTINEL
    );

    const ownerTokenAAccount = getOrCreateAtA(
      svm,
      creator,
      poolTokenAMint,
      creator.publicKey
    );

    await accrueFeesBothDirections();

    await fundByWhitelistedActionDammV2(
      svm,
      shareHolder,
      feeVault,
      token0Vault,
      null,
      whitelistedAction,
      dammV2Pool,
      position,
      positionNftAccount,
      ownerTokenAAccount,
      token0Vault
    );

    const vault = getDynamicFeeVault(svm, feeVault);
    expect(vault.fixed.totalFundedFeeToken0.gt(new BN(0))).eq(true);
    expect(vault.fixed.feePerShareToken0.gt(new BN(0))).eq(true);
    // second token slot never funded
    expect(vault.fixed.totalFundedFeeToken1.eq(new BN(0))).eq(true);
    expect(vault.fixed.feePerShareToken1.eq(new BN(0))).eq(true);
  });
});
