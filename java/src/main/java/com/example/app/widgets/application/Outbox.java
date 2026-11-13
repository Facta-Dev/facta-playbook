package com.example.app.widgets.application;

import com.example.app.shared.TenantId;

public interface Outbox {
    void append(TenantId tenant, String type, String payload, String idempotencyKey);
}
