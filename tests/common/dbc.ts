import {
  AnchorProvider,
  BN,
  IdlAccounts,
  Program,
  Wallet,
} from "@coral-xyz/anchor";

import { DynamicBondingCurve } from "./idl/dynamic_bonding_curve";
import DynamicBondingCurveIDL from "../../idls/dynamic_bonding_curve.json";
import {
  clusterApiUrl,
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import { LiteSVM } from "litesvm";
import {
  getAssociatedTokenAddressSync,
  NATIVE_MINT,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  getFirstKey,
  getSecondKey,
  MAX_SQRT_PRICE,
  MIN_SQRT_PRICE,
} from "./damm_v2";
import Decimal from "decimal.js";
import { sendTransactionOrExpectThrowError } from "./svm";
import { getOrCreateAtA, unwrapSOLInstruction, wrapSOLInstruction } from ".";

export const DBC_PROGRAM_ID = new PublicKey(DynamicBondingCurveIDL.address);
export type VirtualPool = IdlAccounts<DynamicBondingCurve>["virtualPool"];
export type PoolConfig = IdlAccounts<DynamicBondingCurve>["poolConfig"];

export function createDBCProgram() {
  const wallet = new Wallet(Keypair.generate());
  const provider = new AnchorProvider(
    new Connection(clusterApiUrl("devnet")),
    wallet,
    {}
  );
  const program = new Program<DynamicBondingCurve>(
    DynamicBondingCurveIDL as DynamicBondingCurve,
    provider
  );
  return program;
}

export async function createConfig(
  svm: LiteSVM,
  params: CreateConfigParams
): Promise<PublicKey> {
  const { payer, leftoverReceiver, feeClaimer, quoteMint, instructionParams } =
    params;
  const config = Keypair.generate();

  const program = createDBCProgram();

  const transaction = await program.methods
    .createConfig(instructionParams)
    .accountsPartial({
      config: config.publicKey,
      feeClaimer,
      leftoverReceiver,
      quoteMint,
      payer: payer.publicKey,
    })
    .transaction();

  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.sign(payer, config);

  sendTransactionOrExpectThrowError(svm, transaction);

  return config.publicKey;
}

export type CreatePoolToken2022Params = {
  payer: Keypair;
  poolCreator: Keypair;
  quoteMint: PublicKey;
  config: PublicKey;
  instructionParams: {
    name: string;
    symbol: string;
    uri: string;
  };
};

export async function createVirtualPool(
  svm: LiteSVM,
  params: CreatePoolToken2022Params
): Promise<PublicKey> {
  const { payer, quoteMint, config, instructionParams, poolCreator } = params;
  const program = createDBCProgram();
  const poolAuthority = deriveDbcPoolAuthority();
  const baseMintKP = Keypair.generate();
  const pool = derivePoolAddress(config, baseMintKP.publicKey, quoteMint);
  const baseVault = deriveTokenVaultAddress(baseMintKP.publicKey, pool);
  const quoteVault = deriveTokenVaultAddress(quoteMint, pool);
  const transaction = await program.methods
    .initializeVirtualPoolWithToken2022(instructionParams)
    .accountsPartial({
      config,
      baseMint: baseMintKP.publicKey,
      quoteMint,
      pool,
      payer: payer.publicKey,
      creator: poolCreator.publicKey,
      poolAuthority,
      baseVault,
      quoteVault,
      tokenQuoteProgram: TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .transaction();

  transaction.add(
    ComputeBudgetProgram.setComputeUnitLimit({
      units: 400_000,
    })
  );

  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.sign(payer, baseMintKP, poolCreator);

  sendTransactionOrExpectThrowError(svm, transaction);

  return pool;
}

export type SwapParams = {
  config: PublicKey;
  payer: Keypair;
  pool: PublicKey;
  inputTokenMint: PublicKey;
  outputTokenMint: PublicKey;
  amountIn: BN;
  minimumAmountOut: BN;
  referralTokenAccount: PublicKey | null;
};

export async function swap(svm: LiteSVM, params: SwapParams): Promise<void> {
  const {
    config,
    payer,
    pool,
    inputTokenMint,
    outputTokenMint,
    amountIn,
    minimumAmountOut,
    referralTokenAccount,
  } = params;

  const poolAuthority = deriveDbcPoolAuthority();
  let poolState = getVirtualPoolState(svm, pool);
  const program = createDBCProgram();

  const tokenBaseProgram = TOKEN_2022_PROGRAM_ID;

  const isInputBaseMint = inputTokenMint.equals(poolState.baseMint);

  const quoteMint = isInputBaseMint ? outputTokenMint : inputTokenMint;
  const [inputTokenProgram, outputTokenProgram] = isInputBaseMint
    ? [tokenBaseProgram, TOKEN_PROGRAM_ID]
    : [TOKEN_PROGRAM_ID, tokenBaseProgram];

  const preInstructions: TransactionInstruction[] = [];
  const postInstructions: TransactionInstruction[] = [];

  const [inputTokenAccount, outputTokenAccount] = [
    getOrCreateAtA(
      svm,
      payer,
      inputTokenMint,
      payer.publicKey,
      inputTokenProgram
    ),
    getOrCreateAtA(
      svm,
      payer,
      outputTokenMint,
      payer.publicKey,
      outputTokenProgram
    ),
  ];

  if (inputTokenMint.equals(NATIVE_MINT) && !amountIn.isZero()) {
    const wrapSOLIx = wrapSOLInstruction(
      payer.publicKey,
      inputTokenAccount,
      BigInt(amountIn.toString())
    );

    preInstructions.push(...wrapSOLIx);
  }

  if (outputTokenMint.equals(NATIVE_MINT)) {
    const unrapSOLIx = unwrapSOLInstruction(payer.publicKey);

    unrapSOLIx && postInstructions.push(unrapSOLIx);
  }

  const transaction = await program.methods
    .swap({ amountIn, minimumAmountOut })
    .accountsPartial({
      poolAuthority,
      config,
      pool,
      inputTokenAccount,
      outputTokenAccount,
      baseVault: poolState.baseVault,
      quoteVault: poolState.quoteVault,
      baseMint: poolState.baseMint,
      quoteMint,
      payer: payer.publicKey,
      tokenBaseProgram,
      tokenQuoteProgram: TOKEN_PROGRAM_ID,
      referralTokenAccount,
    })
    .remainingAccounts(
      // TODO should check condition to add this in remaning accounts
      [
        {
          isSigner: false,
          isWritable: false,
          pubkey: SYSVAR_INSTRUCTIONS_PUBKEY,
        },
      ]
    )
    .preInstructions(preInstructions)
    .postInstructions(postInstructions)
    .transaction();

  transaction.add(
    ComputeBudgetProgram.setComputeUnitLimit({
      units: 400_000,
    })
  );

  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.sign(payer);

  sendTransactionOrExpectThrowError(svm, transaction);
}

export async function transferCreator(
  svm: LiteSVM,
  virtualPool: PublicKey,
  creator: Keypair,
  newCreator: PublicKey
): Promise<void> {
  const program = createDBCProgram();
  const poolState = getVirtualPoolState(svm, virtualPool);
  const migrationMetadata = deriveMigrationMetadataAddress(virtualPool);
  const transaction = await program.methods
    .transferPoolCreator()
    .accountsPartial({
      virtualPool,
      newCreator,
      config: poolState.config,
      creator: creator.publicKey,
    })
    .remainingAccounts([
      {
        isSigner: false,
        isWritable: false,
        pubkey: migrationMetadata,
      },
    ])
    .transaction();

  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.sign(creator);

  sendTransactionOrExpectThrowError(svm, transaction);
}

export function getVirtualPoolState(
  svm: LiteSVM,
  virtualPool: PublicKey
): VirtualPool {
  const program = createDBCProgram();
  const account = svm.getAccount(virtualPool);
  return program.coder.accounts.decode(
    "virtualPool",
    Buffer.from(account.data)
  );
}

export function getVirtualConfigState(
  svm: LiteSVM,
  virtualPool: PublicKey
): PoolConfig {
  const program = createDBCProgram();
  const account = svm.getAccount(virtualPool);
  return program.coder.accounts.decode("poolConfig", Buffer.from(account.data));
}

export function deriveMigrationMetadataAddress(
  virtual_pool: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("meteora"), virtual_pool.toBuffer()],
    DBC_PROGRAM_ID
  )[0];
}

function derivePoolAddress(
  config: PublicKey,
  tokenAMint: PublicKey,
  tokenBMint: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("pool"),
      config.toBuffer(),
      getFirstKey(tokenAMint, tokenBMint),
      getSecondKey(tokenAMint, tokenBMint),
    ],
    DBC_PROGRAM_ID
  )[0];
}

