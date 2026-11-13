package com.example.app.shared;

import jakarta.enterprise.context.ApplicationScoped;
import java.time.Instant;

@ApplicationScoped
public final class SystemClock implements Clock {
    @Override
    public Instant now() {
        return Instant.now();
    }
}
