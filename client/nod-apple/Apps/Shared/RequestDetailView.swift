import NodKit
import SwiftUI

struct RequestDetail: View {
  @EnvironmentObject private var store: NodStore
  let request: NodRequest
  @State private var optionNeedingText: NodRequestOption?
  @State private var optionText = ""
  @State private var autoDismissedRequestIds = Set<String>()

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        HStack(alignment: .firstTextBaseline) {
          VStack(alignment: .leading, spacing: 4) {
            Text(request.title)
              .font(.title2)
              .fontWeight(.semibold)
            Text(request.createdAt, format: .dateTime)
              .font(.caption)
              .foregroundStyle(.secondary)
          }
          Spacer()
          StatusBadge(status: request.status)
        }

        if let imageURL = resolvedImageURL(request.imageUrl) {
          RequestImageView(url: imageURL)
        }

        if !request.bodyMarkdown.isEmpty {
          MarkdownText(markdown: request.bodyMarkdown)
            .textSelection(.enabled)
        }

        if !request.fields.isEmpty {
          Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 8) {
            ForEach(request.fields, id: \.self) { field in
              GridRow {
                Text(field.label)
                  .foregroundStyle(.secondary)
                Text(field.value)
                  .textSelection(.enabled)
              }
            }
          }
        }

        if !request.links.isEmpty {
          VStack(alignment: .leading, spacing: 8) {
            ForEach(request.links, id: \.self) { link in
              if let resolvedLink = resolvedLink(link) {
                RequestLinkView(label: resolvedLink.label, url: resolvedLink.url)
              }
            }
          }
        }

        if let decision = request.decision {
          RequestDecisionView(decision: decision)
        } else if request.status == .pending && !request.options.isEmpty {
          RequestOptionArea(options: request.options) { option, text in
            perform(option, text: text)
          }
        }
      }
      .padding()
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .navigationTitle(request.title)
    .task(id: request.id) {
      await dismissInformationalRequestIfNeeded(request)
    }
    .sheet(item: $optionNeedingText) { option in
      NavigationStack {
        Form {
          TextField(option.textPlaceholder ?? "Notes", text: $optionText, axis: .vertical)
            .lineLimit(4...8)
        }
        .navigationTitle(option.label)
        .toolbar {
          ToolbarItem(placement: .cancellationAction) {
            Button("Cancel") {
              optionNeedingText = nil
              optionText = ""
            }
          }
          ToolbarItem(placement: .confirmationAction) {
            Button("Submit") {
              let text = optionText
              Task {
                await store.submit(request: request, option: option, text: text)
                optionNeedingText = nil
                optionText = ""
              }
            }
          }
        }
      }
    }
  }

  private func perform(_ option: NodRequestOption, text: String? = nil) {
    if let text {
      Task { await store.submit(request: request, option: option, text: text) }
      return
    }

    if option.requiresText {
      optionNeedingText = option
    } else {
      Task { await store.submit(request: request, option: option) }
    }
  }

  private func dismissInformationalRequestIfNeeded(_ request: NodRequest) async {
    guard request.status == .pending, request.options.isEmpty else {
      return
    }
    guard !autoDismissedRequestIds.contains(request.id) else {
      return
    }
    autoDismissedRequestIds.insert(request.id)
    await store.dismissIfInformational(request: request)
  }
}

private struct RequestDecisionView: View {
  let decision: NodDecision

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text("Handled")
        .font(.headline)
      Text(decision.optionLabel)
      if let text = decision.text, !text.isEmpty {
        Text(text)
          .padding(10)
          .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
      }
    }
  }
}

private struct RequestOptionArea: View {
  let options: [NodRequestOption]
  let perform: (NodRequestOption, String?) -> Void
  @State private var inlineText = ""

  private var approveOption: NodRequestOption? {
    options.first { $0.kind == .approve }
  }

  private var approveTextOption: NodRequestOption? {
    options.first { $0.kind == .approveWithText }
  }

  private var rejectOption: NodRequestOption? {
    options.first { $0.kind == .reject }
  }

  private var rejectTextOption: NodRequestOption? {
    options.first { $0.kind == .rejectWithText }
  }

  private var approveOptions: [NodRequestOption] {
    options.filter { $0.kind == .approve || $0.kind == .approveWithText }
  }

  private var rejectOptions: [NodRequestOption] {
    options.filter { $0.kind == .reject || $0.kind == .rejectWithText }
  }

  private var includesInlineText: Bool {
    approveTextOption != nil || rejectTextOption != nil
  }

  private var trimmedInlineText: String {
    inlineText.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  private var inlineTextLabel: String {
    approveTextOption == nil && rejectTextOption != nil ? "Rejection reason" : "Notes"
  }

  private var inlineTextPlaceholder: String {
    let preferredOption: NodRequestOption?
    if approveTextOption == nil {
      preferredOption = rejectTextOption
    } else if rejectTextOption == nil {
      preferredOption = approveTextOption
    } else {
      preferredOption = nil
    }

    if let placeholder = preferredOption?.textPlaceholder?.trimmingCharacters(in: .whitespacesAndNewlines),
      !placeholder.isEmpty
    {
      return placeholder
    }

    return inlineTextLabel == "Rejection reason" ? "Add a reason" : "Add notes"
  }

  private var otherOptions: [NodRequestOption] {
    options.filter { option in
      switch option.kind {
      case .approve, .approveWithText, .reject, .rejectWithText:
        return false
      case .dismiss, .open, .custom:
        return true
      }
    }
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      if !approveOptions.isEmpty || !rejectOptions.isEmpty {
        pairedApprovalOptions
      }

      ForEach(otherOptions) { option in
        Button(role: option.destructive ? .destructive : nil) {
          perform(option, nil)
        } label: {
          Text(option.label)
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.borderedProminent)
      }
    }
  }

