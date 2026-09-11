// Native Accessibility fixture for `smoke-elements`. It does not inject input
// or inspect other applications.
import AppKit

func emit(_ value: [String: Any]) {
    let data = try! JSONSerialization.data(withJSONObject: value)
    FileHandle.standardOutput.write(data + Data([10]))
}

final class Delegate: NSObject, NSApplicationDelegate {
    private var primaryAction: NSButton!

    func applicationDidFinishLaunching(_ notification: Notification) {
        let screen = NSScreen.screens[0]
        let frame = NSRect(x: screen.frame.midX - 260, y: screen.frame.midY - 180, width: 520, height: 360)
        let window = NSWindow(contentRect: frame, styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "Kact accessibility fixture"

        let content = NSView(frame: NSRect(origin: .zero, size: frame.size))
        primaryAction = NSButton(title: "Primary action", target: self, action: #selector(pressed(_:)))
        primaryAction.frame = NSRect(x: 30, y: 260, width: 140, height: 32)
        primaryAction.setAccessibilityIdentifier("primary-action")
        content.addSubview(primaryAction)

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

        // Long document checks viewport clipping. Most controls are deliberately
        // outside the viewport; a scan must not return them as selectable.
        let scroll = NSScrollView(frame: NSRect(x: 290, y: 105, width: 190, height: 180))
        scroll.hasVerticalScroller = true
        let document = NSView(frame: NSRect(x: 0, y: 0, width: 170, height: 4_800))
        for index in 0 ..< 120 {
            let item = NSButton(title: "Document item \(index)", target: self, action: #selector(pressed(_:)))
            item.frame = NSRect(x: 8, y: 4_760 - index * 40, width: 145, height: 28)
            item.setAccessibilityIdentifier("document-item-\(index)")
            document.addSubview(item)
        }
        scroll.documentView = document
        scroll.setAccessibilityIdentifier("long-document")
        content.addSubview(scroll)

        // Nested viewport covers the common Electron/native pattern where a
        // scroll region exists inside another independently clipped container.
        let outer = NSScrollView(frame: NSRect(x: 290, y: 35, width: 190, height: 58))
        outer.hasVerticalScroller = true
        let innerDocument = NSView(frame: NSRect(x: 0, y: 0, width: 170, height: 240))
        let nested = NSButton(title: "Nested offscreen action", target: self, action: #selector(pressed(_:)))
        nested.frame = NSRect(x: 8, y: 202, width: 145, height: 28)
        nested.setAccessibilityIdentifier("nested-offscreen")
        innerDocument.addSubview(nested)
        let hiddenNested = NSButton(title: "Nested visible action", target: self, action: #selector(pressed(_:)))
        hiddenNested.frame = NSRect(x: 8, y: 8, width: 145, height: 28)
        hiddenNested.setAccessibilityIdentifier("nested-visible")
        innerDocument.addSubview(hiddenNested)
        outer.documentView = innerDocument
        outer.setAccessibilityIdentifier("nested-viewport")
        content.addSubview(outer)

        window.contentView = content
        window.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            emit(["ready": true, "pid": ProcessInfo.processInfo.processIdentifier])
        }
        // A real layout change after initial discovery catches cached geometry
        // bugs without requiring input injection from the smoke test.
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak self] in
            guard let self else { return }
            self.primaryAction.frame.origin.x = 190
            self.primaryAction.title = "Primary action moved"
            self.primaryAction.setAccessibilityIdentifier("primary-action-moved")
            emit(["event": "layout", "id": "primary-action-moved"])
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
