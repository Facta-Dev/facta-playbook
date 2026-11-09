// Shared authz answers the one authorization question the auth slices ask:
// may this actor manage that account — itself, or a subordinate in the rolex
// hierarchy? CanManageSubordinate = Rank(subject) > Rank(target);
// CanManageSelf = same id AND Rank(target) <= Rank(subject) — equals never
// manage equals, and a stale token cannot outrank the row it acts on.

import { E, CodeForbidden, AuthForbidden, type AppError } from "#shared/apperr.js";
import { rank, type Role } from "#shared/rolex.js";

// ErrForbidden is the stable 403 guards raise when the answer is no.
export const ErrForbidden: AppError = E(CodeForbidden, AuthForbidden, "insufficient role");

// AuthzContext is one authorization question: Subject (the actor, id + role
// from the verified token) acts on Target (id + role from the store).
export interface AuthzContext {
  subjectId: string;
  subjectRole: Role;
  targetId: string;
  targetRole: Role;
}

// canManageSelf reports whether the actor IS the target and may keep managing
// it: the acting role must still rank at-or-above the target's DB role — a
// token minted before a demotion answers false.
export const canManageSelf = (c: AuthzContext): boolean =>
  c.subjectId !== "" && c.subjectId === c.targetId && rank(c.targetRole) <= rank(c.subjectRole);

// canManageSubordinate reports whether the actor outranks the target. The
// hierarchy is strict: an ADMIN cannot reach another ADMIN's row.
export const canManageSubordinate = (c: AuthzContext): boolean =>
  rank(c.subjectRole) > rank(c.targetRole);

// canManage is the composite use-case rule: self or subordinate.
export const canManage = (c: AuthzContext): boolean => canManageSelf(c) || canManageSubordinate(c);
