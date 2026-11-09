// Platform oauthx is the OAuth infrastructure: Google One-Tap ID-token
// verification (RS256 against Google's published JWKS), the GitHub
// authorization-code client, and the github_link state-JWT codec. Offline
// mode (<APP>_OFFLINE, INV-T7) refuses before any socket is opened with a
// config-style error. Port of the Go template's oauthx package.

import { type JWTPayload, createRemoteJWKSet, SignJWT, jwtVerify } from "jose";

// The config-style refusal whenever a socket would be opened in offline mode.
export const errOffline = new Error("oauthx: offline mode forbids network calls");

// googleJWKSURL is Google's public signing-key set.
const googleJWKSURL = "https://www.googleapis.com/oauth2/v3/certs";

// GoogleIdentity is the verified bearer of a One-Tap credential.
export interface GoogleIdentity {
  sub: string;
  email: string;
  givenName: string;
  familyName: string;
  picture: string;
}

// Errors of the google verifier (mappable slices check message prefixes).
export const errInvalidIDToken = new Error("oauthx: invalid google id token");

interface GoogleClaims {
  email?: string;
  email_verified?: boolean;
  given_name?: string;
  family_name?: string;
  picture?: string;
}

interface JWTPayloadWithClaims extends Record<string, unknown> {
  iss?: string;
  sub?: string;
  exp?: number;
}

// GoogleVerifier verifies RS256 ID tokens: kid → cached key (jose's remote
// JWKS set honors max-age and force-refreshes on unknown kids), aud == client
// ID, iss == accounts.google.com, expiry, and a verified email.
export class GoogleVerifier {
  // ponytail: jose owns the JWKS cache/cooldown state machine here; the Go
  // template hand-rolls it. Same observable contract, fewer moving parts.
  private readonly jwks: ReturnType<typeof createRemoteJWKSet>;

  constructor(
    jwksURL: string,
    private readonly offline: boolean,
  ) {
    this.jwks = createRemoteJWKSet(new URL(jwksURL === "" ? googleJWKSURL : jwksURL));
  }

  // verify checks the RS256 signature, audience, issuer (both Google
  // spellings), required expiry, and a verified email.
  async verify(credential: string, clientID: string): Promise<GoogleIdentity> {
    if (this.offline) {
      throw errOffline;
    }
    const c = await this.claimsOf(credential, clientID);
    if (c.email === undefined || c.email === "" || c.email_verified !== true) {
      invalidIDToken();
    }
    return {
      sub: c.sub ?? "",
      email: c.email ?? "",
      givenName: c.given_name ?? "",
      familyName: c.family_name ?? "",
      picture: c.picture ?? "",
    };
  }

  // claimsOf verifies the token and returns its payload; every failure mode
  // (bad signature, wrong alg/aud/iss, expired, missing exp) collapses into
  // the caller-mappable invalid-id-token error.
  private async claimsOf(
    credential: string,
    clientID: string,
  ): Promise<GoogleClaims & JWTPayloadWithClaims> {
    try {
      const { payload } = await jwtVerify(credential, this.jwks, {
        algorithms: ["RS256"],
        audience: clientID,
        issuer: ["accounts.google.com", "https://accounts.google.com"],
      });
      if (payload["exp"] === undefined) {
        throw new Error("missing exp");
      }
      return payload as GoogleClaims & JWTPayloadWithClaims;
    } catch {
      return invalidIDToken();
    }
  }
}

const invalidIDToken = (): never => {
  throw errInvalidIDToken;
};

// ---------------------------------------------------------------------------
// GitHub authorization-code client (port of github.go).
// ---------------------------------------------------------------------------

// Production GitHub endpoints (overridable per-instance in tests).
const githubAuthorizeURL = "https://github.com/login/oauth/authorize";
const githubTokenURL = "https://github.com/login/oauth/access_token";
const githubUserURL = "https://api.github.com/user";
const githubEmailsURL = "https://api.github.com/user/emails";
const githubScope = "user:email";

// noreplyHost marks GitHub's placeholder addresses: never trusted when
// something real is available.
const noreplyHost = "noreply.github.com";

// bodyCap is the 1 MiB reply cap (the LimitReader analog).
const bodyCap = 1 << 20;

// GitHubUser is /user plus the resolved email.
export interface GitHubUser {
  id: string;
  login: string;
  name: string;
  email: string;
  avatarURL: string;
}

interface GhEmail {
  email: string;
  primary: boolean;
  verified: boolean;
}

// GitHubClient is the thin GitHub-side HTTP client (global fetch only).
export class GitHubClient {
  // Overridable per-instance in tests.
  authorizeURL = githubAuthorizeURL;
  tokenURL = githubTokenURL;
  userURL = githubUserURL;
  emailsURL = githubEmailsURL;

  constructor(private readonly offline: boolean) {}

  // authorize builds the consent redirect URL: client_id, redirect_uri,
  // scope=user:email and (optional) state passthrough.
  authorize(clientID: string, redirectURI: string, state: string): string {
    const q = new URLSearchParams({
      client_id: clientID,
      redirect_uri: redirectURI,
      scope: githubScope,
    });
    if (state !== "") {
      q.set("state", state);
    }
    return `${this.authorizeURL}?${q.toString()}`;
  }

