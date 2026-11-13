package com.example.app.shared;

public record AppError(String code, String message, String requiredPermission, String correlationId) {
    public static AppError notFound() {
        return new AppError("NOT_FOUND", "Not found", null, null);
    }

    public static AppError conflict() {
        return new AppError("CONFLICT", "The resource changed", null, null);
    }

    public static AppError forbidden(String permission) {
        return new AppError("FORBIDDEN", "Forbidden", permission, null);
    }
}
