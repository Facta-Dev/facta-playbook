package acceptance

import (
	"context"
	"net/http"
	"testing"

	"github.com/cucumber/godog"
	"github.com/stretchr/testify/require"

	"github.com/<scope>/<app>/test/testutil"
)

// TestAcceptance runs every feature under specs/features against the real
// HTTP mux (in-memory fakes, recorded emails, in-process hub — no database).
func TestAcceptance(t *testing.T) {
	suite := godog.TestSuite{
		ScenarioInitializer: InitializeScenario,
		Options: &godog.Options{
			Paths:    []string{"../../specs/features"}, // relative to test/acceptance
			Format:   "pretty",
			TestingT: t,
			Strict:   true, // an undefined step must fail the run
		},
	}
	// require keeps testify load-bearing in the untagged suite.
	require.Zero(t, suite.Run())
}

// InitializeScenario wires fresh state per scenario and registers every
// step family; each feature file contributes its own steps_*.go.
func InitializeScenario(sc *godog.ScenarioContext) {
	sc.Before(func(ctx context.Context, _ *godog.Scenario) (context.Context, error) {
		testutil.ResetIDs()
		a := newApp()
		st := &testState{
			app: a,
			// Redirects are responses, not navigations: the github oauth
			// routes 302 and their Location stays under assertion.
			client: &http.Client{CheckRedirect: func(*http.Request, []*http.Request) error {
				return http.ErrUseLastResponse
			}},
			pairs: map[string]tokenPair{},
			vars:  map[string]string{},
			sse:   map[string]*sseStream{},
		}
		return context.WithValue(ctx, stateKey{}, st), nil
	})
	sc.After(func(ctx context.Context, _ *godog.Scenario, _ error) (context.Context, error) {
		st := fromCtx(ctx)
		// Cancel the SSE readers first: server.Close blocks until handlers
		// exit, and the readers hold the connections open.
		for _, stream := range st.sse {
			stream.cancel()
		}
		st.app.server.Close()
		return ctx, nil
	})

	registerHTTPSteps(sc)
	registerSignupSteps(sc)
	registerSessionSteps(sc)
	registerPasswordSteps(sc)
	registerAdminSteps(sc)
	registerWidgetSteps(sc)
	registerOAuthSteps(sc)
}