export function deriveDbcPoolAuthority(): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("pool_authority")],
    DBC_PROGRAM_ID
  )[0];
}

export function deriveDbcEventAuthority() {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("__event_authority")],
    DBC_PROGRAM_ID
  )[0];
}

function deriveTokenVaultAddress(
  tokenMint: PublicKey,
  pool: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("token_vault"), tokenMint.toBuffer(), pool.toBuffer()],
    DBC_PROGRAM_ID
  )[0];
}

////// DBC utils

export function buildDefaultCurve(): ConfigParameters {
  let totalTokenSupply = 1_000_000_000;
  let percentageSupplyOnMigration = 10; // 10%;
  let migrationQuoteThreshold = 300; // 300 sol
  let migrationOption = 1; // damm v2
  let tokenBaseDecimal = 6;
  let tokenQuoteDecimal = 9;
  let creatorTradingFeePercentage = 50;
  let collectFeeMode = 0;

  let migrationBaseSupply = new BN(totalTokenSupply)
    .mul(new BN(percentageSupplyOnMigration))
    .div(new BN(100));

  let totalSupply = new BN(totalTokenSupply).mul(
    new BN(10).pow(new BN(tokenBaseDecimal))
  );
  let migrationQuoteThresholdWithDecimals = new BN(
    migrationQuoteThreshold * 10 ** tokenQuoteDecimal
  );

  let migrationPrice = new Decimal(migrationQuoteThreshold.toString()).div(
    new Decimal(migrationBaseSupply.toString())
  );
  let migrateSqrtPrice = getSqrtPriceFromPrice(
    migrationPrice.toString(),
    tokenBaseDecimal,
    tokenQuoteDecimal
  );

  let migrationBaseAmount = getMigrationBaseToken(
    new BN(migrationQuoteThresholdWithDecimals),
    migrateSqrtPrice,
    migrationOption
  );
  let swapAmount = totalSupply.sub(migrationBaseAmount);

  let { sqrtStartPrice, curve } = getFirstCurve(
    migrateSqrtPrice,
    migrationBaseAmount,
    swapAmount,
    migrationQuoteThresholdWithDecimals
  );

  let totalDynamicSupply = getTotalSupplyFromCurve(
    migrationQuoteThresholdWithDecimals,
    sqrtStartPrice,
    curve,
    migrationOption,
    new BN(0)
  );

  let remainingAmount = totalSupply.sub(totalDynamicSupply);

  let lastLiquidity = getInitialLiquidityFromDeltaBase(
    remainingAmount,
    MAX_SQRT_PRICE,
    migrateSqrtPrice
  );
  if (!lastLiquidity.isZero()) {
    curve.push({
      sqrtPrice: MAX_SQRT_PRICE,
      liquidity: lastLiquidity,
    });
  }

  const instructionParams: ConfigParameters = {
    poolFees: {
      baseFee: {
        cliffFeeNumerator: new BN(2_500_000),
        firstFactor: 0,
        secondFactor: new BN(0),
        thirdFactor: new BN(0),
        baseFeeMode: 0,
      },
      dynamicFee: null,
    },
    activationType: 0,
    collectFeeMode,
    migrationOption,
    tokenType: 1, // token 2022
    tokenDecimal: tokenBaseDecimal,
    migrationQuoteThreshold: migrationQuoteThresholdWithDecimals,
    partnerLpPercentage: 0,
    creatorLpPercentage: 0,
    partnerLockedLpPercentage: 100,
    creatorLockedLpPercentage: 0,
    sqrtStartPrice,
    lockedVesting: {
      amountPerPeriod: new BN(0),
      cliffDurationFromMigrationTime: new BN(0),
      frequency: new BN(0),
      numberOfPeriod: new BN(0),
      cliffUnlockAmount: new BN(0),
    },
    migrationFeeOption: 0,
    tokenSupply: {
      preMigrationTokenSupply: totalSupply,
      postMigrationTokenSupply: totalSupply,
    },
    creatorTradingFeePercentage,
    tokenUpdateAuthority: 0,
    migrationFee: {
      feePercentage: 50,
      creatorFeePercentage: 50,
    },
    migratedPoolFee: {
      collectFeeMode: 0,
      dynamicFee: 0,
      poolFeeBps: 0,
    },
    padding: [],
    curve,
  };
  return instructionParams;
}

