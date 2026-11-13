package com.example.app.shared;

import java.util.Objects;

public record TenantId(String value) {
    public TenantId {
        Objects.requireNonNull(value, "tenant id");
        if (value.isBlank()) {
            throw new IllegalArgumentException("tenant id must not be blank");
        }
    }
}
