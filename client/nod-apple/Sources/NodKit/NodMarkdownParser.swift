import Foundation

public enum NodMarkdownBlock: Equatable, Sendable {
  case paragraph(String)
  case heading(level: Int, text: String)
  case unorderedItem(String)
  case orderedItem(marker: String, text: String)
  case quote(String)
  case code(String)
  case divider
}

public enum NodMarkdownParser {
  /// Parses only the block types Nod renders natively; inline Markdown stays as text.
  public static func blocks(from markdown: String) -> [NodMarkdownBlock] {
    var blocks: [NodMarkdownBlock] = []
    var paragraphLines: [String] = []
    var codeLines: [String] = []
    var isInCodeBlock = false

    func flushParagraph() {
      guard !paragraphLines.isEmpty else {
        return
      }
      blocks.append(.paragraph(paragraphLines.joined(separator: "\n")))
      paragraphLines = []
    }

    func flushCodeBlock() {
      blocks.append(.code(codeLines.joined(separator: "\n")))
      codeLines = []
    }

    let normalized = markdown.replacingOccurrences(of: "\r\n", with: "\n")
    for line in normalized.components(separatedBy: "\n") {
      let trimmed = line.trimmingCharacters(in: .whitespaces)

      if trimmed.hasPrefix("```") {
        if isInCodeBlock {
          flushCodeBlock()
        } else {
          flushParagraph()
        }
        isInCodeBlock.toggle()
        continue
      }

      if isInCodeBlock {
        codeLines.append(line)
        continue
      }

      if trimmed.isEmpty {
        flushParagraph()
        continue
      }

      if trimmed == "---" || trimmed == "***" {
        flushParagraph()
        blocks.append(.divider)
        continue
      }

      if let heading = heading(from: trimmed) {
        flushParagraph()
        blocks.append(.heading(level: heading.level, text: heading.text))
        continue
      }

      if let unorderedItem = unorderedItem(from: trimmed) {
        flushParagraph()
        blocks.append(.unorderedItem(unorderedItem))
        continue
      }

      if let orderedItem = orderedItem(from: trimmed) {
        flushParagraph()
        blocks.append(.orderedItem(marker: orderedItem.marker, text: orderedItem.text))
        continue
      }

      if let quote = quote(from: trimmed) {
        flushParagraph()
        blocks.append(.quote(quote))
        continue
      }

      paragraphLines.append(line)
    }

    if isInCodeBlock {
      flushCodeBlock()
    }
    flushParagraph()

    return blocks
  }

  private static func heading(from line: String) -> (level: Int, text: String)? {
    let level = line.prefix(while: { $0 == "#" }).count
    guard (1...6).contains(level) else {
      return nil
    }
    let index = line.index(line.startIndex, offsetBy: level)
    guard index < line.endIndex, line[index] == " " else {
      return nil
    }
    let textStart = line.index(after: index)
    return (level, String(line[textStart...]))
  }

  private static func unorderedItem(from line: String) -> String? {
    for marker in ["- ", "* ", "+ "] where line.hasPrefix(marker) {
      return String(line.dropFirst(marker.count))
    }
    return nil
  }

  private static func orderedItem(from line: String) -> (marker: String, text: String)? {
    var digitEnd = line.startIndex
    while digitEnd < line.endIndex, line[digitEnd].isNumber {
      digitEnd = line.index(after: digitEnd)
    }

    guard digitEnd > line.startIndex, digitEnd < line.endIndex else {
      return nil
    }

    let separator = line[digitEnd]
    guard separator == "." || separator == ")" else {
      return nil
    }

    let textStart = line.index(after: digitEnd)
    guard textStart < line.endIndex, line[textStart] == " " else {
      return nil
    }

    let marker = String(line[...digitEnd])
    let contentStart = line.index(after: textStart)
    let text = contentStart < line.endIndex ? String(line[contentStart...]) : ""
    return (marker, text)
  }

  private static func quote(from line: String) -> String? {
    guard line.hasPrefix(">") else {
      return nil
    }
    return line
      .dropFirst()
      .trimmingCharacters(in: .whitespaces)
  }
}