type BaseFee = {
  cliffFeeNumerator: BN;
  firstFactor: number;
  secondFactor: BN;
  thirdFactor: BN;
  baseFeeMode: number;
};

type DynamicFee = {
  binStep: number;
  binStepU128: BN;
  filterPeriod: number;
  decayPeriod: number;
  reductionFactor: number;
  maxVolatilityAccumulator: number;
  variableFeeControl: number;
};

type LockedVestingParams = {
  amountPerPeriod: BN;
  cliffDurationFromMigrationTime: BN;
  frequency: BN;
  numberOfPeriod: BN;
  cliffUnlockAmount: BN;
};

type TokenSupplyParams = {
  preMigrationTokenSupply: BN;
  postMigrationTokenSupply: BN;
};

type ConfigParameters = {
  poolFees: {
    baseFee: BaseFee;
    dynamicFee: DynamicFee | null;
  };
  collectFeeMode: number;
  migrationOption: number;
  activationType: number;
  tokenType: number;
  tokenDecimal: number;
  migrationQuoteThreshold: BN;
  partnerLpPercentage: number;
  partnerLockedLpPercentage: number;
  creatorLpPercentage: number;
  creatorLockedLpPercentage: number;
  sqrtStartPrice: BN;
  lockedVesting: LockedVestingParams;
  migrationFeeOption: number;
  tokenSupply: TokenSupplyParams | null;
  creatorTradingFeePercentage: number;
  tokenUpdateAuthority: number;
  migrationFee: {
    feePercentage: number;
    creatorFeePercentage: number;
  };
  migratedPoolFee: {
    poolFeeBps: number;
    collectFeeMode: number;
    dynamicFee: number;
  };
  padding: BN[];
  curve: Array<LiquidityDistributionParameters>;
};

