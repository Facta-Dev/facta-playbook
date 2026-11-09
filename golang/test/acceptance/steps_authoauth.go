package acceptance

import (
	"context"
	"fmt"

	"github.com/cucumber/godog"
)

// registerOAuthSteps mounts the social sign-in steps. Provider toggles
// rebuild the app (the use case copies its config at Register time); the
// Background's bearer pairs stay valid across a rebuild because the JWT
// secret is shared and bearer verification is stateless.
func registerOAuthSteps(sc *godog.ScenarioContext) {
	sc.Step(`^"([^"]*)" signs in with a Google credential$`, func(ctx context.Context, emailAddr string) error {
		return fromCtx(ctx).doSend("POST", "/api/v1/auth/google",
			fmt.Sprintf(`{"credential": %q}`, emailAddr), "")
	})

	sc.Step(`^"([^"]*)" signs in with an invalid Google credential$`, func(ctx context.Context, emailAddr string) error {
		return fromCtx(ctx).doSend("POST", "/api/v1/auth/google", `{"credential": "not-a-credential"}`, "")
	})

	sc.Step(`^the google provider is not configured$`, func(ctx context.Context) error {
		fromCtx(ctx).rebuildApp(func(f *oauthFlags) { f.google = false })
		return nil
	})

	sc.Step(`^the github provider is disabled$`, func(ctx context.Context) error {
		fromCtx(ctx).rebuildApp(func(f *oauthFlags) { f.github = false })
		return nil
	})
}

// rebuildApp closes the running server and starts a fresh app with the
// given flags applied on the default switches. State (table, emails) starts
// empty again, mirroring a restart with different configuration.
func (st *testState) rebuildApp(apply func(*oauthFlags)) {
	flags := oauthFlags{google: true, github: true}
	apply(&flags)
	st.app.server.Close()
	st.app = newAppWith(flags)
}
