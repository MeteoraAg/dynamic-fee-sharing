import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";

import {
  AccountInfoBytes,
  FailedTransactionMetadata,
  LiteSVM,
  TransactionMetadata,
} from "litesvm";
import { DYNAMIC_FEE_SHARING_PROGRAM_ID } from "./index";
import { expect } from "chai";
import BN from "bn.js";
import { DAMM_V2_PROGRAM_ID } from "./damm_v2";
import { AccountLayout } from "@solana/spl-token";
import { DBC_PROGRAM_ID } from "./dbc";

export function startSvm() {
  const svm = new LiteSVM();
  svm.addProgramFromFile(
    new PublicKey(DYNAMIC_FEE_SHARING_PROGRAM_ID),
    "./target/deploy/dynamic_fee_sharing.so"
  );

  svm.addProgramFromFile(
    new PublicKey(DAMM_V2_PROGRAM_ID),
    "./tests/fixtures/damm_v2.so"
  );

  svm.addProgramFromFile(
    new PublicKey(DBC_PROGRAM_ID),
    "./tests/fixtures/dynamic_bonding_curve.so"
  );

  const accountInfo: AccountInfoBytes = {
    data: new Uint8Array(),
    executable: false,
    lamports: 1200626308,
    owner: SystemProgram.programId,
  };

  svm.setAccount(
    new PublicKey("4EWqcx3aNZmMetCnxwLYwyNjan6XLGp3Ca2W316vrSjv"),
    accountInfo
  );

  return svm;
}

export function sendTransactionOrExpectThrowError(
  svm: LiteSVM,
  transaction: Transaction,
  logging = false,
  errorCode?: number
) {
  const result = svm.sendTransaction(transaction);
  if (logging) {
    if (result instanceof TransactionMetadata) {
      console.log(result.logs());
    } else {
      console.log(result.meta().logs());
    }
  }
  if (errorCode) {
    expectThrowsErrorCode(result, errorCode);
  } else {
    expect(result).instanceOf(TransactionMetadata);
  }

  return result
}

export function generateUsers(svm: LiteSVM, numberOfUsers: number): Keypair[] {
  const res = [];
  for (let i = 0; i < numberOfUsers; i++) {
    const user = Keypair.generate();
    svm.airdrop(user.publicKey, BigInt(LAMPORTS_PER_SOL));
    res.push(user);
  }

  return res;
}

export function expectThrowsErrorCode(
  response: TransactionMetadata | FailedTransactionMetadata,
  errorCode: number
) {
  if (response instanceof FailedTransactionMetadata) {
    const message = response.err().toString();

    if (!message.toString().includes(errorCode.toString())) {
      throw new Error(
        `Unexpected error: ${message}. Expected error: ${errorCode}`
      );
    }

    return;
  } else {
    throw new Error("Expected an error but didn't get one");
  }
}

export function warpToTimestamp(svm: LiteSVM, timestamp: BN) {
  let clock = svm.getClock();
  clock.unixTimestamp = BigInt(timestamp.toString());
  svm.setClock(clock);
}

export function getTokenBalance(svm: LiteSVM, ataAccount: PublicKey): BN {
  const account = svm.getAccount(ataAccount);
  return new BN(AccountLayout.decode(account.data).amount.toString());
}