type LiquidityDistributionParameters = {
  sqrtPrice: BN;
  liquidity: BN;
};

export type CreateConfigParams = {
  payer: Keypair;
  leftoverReceiver: PublicKey;
  feeClaimer: PublicKey;
  quoteMint: PublicKey;
  instructionParams: ConfigParameters;
};

const getSqrtPriceFromPrice = (
  price: string,
  tokenADecimal: number,
  tokenBDecimal: number
): BN => {
  const decimalPrice = new Decimal(price);
  const adjustedByDecimals = decimalPrice.div(
    new Decimal(10 ** (tokenADecimal - tokenBDecimal))
  );
  const sqrtValue = Decimal.sqrt(adjustedByDecimals);
  const sqrtValueQ64 = sqrtValue.mul(Decimal.pow(2, 64));

  return new BN(sqrtValueQ64.floor().toFixed());
};

const getInitialLiquidityFromDeltaQuote = (
  quoteAmount: BN,
  sqrtMinPrice: BN,
  sqrtPrice: BN
): BN => {
  let priceDelta = sqrtPrice.sub(sqrtMinPrice);
  quoteAmount = quoteAmount.shln(128);
  let liquidity = quoteAmount.div(priceDelta); // round down
  return liquidity;
};

function getDeltaAmountBase(
  lowerSqrtPrice: BN,
  upperSqrtPrice: BN,
  liquidity: BN
): BN {
  let numerator = liquidity.mul(upperSqrtPrice.sub(lowerSqrtPrice));
  let denominator = lowerSqrtPrice.mul(upperSqrtPrice);
  return numerator.add(denominator).sub(new BN(1)).div(denominator);
}

