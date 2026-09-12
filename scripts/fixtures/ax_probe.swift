import Cocoa
import ApplicationServices

// Explicit diagnostic probe: requests accessibility enablement, without clicks,
// focus changes, or application UI text output.
guard CommandLine.arguments.count == 2, let pid = Int32(CommandLine.arguments[1]) else {
    fatalError("Usage: swift ax_probe.swift PID")
}
let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 0.1)
for name in ["AXManualAccessibility", "AXEnhancedUserInterface"] {
    var value: CFTypeRef?
    let read = AXUIElementCopyAttributeValue(app, name as CFString, &value)
    let write = AXUIElementSetAttributeValue(app, name as CFString, kCFBooleanTrue)
    print("\(name): read=\(read.rawValue) write=\(write.rawValue)")
}
