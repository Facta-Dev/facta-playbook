// Package rolex holds the single role vocabulary of the system and the
// hierarchy used to authorize. ctx0 parity: one role column, ranking
// ADMIN > MODERATOR > USER > GUEST.
package rolex

type Role string

const (
	Admin     Role = "ADMIN"
	Moderator Role = "MODERATOR"
	User      Role = "USER"
	Guest     Role = "GUEST"
)

// All enumerates valid role values (DB CHECK constraint mirrors this).
var All = []Role{Admin, Moderator, User, Guest}

// Valid reports whether r is a known role.
func Valid(r Role) bool {
	for _, k := range All {
		if k == r {
			return true
		}
	}
	return false
}

// Rank orders roles: higher wins. Unknown roles rank below Guest.
func Rank(r Role) int {
	switch r {
	case Admin:
		return 4
	case Moderator:
		return 3
	case User:
		return 2
	case Guest:
		return 1
	default:
		return 0
	}
}

// AtLeast reports whether have meets the required minimum.
func AtLeast(have, min Role) bool { return Rank(have) >= Rank(min) }
