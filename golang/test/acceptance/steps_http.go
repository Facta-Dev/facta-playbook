package acceptance

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/cucumber/godog"
)

// httpResult is the most recent HTTP exchange.
type httpResult struct {
	status   int
	body     string
	location string // Location header of a redirect response
}

// tokenPair is one remembered login (access + refresh).
type tokenPair struct{ access, refresh string }

// sseStream is one open server-sent-event stream; the reader goroutine
// pushes complete frames ("<event: ...>\n<data: ...>") onto events.
type sseStream struct {
	cancel func()
	events chan string
}

// testState is this scenario's harness state, carried on the scenario ctx.
type testState struct {
	app    *app
	client *http.Client
	last   httpResult
	pairs  map[string]tokenPair
	vars   map[string]string
	sse    map[string]*sseStream
}

type stateKey struct{}

func fromCtx(ctx context.Context) *testState {
	return ctx.Value(stateKey{}).(*testState)
}

// pair is the remembered pair behind a step label, or a helpful error.
func (st *testState) pair(label string) (tokenPair, error) {
	p, ok := st.pairs[label]
	if !ok {
		return tokenPair{}, fmt.Errorf("no logged-in session %q: log in with 'remember the pair as %s' first", label, label)
	}
	return p, nil
}

// expand substitutes {{name}} placeholders in paths and bodies.
func (st *testState) expand(s string) string {
	for k, v := range st.vars {
		s = strings.ReplaceAll(s, "{{"+k+"}}", v)
	}
	return s
}

// doSend issues one request against the real server, records the exchange
// and returns the status (steps assert on it).
func (st *testState) doSend(method, path, body, bearer string) error {
	var rdr io.Reader
	if body != "" {
		rdr = strings.NewReader(st.expand(body))
	}
	req, err := http.NewRequest(method, st.app.server.URL+st.expand(path), rdr)
	if err != nil {
		return err
	}
	if body != "" {
		req.Header.Set("Content-Type", "application/json")
	}
	if bearer != "" {
		req.Header.Set("Authorization", "Bearer "+bearer)
	}
	resp, err := st.client.Do(req)
	if err != nil {
		return err
	}
	defer func() { _ = resp.Body.Close() }()
	raw, err := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
	if err != nil {
		return err
	}
	st.last = httpResult{status: resp.StatusCode, body: string(raw), location: resp.Header.Get("Location")}
	return nil
}

// jsonMap decodes the last response body into a JSON object.
func jsonMap(body string) (map[string]any, error) {
	m := map[string]any{}
	if body == "" {
		return m, nil
	}
	if err := json.Unmarshal([]byte(body), &m); err != nil {
		return nil, fmt.Errorf("response body is not a JSON object: %s", body)
	}
	return m, nil
}

// jsonArray decodes the last response body into a JSON array.
func jsonArray(body string) ([]any, error) {
	if body == "" {
		return nil, nil
	}
	arr := []any{}
	if err := json.Unmarshal([]byte(body), &arr); err != nil {
		return nil, fmt.Errorf("response body is not a JSON array: %s", body)
	}
	return arr, nil
}

