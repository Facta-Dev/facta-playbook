// <app> — the policy reader.
//
// Reads the administrator policy and the user settings, then resolves them.
// A value that does not validate is logged and ignored: the resolver falls
// back to the next source down, so the extension fails closed, never open.

import {
  ManagedPolicySchema,
  UserPolicySchema,
  type ManagedPolicy,
  type UserPolicy,
} from "../contracts/policy";
import { type ManagedArea, type StorageArea } from "../platform/storage";
import { type Logger } from "../telemetry/logger";
import { isUrlAllowed, resolvePolicy, type EffectivePolicy } from "./resolver";

/** The storage key of the user settings. */
export const USER_POLICY_KEY = "policy:settings";

export interface PolicyReader {
  /** The effective policy. */
  read(): Promise<EffectivePolicy>;
  /** True when the extension may run on the url. */
  allows(url: string): Promise<boolean>;
}

export interface PolicyReaderOptions {
  readonly local: StorageArea;
  readonly managed: ManagedArea;
  readonly logger: Logger;
}

export function createPolicyReader(options: PolicyReaderOptions): PolicyReader {
  const { local, managed, logger } = options;

  const readManaged = async (): Promise<ManagedPolicy> => {
    const parsed = ManagedPolicySchema.safeParse(await managed.read());
    if (!parsed.success) {
      logger.log("warn", "the administrator policy is invalid; ignored", {
        issue: parsed.error.issues[0]?.code,
      });
      return {};
    }
    return parsed.data;
  };

  const readUser = async (): Promise<UserPolicy> => {
    const parsed = UserPolicySchema.safeParse((await local.get<unknown>(USER_POLICY_KEY)) ?? {});
    if (!parsed.success) {
      logger.log("warn", "the user settings are invalid; ignored", {
        issue: parsed.error.issues[0]?.code,
      });
      return {};
    }
    return parsed.data;
  };

  const read = async (): Promise<EffectivePolicy> =>
    resolvePolicy(await readManaged(), await readUser());

  return {
    read,
    allows: async (url) => isUrlAllowed((await read()).settings, url),
  };
}
