import AppKit
import SwiftUI

@main
struct UrdrApp: App {
    init() {
        // Started from `swift run` there is no app bundle, so the window needs to be activated by hand.
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate(ignoringOtherApps: true)
    }

    var body: some Scene {
        WindowGroup { ContentView() }
            .defaultSize(width: 720, height: 460)
    }
}
