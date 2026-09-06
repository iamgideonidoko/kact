// Native test receiver; never injects input or interacts with other applications.
import AppKit

func emit(_ value: [String: Any]) {
    let data = try! JSONSerialization.data(withJSONObject: value)
    FileHandle.standardOutput.write(data + Data([10]))
}
final class Receiver: NSView {
    override var acceptsFirstResponder: Bool { true }
    override func draw(_ rect: NSRect) {
        NSColor.windowBackgroundColor.setFill()
        rect.fill()
        ("Kact mouse test — closes automatically" as NSString).draw(
            at: NSPoint(x: 30, y: 30), withAttributes: [.font: NSFont.systemFont(ofSize: 16)])
    }
    func record(_ kind: String, _ event: NSEvent) {
        emit(["event": kind, "count": event.clickCount,
              "command": event.modifierFlags.contains(.command)])
    }
    override func keyDown(with event: NSEvent) { emit(["event": "key", "code": event.keyCode]) }
    override func mouseDown(with event: NSEvent) { record("left-down", event) }
    override func mouseUp(with event: NSEvent) { record("left-up", event) }
    override func rightMouseDown(with event: NSEvent) { record("right-down", event) }
    override func rightMouseUp(with event: NSEvent) { record("right-up", event) }
    override func otherMouseDown(with event: NSEvent) { record("middle-down", event) }
    override func otherMouseUp(with event: NSEvent) { record("middle-up", event) }
    override func mouseDragged(with event: NSEvent) { emit(["event": "drag"]) }
    override func scrollWheel(with event: NSEvent) {
        emit(["event": "scroll", "dx": event.scrollingDeltaX, "dy": event.scrollingDeltaY])
    }
}
let application = NSApplication.shared
application.setActivationPolicy(.regular)
let original = CGEvent(source: nil)!.location
let screen = NSScreen.screens[0]
let frame = NSRect(x: screen.frame.midX - 250, y: screen.frame.midY - 120, width: 500, height: 240)
let window = NSWindow(contentRect: frame, styleMask: [.titled], backing: .buffered, defer: false)
window.title = "Kact mouse test"
window.contentView = Receiver(frame: NSRect(origin: .zero, size: frame.size))
window.level = .statusBar
window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
window.makeKeyAndOrderFront(nil)
window.makeFirstResponder(window.contentView)
application.activate(ignoringOtherApps: true)
DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
    let point = window.convertPoint(toScreen: NSPoint(x: 250, y: 120))
    emit(["ready": true, "x": point.x, "y": screen.frame.height - point.y,
          "key": window.isKeyWindow, "visible": window.occlusionState.contains(.visible),
          "frontmost": NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,
          "pid": ProcessInfo.processInfo.processIdentifier,
          "original_x": original.x, "original_y": original.y])
}
application.run()
