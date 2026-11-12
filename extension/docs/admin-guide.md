# Admin Guide: Deploying Managed Policy

How an enterprise administrator pushes a policy to the extension: what has to
be in the build, what has to be on the machine, and how to check that it
arrived.

## 1. What managed policy is for

The extension resolves three sources, in this order: **administrator policy,
user settings, code defaults**. The administrator policy is published
through `chrome.storage.managed`, which is read-only from the extension's
point of view and is set by the operating system, not by the user.

Use it to:

- restrict the sites the extension may run on (`AllowedHosts`,
  `BlockedHosts`);
- turn features on or off (`FeatureFlags`);
- switch the extension off without uninstalling it (`KillSwitch`);
- freeze the user settings (`AllowUserSettings: false`).

## 2. Step 0 — the build must be wired (a known gap)

The template ships `managed-schema.json` but the manifest does **not**
reference it, so `chrome.storage.managed` is empty in a deployed build. Two
changes are needed, in the project that consumes this template:

1. **Ship the schema with the extension.** The file must be in the root of
   the packaged extension. Copy `managed-schema.json` into `public/` (the
   Vite `publicDir` is copied verbatim into `dist/`), or add a build step
   that does it.
2. **Declare the schema in the manifest.** In `src/manifest.ts`:

   ```ts
   const manifest: chrome.runtime.ManifestV3 = {
     manifest_version: 3,
     // ... the rest of the manifest
     storage: {
       managed_schema: "managed-schema.json",
     },
   };
   ```

   Without this key, Chrome never reads the schema and the managed area
   stays empty. The key `storage` also grants the `storage` permission; the
   permission itself is already in the register.

3. **Give the extension a stable id.** The template declares no `key`, so
   the extension id is derived from the install path and differs per
   machine — an administrator policy is addressed by extension id and would
   apply nowhere. For a distributed build, generate a key pair and put the
   public key in the manifest:

   ```sh
   openssl genrsa -out extension.pem 2048
   openssl rsa -pubout -outform DER -in extension.pem \
     | openssl dgst -sha256 -binary | xxd -p -c 64 | head -c 32 \
     | tr '0-9a-f' 'abcdefghijklmnop'
   ```

   The printed value is the extension id. Put the base64 of the DER public
   key in `manifest.key`. Keep `extension.pem` out of the repository.

## 3. The policy keys

| Key                 | Type                    | Meaning                                            | Effect in the extension                                       |
| ------------------- | ----------------------- | -------------------------------------------------- | ------------------------------------------------------------- |
| `AllowedHosts`      | array of match patterns | The sites the extension may run on                 | The allowlist of `isUrlAllowed`; an empty list allows nothing |
| `BlockedHosts`      | array of match patterns | The sites the extension must never run on          | Checked before the allowlist                                  |
| `FeatureFlags`      | object of booleans      | The features the administrator turns on or off     | Merged over the user flags; the administrator wins            |
| `KillSwitch`        | boolean                 | Switches the extension off without uninstalling it | `isUrlAllowed` returns false for every URL                    |
| `AllowUserSettings` | boolean                 | Whether the user may change the settings           | When false, the user settings are ignored                     |

The property names must stay in sync with `managed-schema.json` and with
`src/contracts/policy.ts`; the Chrome policy key names and the resolved
(camelCase) shape are different things on purpose.

## 4. Publishing the policy

Chrome reads enterprise policies from the OS. The extension's policy is
addressed by its **extension id**.

**Linux** — a JSON file in the managed policy directory:

```json
{
  "abcdefghijklmnopabcdefghijklmnop": {
    "AllowedHosts": ["https://intranet.example.com/*"],
    "BlockedHosts": ["https://blocked.example.com/*"],
    "FeatureFlags": { "betaPanel": false },
    "KillSwitch": false,
    "AllowUserSettings": false
  }
}
```

```sh
sudo install -m 0644 -o root -g root policy.json \
  /etc/opt/chrome/policies/managed/extension-policy.json
```

**macOS** — a managed preferences payload whose key is the extension id and
whose value is a dictionary with the same keys:

```xml
<key>abcdefghijklmnopabcdefghijklmnop</key>
<dict>
  <key>AllowedHosts</key>
  <array><string>https://intranet.example.com/*</string></array>
  <key>AllowUserSettings</key>
  <false/>
</dict>
```

Install it as a configuration profile, or into
`/Library/Managed Preferences/com.google.Chrome.plist`.

**Windows** — the registry, under the Chrome policy key. The extension id is
used as the policy name and the settings are published as a dictionary under
it; the shape below is the one Chrome documents for extension
`storage.managed` schemas:

```text
HKEY_LOCAL_MACHINE\SOFTWARE\Policies\Google\Chrome\abcdefghijklmnopabcdefghijklmnop
    AllowedHosts (REG_SZ)    = ["https://intranet.example.com/*"]
    KillSwitch   (REG_DWORD) = 0
```

Chrome reads the policy at start-up; a policy change is applied on the next
browser restart. The exact registry location and the value encoding
(a JSON string, a sub-key with one value per setting) must be checked
against the current Chrome Enterprise policy documentation for the Chrome
version the fleet runs, and confirmed in `chrome://policy`.

## 5. Verifying

| Where                        | What to check                                                                                                                                  |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `chrome://policy`            | The policy is listed, its source is shown, and its value is the one you published. An empty "No policies set" means the OS did not deliver it. |
| `chrome://extensions`        | The extension is installed and enabled.                                                                                                        |
| The extension's options page | The effective policy is listed, and the administrator's settings are marked "(set by the administrator)".                                      |
| The extension's logs         | A warning `the administrator policy is invalid; ignored` means the value did not match the schema.                                             |

The options page is the end-to-end check: it resolves the managed policy, the
user settings and the defaults, and renders the result.

## 6. Failure modes

| Symptom                                           | Cause                                                                | Fix                                                             |
| ------------------------------------------------- | -------------------------------------------------------------------- | --------------------------------------------------------------- |
| `chrome://policy` is empty                        | The file is in the wrong directory, or the browser has not restarted | Restart Chrome; check the platform path                         |
| The policy is listed but the extension ignores it | The manifest does not declare `storage.managed_schema`               | Do step 0                                                       |
| The policy is listed under another extension id   | The id changed (no `key` in the manifest)                            | Do step 0, publish again with the stable id                     |
| The options page shows the defaults               | The managed values failed the schema                                 | Read the warning in the log; compare with `managed-schema.json` |
| The user can still change a setting               | `AllowUserSettings` is not published, or the value is `true`         | Publish `AllowUserSettings: false`                              |

## 7. Removing the policy

1. Delete the file (or the registry value, or the profile payload).
2. Restart Chrome.
3. Confirm in `chrome://policy` that the policy is gone.
4. The extension falls back to the user settings and then to the defaults —
   with the default allowlist empty, `isUrlAllowed` refuses every URL.

## 8. See also

- [Policy](09-policy-admin-over-user.md)
- [Permissions Register](permissions-register.md)
- [Manifest & Permission Engineering](06-manifest-and-permission-engineering.md)