  // exchange turns an authorization code into the GitHub identity: token
  // exchange, /user, then /user/emails with the exact preference order (a
  // failed /user/emails is a warning, never a hard error).
  async exchange(
    code: string,
    clientID: string,
    clientSecret: string,
    redirectURI: string,
  ): Promise<GitHubUser> {
    if (this.offline) {
      throw errOffline;
    }
    const tok = await requestJSON<{
      access_token?: string;
      error?: string;
      error_description?: string;
    }>(this.tokenURL, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        client_id: clientID,
        client_secret: clientSecret,
        code,
        redirect_uri: redirectURI,
      }).toString(),
    });
    if (tok.access_token === undefined || tok.access_token === "") {
      if (tok.error !== undefined && tok.error !== "") {
        throw new Error(`oauthx: github ${tok.error} — ${tok.error_description ?? ""}`);
      }
      throw new Error("oauthx: github authorization failed");
    }
    const raw = await requestJSON<{
      id?: number;
      login?: string;
      name?: string;
      email?: string;
      avatar_url?: string;
    }>(this.userURL, {
      headers: { authorization: `Bearer ${tok.access_token}` },
    });
    const u: GitHubUser = {
      id: String(raw.id ?? 0),
      login: raw.login ?? "",
      name: raw.name ?? "",
      email: raw.email ?? "",
      avatarURL: raw.avatar_url ?? "",
    };
    // The profile email is often the noreply placeholder — drop it.
    if (u.email.includes(noreplyHost)) {
      u.email = "";
    }
    // Always try /user/emails for the real primary email.
    let emails: GhEmail[] = [];
    try {
      emails = await requestJSON<GhEmail[]>(this.emailsURL, {
        headers: { authorization: `Bearer ${tok.access_token}` },
      });
    } catch {
      // scope-restricted tokens may never see it; fall through
    }
    resolveEmail(u, emails);
    if (u.email === "") {
      u.email = `${u.id}@users.noreply.github.com`;
    }
    return u;
  }
}

// resolveEmail applies the preference order when the profile email was empty:
// primary+verified, then any verified, then the first real address. Noreply
// placeholders never qualify.
const resolveEmail = (u: GitHubUser, emails: GhEmail[]): void => {
  if (u.email !== "") {
    return;
  }
  const real = emails.filter((e) => !e.email.endsWith(noreplyHost));
  const pick = (f: (e: GhEmail) => boolean): void => {
    const hit = real.find(f);
    if (hit !== undefined) {
      u.email = hit.email;
    }
  };
  pick((e) => e.primary && e.verified);
  if (u.email === "") {
    pick((e) => e.verified);
  }
  if (u.email === "" && real.length > 0) {
    u.email = real[0]!.email;
  }
};

// requestJSON runs the request with a 15s timeout, caps the reply at 1 MiB and
// decodes 2xx JSON (the postForm/getJSON/decode trio of github.go, collapsed).
const requestJSON = async <T>(target: string, init: RequestInit): Promise<T> => {
  const res = await fetch(target, {
    ...init,
    headers: { accept: "application/json", ...init.headers },
    signal: AbortSignal.timeout(15_000),
  });
  const text = await res.text();
  if (Buffer.byteLength(text) > bodyCap) {
    throw new Error(`oauthx: github reply exceeds ${bodyCap} bytes`);
  }
  if (res.status < 200 || res.status > 299) {
    throw new Error(`oauthx: github http ${res.status}: ${text}`);
  }
  return JSON.parse(text) as T;
};

// ---------------------------------------------------------------------------
// Link-token codec (port of linktoken.go): the ?state= JWT of the GitHub
// account-linking flow ({sub, purpose:"github_link"}, HS256, shared API
// secret). The purpose claim keeps access tokens from double-clicking as
// link states.
// ---------------------------------------------------------------------------

// 600s, matching the reference LINK_STATE_EXP_SEC.
const linkStateTTL = 10 * 60 * 1000;
const githubLinkPurpose = "github_link";

// Marks a state JWT that failed verification (wrong signature, expired,
// missing/wrong purpose).
export const errInvalidLinkState = new Error("oauthx: invalid link state");

export class LinkToken {
  private readonly key: Uint8Array;

  constructor(secret: string) {
    this.key = new TextEncoder().encode(secret);
  }

  // mintLink signs {sub, purpose, exp}.
  async mintLink(userID: string): Promise<string> {
    return new SignJWT({ purpose: githubLinkPurpose })
      .setProtectedHeader({ alg: "HS256" })
      .setSubject(userID)
      .setExpirationTime(Math.floor((Date.now() + linkStateTTL) / 1000))
      .sign(this.key);
  }

  // parseLink returns the subject of a valid link-state token; anything else
  // (wrong signature, expired, missing exp, wrong/missing purpose, empty
  // subject) is a refusal.
  async parseLink(state: string): Promise<string> {
    const payload = await this.claims(state);
    if (
      payload["exp"] === undefined ||
      payload["purpose"] !== githubLinkPurpose ||
      payload.sub === undefined ||
      payload.sub === ""
    ) {
      throw errInvalidLinkState;
    }
    return payload.sub;
  }

  // claims verifies HS256 and returns the payload; every failure mode
  // collapses into errInvalidLinkState.
  private async claims(state: string): Promise<JWTPayload> {
    try {
      const { payload } = await jwtVerify(state, this.key, { algorithms: ["HS256"] });
      return payload;
    } catch {
      throw errInvalidLinkState;
    }
  }
}
