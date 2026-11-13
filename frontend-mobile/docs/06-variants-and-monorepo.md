# Variants & Monorepo

The product family is three apps from one workspace: **game**, **market** and
**avatar**. Each ships its own binary, its own bundle id, its own store
listing and only the native code it needs. The template ships **one** app
with one bundle id; the variant seam (`app/app.config.ts` and
`APP_VARIANT`) is an adoption step.

## 1. The variant matrix

| Capability                                          | Game                             | Market                     | Avatar                       |
| --------------------------------------------------- | -------------------------------- | -------------------------- | ---------------------------- |
| Shared core (auth, onboarding, push, settings, SDK) | ✔                                | ✔                          | ✔                            |
| Location                                            | Foreground + optional background | Optional, approximate      | No                           |
| Camera                                              | Optional (AR)                    | Optional (barcode, photos) | Optional (face/pose)         |
| Microphone                                          | Optional                         | Optional                   | **Core**                     |
| 3D rendering                                        | **Core**                         | Optional, off by default   | **Core**                     |
| Realtime                                            | **Core**                         | Order status, price alerts | **Core**                     |
| Payments                                            | In-app purchases                 | **Core**                   | Subscriptions, digital       |
| Offline                                             | Cached tiles, collection         | Cart, wishlist, recent     | Cached avatar, last sessions |

The rule that keeps this cheap: **one variant = one binary.** A market app
that carries a game engine pays for it in download size, memory and store
review — on the low tier, that is the difference between installing and not.

## 2. The Expo config

The Expo config of the template is `app/app.json`, and it is static:

```json
{
  "expo": {
    "name": "<APP>",
    "slug": "<app>",
    "scheme": "<app>",
    "version": "1.0.0",
    "orientation": "portrait",
    "userInterfaceStyle": "light",
    "android": { "package": "com.<scope>.<app>" },
    "ios": { "bundleIdentifier": "com.<scope>.<app>" }
  }
}
```

After `scripts/rename.sh` runs, the three tokens are the app's identity: the
display name, the slug, the URL scheme and both bundle ids.

**There is no variant matrix in the template.** The seam a product adds is
`app/app.config.ts`, a dynamic config that reads one environment variable
and produces the same shape:

```ts
const variant = process.env.APP_VARIANT ?? "market";
```

The variant then selects:

| What                   | Where it is decided                                 |
| ---------------------- | --------------------------------------------------- |
| The display name       | `expo.name`                                         |
| The slug               | `expo.slug`                                         |
| The URL scheme         | `expo.scheme`                                       |
| The bundle id          | `expo.ios.bundleIdentifier`, `expo.android.package` |
| The icon and splash    | The asset folders                                   |
| The plugins            | `expo.plugins`                                      |
| The permissions        | `expo.android.permissions`                          |
| The associated domains | `expo.ios.associatedDomains`                        |

The app code reads the variant from the config's `extra` block, not from the
environment directly, so the bundle and the runtime agree:

```ts
extra: { variant, apiBaseUrl: process.env.API_BASE_URL }
```

> **Known gap.** The template ships `app/app.json`, not `app/app.config.ts`.
> One variant exists, and the dynamic config is an adoption step.

## 3. Per-variant native modules

A native module belongs to the variant that needs it. The plugin list is the
switch:

| Variant | Plugins (the template ships none of them)  |
| ------- | ------------------------------------------ |
| game    | a map SDK, a 3D engine, `expo-location`    |
| market  | a payments SDK, a scanner                  |
| avatar  | a 3D engine, `expo-audio`, a WebRTC module |

Two consequences, both enforced by review rather than by the compiler:

1. **The Metro graph must not pull a foreign engine.** The market build must
   not contain the game engine's native library, even if a module is
   imported somewhere.
2. **The permission list is per variant.** A market build that declares
   `ACCESS_FINE_LOCATION` fails the store review.

## 4. The EAS profiles

The build profiles are per variant × per environment. The template ships
**no `eas.json`**: a build profile is a product decision about signing,
distribution and credentials, not a template decision.

The shape a project adopts:

```json
{
  "build": {
    "development": { "developmentClient": true, "distribution": "internal" },
    "production": { "channel": "production" }
  }
}
```

One profile per variant per environment, with the variant's own credentials
(see [Admin Guide](admin-guide.md)).

> **Known gap.** The template ships no `eas.json`, no credentials and no
> build pipeline. `pnpm run ci` proves the source; it does not build a
> binary.

## 5. What is shared, what is not

| Shared across variants                            | Never shared                    |
| ------------------------------------------------- | ------------------------------- |
| `packages/core` — `Result`, tiers, the four ports | Native modules                  |
| `packages/contracts` — the DTOs, the route table  | The engine adapters             |
| The auth, onboarding, push and settings flows     | The icons, the splash, the name |
| The design tokens and the base component kit      | The permission list             |
| The SDK                                           | The bundle id                   |

The route grammar is shared too: every variant answers to the same path
shape, so a deep link sent to a market app and to a game app has the same
form ([07](07-navigation-routes-and-deep-links.md)).

## 6. The adoption steps

What a product must add to go from this template to three shipping apps:

1. A variant directory per product, or a variant matrix in `app/app.config.ts`.
2. The per-variant plugin and permission lists.
3. The feature packages per variant, each a workspace package like
   `packages/feature-notes`.
4. The EAS profiles and the store credentials.
5. A CI job per variant, so a build that breaks one variant is red.
6. The per-variant asset budget check
   ([Device Support Matrix](device-support-matrix.md)).

## Do / Don't

| Do                                                  | Don't                                |
| --------------------------------------------------- | ------------------------------------ |
| Select the variant from one environment variable    | Branch the source per variant        |
| Give each variant its own bundle id and permissions | Ship one binary with every feature   |
| Keep `packages/core` variant-agnostic               | Put a game engine in `packages/core` |
| Build each variant in CI                            | Build one variant by hand            |

## See also

- [Directory Structure](05-directory-structure.md)
- [Navigation, Routes & Deep Links](07-navigation-routes-and-deep-links.md)
- [Admin Guide](admin-guide.md)
