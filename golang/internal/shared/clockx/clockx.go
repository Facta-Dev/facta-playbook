// Package clockx makes time injectable so tests are deterministic.
package clockx

import "time"

type Clock interface {
	Now() time.Time
}

type System struct{}

func (System) Now() time.Time { return time.Now() }

// Fixed is the test double.
type Fixed time.Time

func (f Fixed) Now() time.Time { return time.Time(f) }
