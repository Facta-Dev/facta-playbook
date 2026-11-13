package com.example.app.widgets.presentation;

import com.example.app.widgets.domain.Widget;

public record WidgetDto(String id, String name, String status, long version) {
    public static WidgetDto from(Widget widget) {
        return new WidgetDto(widget.id().value(), widget.name(), widget.status().name(), widget.version());
    }
}
