package acceptance

import (
	"context"
	"fmt"

	"github.com/cucumber/godog"
)

func registerSessionSteps(sc *godog.ScenarioContext) {
	sc.Step(`^I log in as "([^"]*)" with password "([^"]*)"$`, loginOnce)
	sc.Step(`^I log in as "([^"]*)" with password "([^"]*)" (\d+) times$`, func(ctx context.Context, emailAddr, pw string, n int) error {
		for i := 0; i < n; i++ {
			if err := loginOnce(ctx, emailAddr, pw); err != nil {
				return err
			}
		}
		return nil
	})
	sc.Step(`^I log in as "([^"]*)" with password "([^"]*)" and remember the pair as ([^:\s]+)$`, func(ctx context.Context, emailAddr, pw, label string) error {
		st := fromCtx(ctx)
		if err := loginOnce(ctx, emailAddr, pw); err != nil {
			return err
		}
		if st.last.status != 200 {
			return fmt.Errorf("login as %s failed with %d and body %s", emailAddr, st.last.status, st.last.body)
		}
		m, err := jsonMap(st.last.body)
		if err != nil {
			return err
		}
		access, _ := m["access_token"].(string)
		refresh, _ := m["refresh_token"].(string)
		if access == "" || refresh == "" {
			return fmt.Errorf("login response has no token pair: %s", st.last.body)
		}
		st.pairs[label] = tokenPair{access: access, refresh: refresh}
		return nil
	})

	// Refresh does not overwrite the remembered pair: rotation means the old
	// access token is what the scenario keeps asserting against.
	sc.Step(`^I refresh using the token "([^"]+)"$`, func(ctx context.Context, label string) error {
		st := fromCtx(ctx)
		p, err := st.pair(label)
		if err != nil {
			return err
		}
		return st.doSend("POST", "/api/v1/auth/refresh", fmt.Sprintf(`{"refresh_token": %q}`, p.refresh), "")
	})

	sc.Step(`^I log out with the token "([^"]+)"$`, func(ctx context.Context, label string) error {
		st := fromCtx(ctx)
		p, err := st.pair(label)
		if err != nil {
			return err
		}
		return st.doSend("POST", "/api/v1/auth/logout",
			fmt.Sprintf(`{"refresh_token": %q}`, p.refresh), p.access)
	})
}

func loginOnce(ctx context.Context, emailAddr, pw string) error {
	return fromCtx(ctx).doSend("POST", "/api/v1/auth/login",
		fmt.Sprintf(`{"email": %q, "password": %q}`, emailAddr, pw), "")
}
