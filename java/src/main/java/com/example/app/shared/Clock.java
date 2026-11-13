package com.example.app.shared;

import java.time.Instant;

@FunctionalInterface
public interface Clock {
    Instant now();
}
