// Native Accessibility fixture for `smoke-elements`. It does not inject input
// or inspect other applications.
import AppKit

func emit(_ value: [String: Any]) {
    let data = try! JSONSerialization.data(withJSONObject: value)
    FileHandle.standardOutput.write(data + Data([10]))
}

final class Delegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        let screen = NSScreen.screens[0]
        let frame = NSRect(x: screen.frame.midX - 260, y: screen.frame.midY - 180, width: 520, height: 360)
        let window = NSWindow(contentRect: frame, styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "Kact accessibility fixture"

        let content = NSView(frame: NSRect(origin: .zero, size: frame.size))
        let button = NSButton(title: "Primary action", target: self, action: #selector(pressed(_:)))
        button.frame = NSRect(x: 30, y: 260, width: 140, height: 32)
        button.setAccessibilityIdentifier("primary-action")
        content.addSubview(button)

        let checkbox = NSButton(checkboxWithTitle: "Remember choice", target: self, action: #selector(pressed(_:)))
        checkbox.frame = NSRect(x: 30, y: 215, width: 180, height: 24)
        checkbox.setAccessibilityIdentifier("remember-choice")
        content.addSubview(checkbox)

        let field = NSTextField(frame: NSRect(x: 30, y: 165, width: 240, height: 24))
        field.placeholderString = "Fixture text field"
        field.setAccessibilityIdentifier("fixture-text")
        content.addSubview(field)

        let slider = NSSlider(value: 50, minValue: 0, maxValue: 100, target: self, action: #selector(pressed(_:)))
        slider.frame = NSRect(x: 30, y: 110, width: 240, height: 24)
        slider.setAccessibilityIdentifier("fixture-slider")
        content.addSubview(slider)

        // Deliberately inaccessible drawing surface: confirms grid fallback
        // remains available for custom canvas-like interfaces.
        let canvas = NSView(frame: NSRect(x: 310, y: 40, width: 170, height: 250))
        canvas.wantsLayer = true
        canvas.layer?.backgroundColor = NSColor.systemGray.cgColor
        canvas.setAccessibilityElement(false)
        content.addSubview(canvas)

        window.contentView = content
        window.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            emit(["ready": true, "pid": ProcessInfo.processInfo.processIdentifier])
        }
    }

    @objc func pressed(_ sender: Any?) {
        emit(["event": "action"])
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let delegate = Delegate()
app.delegate = delegate
app.run()
