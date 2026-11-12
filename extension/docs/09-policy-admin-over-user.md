# Policy: Administrator Over User

An extension deployed inside an enterprise is not only used by the person who
installed it. The administrator publishes a policy; the user may have
preferences; the code has defaults. **The order of precedence is fixed:
administrator, then user, then default.**

## 1. The three sources

| Source         | Transport                                      | Written by                         | Shape                                                                                                      |
| -------------- | ---------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Managed policy | `chrome.storage.managed`                       | The administrator, through the OS  | `AllowedHosts`, `BlockedHosts`, `FeatureFlags`, `KillSwitch`, `AllowUserSettings`                          |
| User settings  | `chrome.storage.local` under `policy:settings` | The user, through the options page | The same keys                                                                                              |
| Defaults       | `src/policy/resolver.ts`                       | The code                           | `allowedHosts: []`, `blockedHosts: []`, `featureFlags: {}`, `killSwitch: false`, `allowUserSettings: true` |

The two sources use the **same Chrome policy key names**, so one precedence
rule applies to every field.

## 2. The resolver

`resolvePolicy(managed, user)` is pure — no browser, no storage — so the
precedence can be tested directly (`tests/notes/frontend/unit/policy/policy-resolver.test.ts`):

```ts
const settings: PolicySettings = {
  allowedHosts: managed.AllowedHosts ?? user.AllowedHosts ?? DEFAULT_POLICY.allowedHosts,
  blockedHosts: managed.BlockedHosts ?? user.BlockedHosts ?? DEFAULT_POLICY.blockedHosts,
  featureFlags: { ...user.FeatureFlags, ...managed.FeatureFlags },
  killSwitch: managed.KillSwitch ?? user.KillSwitch ?? DEFAULT_POLICY.killSwitch,
  allowUserSettings:
    managed.AllowUserSettings ?? user.AllowUserSettings ?? DEFAULT_POLICY.allowUserSettings,
};
```

Two details matter:

- **Feature flags merge**, they do not replace: a flag the administrator set
  wins, and the user keeps the flags the administrator did not mention.
- **Locked settings are reported.** The resolver returns `locked[]`, the
  list of settings the administrator published. The options page renders them
  as "set by the administrator" and never offers them as editable.

## 3. Failing closed

The reader (`src/policy/reader.ts`) validates both sources and **ignores
what does not validate**:

```ts
const parsed = ManagedPolicySchema.safeParse(await managed.read());
if (!parsed.success) {
  logger.log("warn", "the administrator policy is invalid; ignored", { ... });
  return {};
}
```

An invalid managed policy therefore falls back to the user settings and then
to the defaults — never to "everything is allowed".

The URL check is closed by construction:

```ts
export function isUrlAllowed(settings: PolicySettings, url: string): boolean {
  if (settings.killSwitch) return false;
  if (settings.blockedHosts.some((pattern) => matchesPattern(pattern, url))) return false;
  return settings.allowedHosts.some((pattern) => matchesPattern(pattern, url));
}
```

- The kill switch stops everything.
- A blocked host is refused.
- **An empty allowlist allows nothing.** The default policy allows no URL at
  all; a feature must opt in.

## 4. The contract

`src/contracts/policy.ts` holds two schemas with different strictness, and
the difference is deliberate:

| Schema                | Strictness               | Why                                                                                                        |
| --------------------- | ------------------------ | ---------------------------------------------------------------------------------------------------------- |
| `ManagedPolicySchema` | Unknown keys are dropped | The administrator may publish keys a later build adds; an unknown key must not invalidate the whole policy |
| `UserPolicySchema`    | Strict                   | The user settings are written by the extension; an unexpected key is a bug                                 |

`PolicySettingsSchema` is strict and camelCase: it is the _resolved_ shape
the code works with, and it never carries a Chrome policy key name.

## 5. Worked example

The options page resolves the policy and renders it read-only:

```ts
term.textContent = policy.locked.includes(setting.key)
  ? `${setting.label} (set by the administrator)`
  : setting.label;
```

The page reads the managed area defensively: an extension deployed without a
managed schema has no managed storage, and an empty policy is the honest
answer, not a failure.

## 6. Known gap: the schema is not wired

`managed-schema.json` ships in the kit, but `src/manifest.ts` does not
reference it (`storage.managed_schema` is absent), so `chrome.storage.managed`
is empty in a deployed build and the administrator policy never arrives. The
wiring step is described in the [Admin Guide](admin-guide.md). Until it is
done, the resolver is exercised by tests only.

## Do / Don't

| Do                                            | Don't                                            |
| --------------------------------------------- | ------------------------------------------------ |
| Resolve managed over user for every field     | Let a user setting override an administrator one |
| Ignore an invalid policy and fail closed      | Accept a partially invalid policy                |
| Render the locked settings read-only          | Offer the user an editable locked field          |
| Treat an empty allowlist as "nothing allowed" | Treat it as "everything allowed"                 |

## See also

- [Admin Guide](admin-guide.md)
- [Storage & Migrations](08-storage-and-migrations.md)
- [Data Flows](data-flows.md)
