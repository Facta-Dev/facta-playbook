package com.example.app.widgets.domain;

import java.util.Objects;

public record WidgetId(String value) {
    public WidgetId {
        Objects.requireNonNull(value, "widget id");
        if (value.isBlank()) {
            throw new IllegalArgumentException("widget id must not be blank");
        }
    }
}