// registerHTTPSteps mounts the generic request/assert steps shared by all
// features. Labels (as <label>) are single words so the body variants can
// never be swallowed by the no-body pattern.
func registerHTTPSteps(sc *godog.ScenarioContext) {
	sc.Step(`^I send "([A-Z]+)" "([^"]*)" as ([^:\s]+)$`, func(ctx context.Context, method, path, who string) error {
		st := fromCtx(ctx)
		p, err := st.pair(who)
		if err != nil {
			return err
		}
		return st.doSend(method, path, "", p.access)
	})
	sc.Step(`^I send "([A-Z]+)" "([^"]*)" as ([^:\s]+) with body:$`, func(ctx context.Context, method, path, who string, body *godog.DocString) error {
		st := fromCtx(ctx)
		p, err := st.pair(who)
		if err != nil {
			return err
		}
		return st.doSend(method, path, body.Content, p.access)
	})
	sc.Step(`^I send "([A-Z]+)" "([^"]*)" with body:$`, func(ctx context.Context, method, path string, body *godog.DocString) error {
		return fromCtx(ctx).doSend(method, path, body.Content, "")
	})
	sc.Step(`^I send "([A-Z]+)" "([^"]*)"$`, func(ctx context.Context, method, path string) error {
		return fromCtx(ctx).doSend(method, path, "", "")
	})

	sc.Step(`^the response status is (\d+)$`, func(ctx context.Context, want int) error {
		st := fromCtx(ctx)
		if st.last.status != want {
			return fmt.Errorf("got status %d with body %s, want %d", st.last.status, st.last.body, want)
		}
		return nil
	})

	sc.Step(`^the response code is "([^"]*)"$`, func(ctx context.Context, want string) error {
		st := fromCtx(ctx)
		m, err := jsonMap(st.last.body)
		if err != nil {
			return err
		}
		got, _ := m["code"].(string)
		if got != want {
			return fmt.Errorf("got code %q (body %s), want %q", got, st.last.body, want)
		}
		return nil
	})

	sc.Step(`^the response field "([^"]*)" is "([^"]*)"$`, func(ctx context.Context, field, want string) error {
		st := fromCtx(ctx)
		m, err := jsonMap(st.last.body)
		if err != nil {
			return err
		}
		v, ok := m[field]
		if !ok {
			return fmt.Errorf("response has no field %q (body %s)", field, st.last.body)
		}
		// fmt.Sprint prints JSON numbers/booleans as bare values ("900", "true").
		if got := fmt.Sprint(v); got != want {
			return fmt.Errorf("field %q is %q (body %s), want %q", field, got, st.last.body, want)
		}
		return nil
	})

	sc.Step(`^the response has token_type "([^"]*)" and expires_in (\d+)$`, func(ctx context.Context, wantType string, wantExp int) error {
		st := fromCtx(ctx)
		m, err := jsonMap(st.last.body)
		if err != nil {
			return err
		}
		gotType, _ := m["token_type"].(string)
		exp, _ := m["expires_in"].(float64)
		if gotType != wantType || int(exp) != wantExp {
			return fmt.Errorf("want token_type %q expires_in %d, got %s", wantType, wantExp, st.last.body)
		}
		return nil
	})

	sc.Step(`^the response lists (\d+) items?$`, func(ctx context.Context, want int) error {
		st := fromCtx(ctx)
		m, err := jsonMap(st.last.body)
		if err != nil {
			return err
		}
		items, _ := m["items"].([]any)
		if len(items) != want {
			return fmt.Errorf("want %d item(s), got %d (body %s)", want, len(items), st.last.body)
		}
		return nil
	})

	// Bare-array responses (session lists, anomaly reports).
	sc.Step(`^the list has (\d+) entr(?:y|ies)$`, func(ctx context.Context, want int) error {
		st := fromCtx(ctx)
		arr, err := jsonArray(st.last.body)
		if err != nil {
			return err
		}
		if len(arr) != want {
			return fmt.Errorf("want %d entries, got %d (body %s)", want, len(arr), st.last.body)
		}
		return nil
	})

	sc.Step(`^the response field "([^"]*)" is "([^"]*)" on entry (\d+)$`, func(ctx context.Context, field, want string, idx int) error {
		st := fromCtx(ctx)
		arr, err := jsonArray(st.last.body)
		if err != nil {
			return err
		}
		if idx >= len(arr) {
			return fmt.Errorf("want entry %d, list has %d (body %s)", idx, len(arr), st.last.body)
		}
		m, ok := arr[idx].(map[string]any)
		if !ok {
			return fmt.Errorf("entry %d is not an object (body %s)", idx, st.last.body)
		}
		v, ok := m[field]
		if !ok {
			return fmt.Errorf("entry %d has no field %q (body %s)", idx, field, st.last.body)
		}
		if got := fmt.Sprint(v); got != want {
			return fmt.Errorf("entry %d field %q is %q (body %s), want %q", idx, field, got, st.last.body, want)
		}
		return nil
	})

	// Redirect steps (the github oauth routes always answer 302).
	sc.Step(`^the response redirects to a URL that starts with "([^"]*)"$`, func(ctx context.Context, prefix string) error {
		loc := fromCtx(ctx).last.location
		if !strings.HasPrefix(loc, prefix) {
			return fmt.Errorf("redirect %q does not start with %q", loc, prefix)
		}
		return nil
	})

	sc.Step(`^the response redirects carrying an access token$`, func(ctx context.Context) error {
		loc := fromCtx(ctx).last.location
		if !strings.Contains(loc, "access_token=") || !strings.Contains(loc, "refresh_token=") {
			return fmt.Errorf("redirect %q carries no token fragment", loc)
		}
		return nil
	})

	sc.Step(`^the response redirects carrying error "([^"]*)"$`, func(ctx context.Context, code string) error {
		loc := fromCtx(ctx).last.location
		if !strings.Contains(loc, "error="+code) {
			return fmt.Errorf("redirect %q carries error other than %q", loc, code)
		}
		return nil
	})

	sc.Step(`^(\d+) "(verify_email|reset_password|password_changed)" emails? (?:is|are|was|were) enqueued for "([^"]*)"$`, func(ctx context.Context, want int, template, to string) error {
		st := fromCtx(ctx)
		if got := st.app.emails.Count(to, template); got != want {
			return fmt.Errorf("want %d %q email(s) for %s, got %d", want, template, to, got)
		}
		return nil
	})

	sc.Step(`^I store the response field "([^"]*)" as ([^:\s]+)$`, func(ctx context.Context, field, name string) error {
		st := fromCtx(ctx)
		m, err := jsonMap(st.last.body)
		if err != nil {
			return err
		}
		v, ok := m[field]
		if !ok {
			return fmt.Errorf("response has no field %q (body %s)", field, st.last.body)
		}
		st.vars[name] = fmt.Sprint(v)
		return nil
	})

	sc.Step(`^(\d+) hours pass$`, func(ctx context.Context, n int) error {
		fromCtx(ctx).app.clock.Advance(time.Duration(n) * time.Hour)
		return nil
	})

	sc.Step(`^(\d+) minutes pass$`, func(ctx context.Context, n int) error {
		fromCtx(ctx).app.clock.Advance(time.Duration(n) * time.Minute)
		return nil
	})
}
