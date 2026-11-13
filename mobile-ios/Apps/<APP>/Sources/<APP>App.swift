import SwiftUI

@main
struct <APP>App: App {
    private let environment = AppEnvironment.live

    var body: some Scene {
        WindowGroup {
            RootView(environment: environment)
        }
    }
}
