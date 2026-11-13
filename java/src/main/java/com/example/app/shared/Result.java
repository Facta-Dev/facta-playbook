package com.example.app.shared;

import java.util.function.Function;

public sealed interface Result<T> permits Result.Success, Result.Failure {
    record Success<T>(T value) implements Result<T> {
    }

    record Failure<T>(AppError error) implements Result<T> {
    }

    static <T> Result<T> success(T value) {
        return new Success<>(value);
    }

    static <T> Result<T> failure(AppError error) {
        return new Failure<>(error);
    }

    @SuppressWarnings("unchecked")
    default <R> Result<R> map(Function<T, R> mapper) {
        if (this instanceof Success<?> success) {
            return Result.success(mapper.apply((T) success.value()));
        }
        Failure<?> failure = (Failure<?>) this;
        return Result.failure(failure.error());
    }
}
