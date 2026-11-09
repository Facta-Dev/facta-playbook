package acceptance

import (
	"context"
	"fmt"
	"strings"

	"github.com/cucumber/godog"

	"github.com/<scope>/<app>/internal/shared/rolex"
)

func registerSignupSteps(sc *godog.ScenarioContext) {
	sc.Step(`^I sign up "([^"]*)" with password "([^"]*)"$`, func(ctx context.Context, emailAddr, pw string) error {
		return fromCtx(ctx).doSend("POST", "/api/v1/auth/signup", signupBody(emailAddr, pw), "")
	})

	sc.Step(`^"([^"]*)" has signed up but not verified$`, func(ctx context.Context, emailAddr string) error {
		// Seeding via the API keeps this step on the real path; the status is
		// not asserted (the feature asserts what follows).
		return fromCtx(ctx).doSend("POST", "/api/v1/auth/signup", signupBody(emailAddr, "s3cure-pass-123"), "")
	})

	// The unified seed step: "a verified user", "an unverified user", "an
	// inactive user", "a blocked user", "an ADMIN user" — optional modifier,
	// article stays grammatical in the feature text.
	sc.Step(`^an? (ADMIN |verified |unverified |inactive |blocked )?user "([^"]*)" with password "([^"]*)"$`, func(ctx context.Context, mod, emailAddr, pw string) error {
		return fromCtx(ctx).app.table.seedUser(emailAddr, pw, func(r *userRecord) {
			switch strings.TrimSpace(mod) {
			case "", "verified":
				r.isVerified = true
			case "unverified":
				// stays unverified (the default)
			case "inactive":
				r.isActive = false
				r.isVerified = true
			case "blocked":
				r.isBlocked = true
				r.isVerified = true
			case "ADMIN":
				r.isVerified = true
				r.role = string(rolex.Admin)
			}
		})
	})

	sc.Step(`^I verify "([^"]*)" with the latest emailed code$`, func(ctx context.Context, emailAddr string) error {
		st := fromCtx(ctx)
		code, ok := st.app.emails.LatestCode(emailAddr, "verify_email")
		if !ok {
			return fmt.Errorf("no verify_email code was enqueued for %s", emailAddr)
		}
		return st.doSend("POST", "/api/v1/auth/verify", `{"email": "`+emailAddr+`", "code": "`+code+`"}`, "")
	})

	sc.Step(`^I verify "([^"]*)" with a wrong code$`, func(ctx context.Context, emailAddr string) error {
		st := fromCtx(ctx)
		code, ok := st.app.emails.LatestCode(emailAddr, "verify_email")
		if !ok {
			return fmt.Errorf("no verify_email code was enqueued for %s", emailAddr)
		}
		return st.doSend("POST", "/api/v1/auth/verify", `{"email": "`+emailAddr+`", "code": "`+wrongCode(code)+`"}`, "")
	})

	sc.Step(`^I resend verification for "([^"]*)"$`, func(ctx context.Context, emailAddr string) error {
		return fromCtx(ctx).doSend("POST", "/api/v1/auth/verify/resend", `{"email": "`+emailAddr+`"}`, "")
	})
}

func signupBody(emailAddr, pw string) string {
	return fmt.Sprintf(`{"email": %q, "password": %q}`, emailAddr, pw)
}

// wrongCode changes one digit of the code: same length, definitely a
// different value.
func wrongCode(code string) string {
	if code[0] != '0' {
		return "0" + code[1:]
	}
	return "1" + code[1:]
}
