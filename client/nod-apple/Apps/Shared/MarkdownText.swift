import NodKit
import SwiftUI

struct MarkdownText: View {
  let markdown: String

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      ForEach(Array(NodMarkdownParser.blocks(from: markdown).enumerated()), id: \.offset) { _, block in
        view(for: block)
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }

  @ViewBuilder
  private func view(for block: NodMarkdownBlock) -> some View {
    switch block {
    case .paragraph(let text):
      inlineText(text)
        .fixedSize(horizontal: false, vertical: true)
    case .heading(let level, let text):
      inlineText(text)
        .font(headingFont(for: level))
        .fontWeight(.semibold)
        .fixedSize(horizontal: false, vertical: true)
        .padding(.top, level <= 2 ? 4 : 0)
    case .unorderedItem(let text):
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text("•")
          .frame(width: 12, alignment: .trailing)
        inlineText(text)
          .fixedSize(horizontal: false, vertical: true)
      }
    case .orderedItem(let marker, let text):
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(marker)
          .monospacedDigit()
          .foregroundStyle(.secondary)
          .frame(width: 28, alignment: .trailing)
        inlineText(text)
          .fixedSize(horizontal: false, vertical: true)
      }
    case .quote(let text):
      HStack(alignment: .top, spacing: 8) {
        Rectangle()
          .fill(.secondary.opacity(0.35))
          .frame(width: 3)
        inlineText(text)
          .foregroundStyle(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }
    case .code(let text):
      Text(text)
        .font(.system(.body, design: .monospaced))
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 6))
    case .divider:
      Divider()
    }
  }

  private func inlineText(_ markdown: String) -> Text {
    if let attributed = try? AttributedString(markdown: markdown) {
      return Text(attributed)
    }
    return Text(markdown)
  }

  private func headingFont(for level: Int) -> Font {
    switch level {
    case 1:
      return .title3
    case 2:
      return .headline
    default:
      return .subheadline
    }
  }
}
