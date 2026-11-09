package acceptance

import (
	"context"
	"fmt"

	"github.com/cucumber/godog"
)

func registerPasswordSteps(sc *godog.ScenarioContext) {
	sc.Step(`^I ask to reset the password for "([^"]*)"$`, forgotOnce)
	sc.Step(`^I asked to reset the password for "([^"]*)" (\d+) times$`, func(ctx context.Context, emailAddr string, n int) error {
		for i := 0; i < n; i++ {
			if err := forgotOnce(ctx, emailAddr); err != nil {
				return err
			}
		}
		return nil
	})
	// "requested a reset code" asserts the 200: the feature needs a live code.
	sc.Step(`^I requested a reset code for "([^"]*)"$`, func(ctx context.Context, emailAddr string) error {
		st := fromCtx(ctx)
		if err := forgotOnce(ctx, emailAddr); err != nil {
			return err
		}
		if st.last.status != 200 {
			return fmt.Errorf("reset request for %s failed with %d and body %s", emailAddr, st.last.status, st.last.body)
		}
		return nil
	})

	sc.Step(`^I validate "([^"]*)" with the latest reset code$`, validateWith(latestResetCode))
	sc.Step(`^I validate "([^"]*)" with a wrong reset code$`, validateWith(wrongResetCode))

	sc.Step(`^I reset the password for "([^"]*)" using the latest reset code and password "([^"]*)"$`, func(ctx context.Context, emailAddr, pw string) error {
		st := fromCtx(ctx)
		code, err := latestResetCode(st, emailAddr)
		if err != nil {
			return err
		}
		return st.doSend("POST", "/api/v1/auth/reset-password",
			fmt.Sprintf(`{"email": %q, "code": %q, "new_password": %q}`, emailAddr, code, pw), "")
	})

	// The current password is wrong => 401; the success case keeps this
	// caller's own session (jti) alive and revokes the others.
	sc.Step(`^I change the password of ([^:\s]+) from "([^"]*)" to "([^"]*)"$`, func(ctx context.Context, label, current, fresh string) error {
		st := fromCtx(ctx)
		p, err := st.pair(label)
		if err != nil {
			return err
		}
		return st.doSend("POST", "/api/v1/auth/change-password",
			fmt.Sprintf(`{"current_password": %q, "new_password": %q}`, current, fresh), p.access)
	})
}

func forgotOnce(ctx context.Context, emailAddr string) error {
	return fromCtx(ctx).doSend("POST", "/api/v1/auth/forgot-password",
		fmt.Sprintf(`{"email": %q}`, emailAddr), "")
}

// latestResetCode reads the newest reset_password code for the address.
func latestResetCode(st *testState, emailAddr string) (string, error) {
	code, ok := st.app.emails.LatestCode(emailAddr, "reset_password")
	if !ok {
		return "", fmt.Errorf("no reset_password code was enqueued for %s", emailAddr)
	}
	return code, nil
}

// wrongResetCode returns a same-length but invalid guess.
func wrongResetCode(st *testState, emailAddr string) (string, error) {
	code, err := latestResetCode(st, emailAddr)
	if err != nil {
		return "", err
	}
	return wrongCode(code), nil
}

// validateWith posts the validation with a resolved code.
func validateWith(resolve func(*testState, string) (string, error)) interface{} {
	return func(ctx context.Context, emailAddr string) error {
		st := fromCtx(ctx)
		code, err := resolve(st, emailAddr)
		if err != nil {
			return err
		}
		return st.doSend("POST", "/api/v1/auth/validate-code",
			fmt.Sprintf(`{"email": %q, "code": %q}`, emailAddr, code), "")
	}
}