const getMigrationBaseToken = (
  migrationQuoteThreshold: BN,
  sqrtMigrationPrice: BN,
  migrationOption: number
): BN => {
  if (migrationOption == 0) {
    let price = sqrtMigrationPrice.mul(sqrtMigrationPrice);
    let quote = migrationQuoteThreshold.shln(128);
    let { div, mod } = quote.divmod(price);
    if (!mod.isZero()) {
      div = div.add(new BN(1));
    }
    return div;
  } else if (migrationOption == 1) {
    let liquidity = getInitialLiquidityFromDeltaQuote(
      migrationQuoteThreshold,
      MIN_SQRT_PRICE,
      sqrtMigrationPrice
    );
    // calculate base threshold
    let baseAmount = getDeltaAmountBase(
      sqrtMigrationPrice,
      MAX_SQRT_PRICE,
      liquidity
    );
    return baseAmount;
  } else {
    throw Error("Invalid migration option");
  }
};

const getNextSqrtPriceFromInput = (
  sqrtPrice: BN,
  liquidity: BN,
  amountIn: BN,
  baseForQuote: boolean
): BN => {
  // round to make sure that we don't pass the target price
  if (baseForQuote) {
    return getNextSqrtPriceFromAmountBaseRoundingUp(
      sqrtPrice,
      liquidity,
      amountIn
    );
  } else {
    return getNextSqrtPriceFromAmountQuoteRoundingDown(
      sqrtPrice,
      liquidity,
      amountIn
    );
  }
};

//  √P' = √P * L / (L + Δx * √P)
const getNextSqrtPriceFromAmountBaseRoundingUp = (
  sqrtPrice: BN,
  liquidity: BN,
  amount: BN
): BN => {
  if (amount.isZero()) {
    return sqrtPrice;
  }
  let prod = sqrtPrice.mul(liquidity);
  let denominator = liquidity.add(amount.mul(sqrtPrice));
  let result = prod.add(denominator).sub(new BN(1)).div(denominator);
  return result;
};

/// * `√P' = √P + Δy / L`
///
const getNextSqrtPriceFromAmountQuoteRoundingDown = (
  sqrtPrice: BN,
  liquidity: BN,
  amount: BN
): BN => {
  return sqrtPrice.add(amount.shln(128).div(liquidity));
};

// Δa = L * (1 / √P_lower - 1 / √P_upper) => L = Δa / (1 / √P_lower - 1 / √P_upper)
const getInitialLiquidityFromDeltaBase = (
  baseAmount: BN,
  sqrtMaxPrice: BN,
  sqrtPrice: BN
): BN => {
  let priceDelta = sqrtMaxPrice.sub(sqrtPrice);
  let prod = baseAmount.mul(sqrtMaxPrice).mul(sqrtPrice);
  let liquidity = prod.div(priceDelta); // round down
  return liquidity;
};

const getDeltaAmountQuote = (
  lowerSqrtPrice: BN,
  upperSqrtPrice: BN,
  liquidity: BN,
  round: String
): BN => {
  let detalPrice = upperSqrtPrice.sub(lowerSqrtPrice);
  let prod = liquidity.mul(detalPrice);
  let denominator = new BN(1).shln(128);
  if (round == "U") {
    let result = prod.add(denominator).sub(new BN(1)).div(denominator);
    return result;
  } else if (round == "D") {
    let result = prod.div(denominator);
    return result;
  } else {
    throw Error("Invalid rounding");
  }
};

const getMigrationThresholdPrice = (
  migrationThreshold: BN,
  sqrtStartPrice: BN,
  curve: Array<LiquidityDistributionParameters>
): BN => {
  let nextSqrtPrice = sqrtStartPrice;
  let totalAmount = getDeltaAmountQuote(
    nextSqrtPrice,
    curve[0].sqrtPrice,
    curve[0].liquidity,
    "U"
  );
  if (totalAmount.gt(migrationThreshold)) {
    nextSqrtPrice = getNextSqrtPriceFromInput(
      nextSqrtPrice,
      curve[0].liquidity,
      migrationThreshold,
      false
    );
  } else {
    let amountLeft = migrationThreshold.sub(totalAmount);
    nextSqrtPrice = curve[0].sqrtPrice;
    for (let i = 1; i < curve.length; i++) {
      let maxAmount = getDeltaAmountQuote(
        nextSqrtPrice,
        curve[i].sqrtPrice,
        curve[i].liquidity,
        "U"
      );
      if (maxAmount.gt(amountLeft)) {
        nextSqrtPrice = getNextSqrtPriceFromInput(
          nextSqrtPrice,
          curve[i].liquidity,
          amountLeft,
          false
        );
        amountLeft = new BN(0);
        break;
      } else {
        amountLeft = amountLeft.sub(maxAmount);
        nextSqrtPrice = curve[i].sqrtPrice;
      }
    }
    if (!amountLeft.isZero()) {
      console.log("migrationThreshold: ", migrationThreshold.toString());
      throw Error("Not enough liquidity, amountLeft: " + amountLeft.toString());
    }
  }
  return nextSqrtPrice;
};

