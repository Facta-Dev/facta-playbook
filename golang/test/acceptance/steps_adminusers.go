package acceptance

import (
	"context"
	"fmt"

	"github.com/cucumber/godog"
)

func registerAdminSteps(sc *godog.ScenarioContext) {
	// The audit trail is in-memory shared state rather than an HTTP
	// response, so it gets its own step.
	sc.Step(`^(\d+) audit entr(?:y|ies) (?:is|are|was|were) recorded$`, func(ctx context.Context, want int) error {
		if got := len(fromCtx(ctx).app.table.audit); got != want {
			return fmt.Errorf("want %d audit entries, got %d", want, got)
		}
		return nil
	})
}
