/// The result of an operation that can fail for an expected reason.
///
/// Expected failures are values, not thrown errors: a use case returns an
/// `Outcome` so the caller must handle both branches before it can read the
/// result.
public enum Outcome<Value: Sendable>: Sendable {
    case success(Value)
    case failure(AppError)

    /// Transforms the success value, preserving a failure unchanged.
    public func map<NewValue: Sendable>(
        _ transform: (Value) -> NewValue
    ) -> Outcome<NewValue> {
        switch self {
        case .success(let value): .success(transform(value))
        case .failure(let error): .failure(error)
        }
    }

    /// The success value, or `nil` when the outcome is a failure.
    public var value: Value? {
        if case .success(let value) = self { return value }
        return nil
    }

    /// The failure, or `nil` when the outcome is a success.
    public var error: AppError? {
        if case .failure(let error) = self { return error }
        return nil
    }
}