function getBaseTokenForSwap(
  sqrtStartPrice: BN,
  sqrtMigrationPrice: BN,
  curve: Array<LiquidityDistributionParameters>
): BN {
  let totalAmount = new BN(0);
  for (let i = 0; i < curve.length; i++) {
    let lowerSqrtPrice = i == 0 ? sqrtStartPrice : curve[i - 1].sqrtPrice;
    if (curve[i].sqrtPrice > sqrtMigrationPrice) {
      let deltaAmount = getDeltaAmountBase(
        lowerSqrtPrice,
        sqrtMigrationPrice,
        curve[i].liquidity
      );
      totalAmount = totalAmount.add(deltaAmount);
      break;
    } else {
      let deltaAmount = getDeltaAmountBase(
        lowerSqrtPrice,
        curve[i].sqrtPrice,
        curve[i].liquidity
      );
      totalAmount = totalAmount.add(deltaAmount);
    }
  }
  return totalAmount;
}

const getSwapAmountWithBuffer = (
  swapBaseAmount: BN,
  sqrtStartPrice: BN,
  curve: Array<LiquidityDistributionParameters>
): BN => {
  let swapAmountBuffer = swapBaseAmount.add(
    swapBaseAmount.mul(new BN(25)).div(new BN(100))
  );
  let maxBaseAmountOnCurve = getBaseTokenForSwap(
    sqrtStartPrice,
    MAX_SQRT_PRICE,
    curve
  );
  return BN.min(swapAmountBuffer, maxBaseAmountOnCurve);
};

const getTotalSupplyFromCurve = (
  migrationQuoteThreshold: BN,
  sqrtStartPrice: BN,
  curve: Array<LiquidityDistributionParameters>,
  migrationOption: number,
  leftOver: BN
): BN => {
  let sqrtMigrationPrice = getMigrationThresholdPrice(
    migrationQuoteThreshold,
    sqrtStartPrice,
    curve
  );
  let swapBaseAmount = getBaseTokenForSwap(
    sqrtStartPrice,
    sqrtMigrationPrice,
    curve
  );
  let swapBaseAmountBuffer = getSwapAmountWithBuffer(
    swapBaseAmount,
    sqrtStartPrice,
    curve
  );
  let migrationBaseAmount = getMigrationBaseToken(
    migrationQuoteThreshold,
    sqrtMigrationPrice,
    migrationOption
  );
  let minimumBaseSupplyWithBuffer = swapBaseAmountBuffer
    .add(migrationBaseAmount)
    .add(leftOver);
  return minimumBaseSupplyWithBuffer;
};

const getLiquidity = (
  baseAmount: BN,
  quoteAmount: BN,
  minSqrtPrice: BN,
  maxSqrtPrice: BN
): BN => {
  let liquidityFromBase = getInitialLiquidityFromDeltaBase(
    baseAmount,
    maxSqrtPrice,
    minSqrtPrice
  );
  let liquidityFromQuote = getInitialLiquidityFromDeltaQuote(
    quoteAmount,
    minSqrtPrice,
    maxSqrtPrice
  );
  return BN.min(liquidityFromBase, liquidityFromQuote);
};

const getFirstCurve = (
  migrationSqrPrice: BN,
  migrationAmount: BN,
  swapAmount: BN,
  migrationQuoteThreshold: BN
) => {
  let sqrtStartPrice = migrationSqrPrice.mul(migrationAmount).div(swapAmount);
  let liquidity = getLiquidity(
    swapAmount,
    migrationQuoteThreshold,
    sqrtStartPrice,
    migrationSqrPrice
  );
  return {
    sqrtStartPrice,
    curve: [
      {
        sqrtPrice: migrationSqrPrice,
        liquidity,
      },
    ],
  };
};
