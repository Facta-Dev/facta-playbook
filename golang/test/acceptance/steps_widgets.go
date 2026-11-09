package acceptance

import (
	"bufio"
	"bytes"
	"context"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/cucumber/godog"
)

func registerWidgetSteps(sc *godog.ScenarioContext) {
	sc.Step(`^I open the event stream for "([^"]*)"$`, func(ctx context.Context, who string) error {
		st := fromCtx(ctx)
		p, err := st.pair(who)
		if err != nil {
			return err
		}
		cctx, cancel := context.WithCancel(context.Background())
		req, err := http.NewRequestWithContext(cctx, "GET", st.app.server.URL+"/api/v1/widgets/events", nil)
		if err != nil {
			cancel()
			return err
		}
		req.Header.Set("Authorization", "Bearer "+p.access)
		req.Header.Set("Accept", "text/event-stream")
		resp, err := st.client.Do(req)
		if err != nil {
			cancel()
			return err
		}
		// The handler subscribes to the hub before its first byte, so this
		// handshake read guarantees the subscription is live before any
		// subsequent create/update/delete can publish.
		handshake := []byte("retry: 2000\n\n")
		have := make([]byte, 0, 128)
		buf := make([]byte, 64)
		for !bytes.Contains(have, handshake) {
			n, rerr := resp.Body.Read(buf)
			if rerr != nil {
				_ = resp.Body.Close()
				cancel()
				return fmt.Errorf("waiting for the %q SSE handshake: %w", who, rerr)
			}
			have = append(have, buf[:n]...)
		}
		stream := &sseStream{
			cancel: func() {
				cancel()
				_ = resp.Body.Close()
			},
			events: make(chan string, 16),
		}
		go func() {
			defer close(stream.events)
			scanner := bufio.NewScanner(resp.Body)
			var frame []string
			for scanner.Scan() {
				line := scanner.Text()
				if line == "" {
					// blank line completes one SSE frame
					if len(frame) > 0 {
						stream.events <- strings.Join(frame, "\n")
						frame = nil
					}
					continue
				}
				frame = append(frame, line)
			}
		}()
		st.sse[who] = stream
		return nil
	})

	sc.Step(`^the event stream of "([^"]*)" shows a "([^"]*)" event$`, func(ctx context.Context, who, want string) error {
		stream, ok := fromCtx(ctx).sse[who]
		if !ok {
			return fmt.Errorf("no open event stream for %q: open it with 'I open the event stream for %q' first", who, who)
		}
		select {
		case frame := <-stream.events:
			if got := strings.SplitN(frame, "\n", 2)[0]; !strings.HasPrefix(got, "event: "+want) {
				return fmt.Errorf("want a %q event, got %q", want, got)
			}
			return nil
		case <-time.After(5 * time.Second):
			return fmt.Errorf("no %q event arrived within 5s", want)
		}
	})
}
