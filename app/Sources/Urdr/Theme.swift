import SwiftUI

// Moises-inspired palette: near-black ground, white primary controls, one hue per stem and nowhere else.
enum Theme {
    static let ground = Color(red: 0.055, green: 0.059, blue: 0.067)
    static let panel = Color(red: 0.09, green: 0.098, blue: 0.114)
    static let raised = Color(red: 0.13, green: 0.14, blue: 0.16)
    static let stroke = Color.white.opacity(0.08)
    static let text = Color(red: 0.95, green: 0.96, blue: 0.97)
    static let secondary = Color(red: 0.61, green: 0.64, blue: 0.69)
    static let failure = Color(red: 1, green: 0.42, blue: 0.4)
    // Playback and selection: playhead, loop, tab cursor, edit box.
    static let mint = Color(red: 0.36, green: 0.95, blue: 0.82)

    static func color(for stem: String) -> Color {
        switch stem {
        case "vocals": Color(red: 1, green: 0.42, blue: 0.6)
        case "guitar": Color(red: 1, green: 0.71, blue: 0.28)
        case "drums": Color(red: 0.65, green: 0.55, blue: 0.98)
        case "bass": Color(red: 0.22, green: 0.84, blue: 0.94)
        case "piano": Color(red: 0.43, green: 0.91, blue: 0.63)
        default: Color(red: 0.58, green: 0.64, blue: 0.72)
        }
    }

    static func icon(for stem: String) -> String {
        switch stem {
        case "vocals": "music.mic"
        case "guitar": "guitars"
        case "drums": "cylinder.split.1x2"
        case "bass": "hifispeaker"
        case "piano": "pianokeys"
        default: "music.note"
        }
    }
}

struct KeyChip: View {
    let text: String
    init(_ text: String) { self.text = text }

    var body: some View {
        if !text.isEmpty {
            Text(text)
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundStyle(Theme.secondary)
                .padding(.horizontal, 5).padding(.vertical, 1.5)
                .background(Theme.raised, in: .rect(cornerRadius: 4))
                .overlay(RoundedRectangle(cornerRadius: 4).stroke(Theme.stroke))
        }
    }
}

// Small square toggle used for mute and solo; `tint` is the fill when on.
struct PillToggleStyle: ButtonStyle {
    let isOn: Bool
    let tint: Color

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 11, weight: .bold))
            .frame(width: 24, height: 22)
            .foregroundStyle(isOn ? Color.black : Theme.secondary)
            .background(isOn ? tint : Theme.raised, in: .rect(cornerRadius: 6))
            .opacity(configuration.isPressed ? 0.7 : 1)
    }
}

// Borderless icon or text button on the dark chrome.
struct ChromeButtonStyle: ButtonStyle {
    var isOn = false
    var onColor = Theme.text

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(isOn ? Color.black : Theme.text)
            .padding(.horizontal, 10).frame(height: 30)
            .background(isOn ? onColor : Theme.raised.opacity(configuration.isPressed ? 1 : 0.7), in: .rect(cornerRadius: 8))
    }
}