  private var pairedApprovalOptions: some View {
    ViewThatFits(in: .horizontal) {
      approvalControls(columnWidth: 168, centerGap: 64)
      approvalControls(columnWidth: 148, centerGap: 48)
      approvalControls(columnWidth: 128, centerGap: 32)
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }

  private func approvalControls(columnWidth: CGFloat, centerGap: CGFloat) -> some View {
    let totalWidth = columnWidth * 2 + centerGap

    return VStack(alignment: .leading, spacing: 10) {
      optionColumns(columnWidth: columnWidth, centerGap: centerGap)

      if includesInlineText {
        inlineTextField(width: totalWidth)
      }
    }
  }

  private func optionColumns(columnWidth: CGFloat, centerGap: CGFloat) -> some View {
    HStack(alignment: .top, spacing: centerGap) {
      optionTile(kind: .approve, tint: .green, systemImage: "checkmark", width: columnWidth)
      optionTile(kind: .reject, tint: .red, systemImage: "xmark", width: columnWidth)
    }
  }

  @ViewBuilder
  private func optionTile(kind: NodOptionKind, tint: Color, systemImage: String, width: CGFloat) -> some View {
    let height = width * 2.0 / 3.0
    if let label = label(for: kind) {
      Button(role: selectedOption(for: kind)?.destructive == true ? .destructive : nil) {
        submit(kind)
      } label: {
        VStack(spacing: 6) {
          Image(systemName: systemImage)
            .font(.title2.weight(.semibold))
          Text(label)
            .font(.subheadline.weight(.semibold))
            .lineLimit(2)
            .multilineTextAlignment(.center)
            .minimumScaleFactor(0.82)
        }
        .frame(width: width, height: height)
        .contentShape(RoundedRectangle(cornerRadius: 8))
      }
      .buttonStyle(RequestOptionTileButtonStyle(tint: tint))
      .disabled(isDisabled(kind))
    } else {
      Color.clear
        .frame(width: width, height: height)
    }
  }

  private func inlineTextField(width: CGFloat) -> some View {
    VStack(alignment: .leading, spacing: 5) {
      Text(inlineTextLabel)
        .font(.caption.weight(.medium))
        .foregroundStyle(.secondary)
      TextField(inlineTextPlaceholder, text: $inlineText, axis: .vertical)
        .lineLimit(2...5)
        .textFieldStyle(.roundedBorder)
    }
    .frame(width: width, alignment: .leading)
  }

  private func label(for kind: NodOptionKind) -> String? {
    switch kind {
    case .approve:
      if let approveOption {
        return approveOption.label
      }
      return approveTextOption == nil ? nil : "Approve"
    case .reject:
      if let rejectOption {
        return rejectOption.label
      }
      return rejectTextOption == nil ? nil : "Reject"
    case .approveWithText, .rejectWithText, .dismiss, .open, .custom:
      return nil
    }
  }

  private func selectedOption(for kind: NodOptionKind) -> NodRequestOption? {
    switch kind {
    case .approve:
      return !trimmedInlineText.isEmpty ? approveTextOption ?? approveOption : approveOption ?? approveTextOption
    case .reject:
      return !trimmedInlineText.isEmpty ? rejectTextOption ?? rejectOption : rejectOption ?? rejectTextOption
    case .approveWithText, .rejectWithText, .dismiss, .open, .custom:
      return nil
    }
  }

  private func isDisabled(_ kind: NodOptionKind) -> Bool {
    guard let option = selectedOption(for: kind) else {
      return true
    }

    return option.requiresText && trimmedInlineText.isEmpty
  }

  private func submit(_ kind: NodOptionKind) {
    guard let option = selectedOption(for: kind) else {
      return
    }

    let supportsInlineText = option.kind == .approveWithText || option.kind == .rejectWithText || option.requiresText
    let text = supportsInlineText && !trimmedInlineText.isEmpty ? inlineText : nil
    perform(option, text)
  }
}

private struct RequestOptionTileButtonStyle: ButtonStyle {
  let tint: Color
  @Environment(\.isEnabled) private var isEnabled

  func makeBody(configuration: Configuration) -> some View {
    configuration.label
      .foregroundStyle(tint)
      .background(
        tint.opacity(configuration.isPressed ? 0.18 : 0.12),
        in: RoundedRectangle(cornerRadius: 8)
      )
      .overlay {
        RoundedRectangle(cornerRadius: 8)
          .stroke(tint.opacity(configuration.isPressed ? 0.7 : 0.42), lineWidth: 1)
      }
      .scaleEffect(configuration.isPressed ? 0.98 : 1)
      .opacity(isEnabled ? 1 : 0.45)
  }
}
