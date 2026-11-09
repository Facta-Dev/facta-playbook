// Shared rolex holds the single role vocabulary of the system and the
// hierarchy used to authorize. One role column, ranking ADMIN > MODERATOR >
// USER > GUEST (the DB CHECK constraint mirrors All).

export type Role = "ADMIN" | "MODERATOR" | "USER" | "GUEST";

export const Admin: Role = "ADMIN";
export const Moderator: Role = "MODERATOR";
export const User: Role = "USER";
export const Guest: Role = "GUEST";

// All enumerates valid role values (DB CHECK constraint mirrors this).
export const All: readonly Role[] = [Admin, Moderator, User, Guest];

// valid reports whether r is a known role.
export const valid = (r: Role): boolean => All.includes(r);

// rank orders roles: higher wins. Unknown roles rank below Guest.
export const rank = (r: Role): number =>
  r === Admin ? 4 : r === Moderator ? 3 : r === User ? 2 : r === Guest ? 1 : 0;

// atLeast reports whether have meets the required minimum.
export const atLeast = (have: Role, min: Role): boolean => rank(have) >= rank(min);
