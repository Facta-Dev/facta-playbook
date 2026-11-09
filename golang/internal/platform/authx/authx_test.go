package authx

import (
	"testing"
	"time"
)

func TestPasswordRoundTrip(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name   string
		pepper string
	}{{name: "with pepper", pepper: "a-pepper-32-bytes-or-more-x"}, {name: "no pepper", pepper: ""}}
	for _, tc := range tests {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			hash, err := HashPassword("s3cret-pw", tc.pepper)
			if err != nil {
				t.Fatalf("hash: %v", err)
			}
			if err := CheckPassword(hash, "s3cret-pw", tc.pepper); err != nil {
				t.Fatalf("CheckPassword right pw: %v", err)
			}
			if err := CheckPassword(hash, "wrong", tc.pepper); err == nil {
				t.Fatal("CheckPassword wrong pw: want error")
			}
		})
	}
}

func TestPasswordPepperMatters(t *testing.T) {
	t.Parallel()
	hA, err := HashPassword("pw", "pepper-aaaaaaaaaaaaaaaaaaaaaaaaaaaa")
	if err != nil {
		t.Fatal(err)
	}
	hB, err := HashPassword("pw", "pepper-bbbbbbbbbbbbbbbbbbbbbbbbbbbb")
	if err != nil {
		t.Fatal(err)
	}
	if hA == hB {
		t.Fatal("same password with different peppers must hash differently")
	}
	if err := CheckPassword(hA, "pw", "pepper-bbbbbbbbbbbbbbbbbbbbbbbbbbbb"); err == nil {
		t.Fatal("hash from pepper A must not verify under pepper B")
	}
	// Salt-independence contract: the HMAC pre-hash means the stored hash
	// verifies identically under re-computation regardless of the random
	// bcrypt salt baked into each new hash.
	hA2, err := HashPassword("pw", "pepper-aaaaaaaaaaaaaaaaaaaaaaaaaaaa")
	if err != nil {
		t.Fatal(err)
	}
	if err := CheckPassword(hA2, "pw", "pepper-aaaaaaaaaaaaaaaaaaaaaaaaaaaa"); err != nil {
		t.Fatal(err)
	}
}

func TestMintParseRoundTrip(t *testing.T) {
	t.Parallel()
	secret := "test-secret-32-bytes-0123456789ab"
	// Verifier and minter must share the secret and issuer.
	ver := NewHS256Verifier(secret, "test-api")
	min := NewHS256Minter(secret, "test-api")
	in := Identity{UserID: "u-1", Email: "u-1@example.com", Role: "ADMIN", JTI: "s-1"}
	raw, exp, err := min.Mint(in, 15*time.Minute)
	if err != nil {
		t.Fatalf("mint: %v", err)
	}
	if time.Until(exp) <= 0 {
		t.Fatalf("exp not in the future: %v", exp)
	}
	got, err := ver.Parse(raw)
	if err != nil {
		t.Fatalf("parse: %v", err)
	}
	if got != in {
		t.Fatalf("roundtrip mismatch: got %+v want %+v", got, in)
	}
}

func TestParseWrongSecret(t *testing.T) {
	t.Parallel()
	min := NewHS256Minter("secret-one-32-bytes-0123456789abcd", "test-api")
	ver := NewHS256Verifier("secret-two-32-bytes-0123456789abcd", "test-api")
	raw, _, err := min.Mint(Identity{UserID: "u-1", Role: "USER"}, time.Minute)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := ver.Parse(raw); err == nil {
		t.Fatal("want error for signature from a different secret")
	}
}

func TestParseExpired(t *testing.T) {
	t.Parallel()
	secret := "test-secret-32-bytes-0123456789ab"
	ver := NewHS256Verifier(secret, "test-api")
	min := NewHS256Minter(secret, "test-api")
	// Clock control via claims: an already-past exp is rejected by jwt/v5.
	raw, _, err := min.Mint(Identity{UserID: "u-1", Role: "USER"}, -time.Minute)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := ver.Parse(raw); err == nil {
		t.Fatal("want error for expired token")
	}
}
