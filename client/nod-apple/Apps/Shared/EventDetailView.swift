import NodKit
import SwiftUI

struct EventDetail: View {
  @EnvironmentObject private var store: NodStore
  let event: NodEvent
  @State private var actionNeedingText: NodAction?
  @State private var actionText = ""
  @State private var autoDismissedEventIds = Set<String>()

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        HStack(alignment: .firstTextBaseline) {
          VStack(alignment: .leading, spacing: 4) {
            Text(event.title)
              .font(.title2)
              .fontWeight(.semibold)
            Text(event.createdAt, format: .dateTime)
              .font(.caption)
              .foregroundStyle(.secondary)
          }
          Spacer()
          StatusBadge(status: event.status)
        }

        if let imageURL = resolvedImageURL(event.imageUrl) {
          EventImageView(url: imageURL)
        }

        if !event.bodyMarkdown.isEmpty {
          MarkdownText(markdown: event.bodyMarkdown)
            .textSelection(.enabled)
        }

        if !event.fields.isEmpty {
          Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 8) {
            ForEach(event.fields, id: \.self) { field in
              GridRow {
                Text(field.label)
                  .foregroundStyle(.secondary)
                Text(field.value)
                  .textSelection(.enabled)
              }
            }
          }
        }

        if !event.links.isEmpty {
          VStack(alignment: .leading, spacing: 8) {
            ForEach(event.links, id: \.self) { link in
              if let resolvedLink = resolvedLink(link) {
                EventLinkView(label: resolvedLink.label, url: resolvedLink.url)
              }
            }
          }
        }

        if let result = event.result {
          EventResultView(result: result)
        } else if event.status == .pending && !event.actions.isEmpty {
          EventActionArea(actions: event.actions) { action, text in
            perform(action, text: text)
          }
        }
      }
      .padding()
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .navigationTitle(event.title)
    .task(id: event.id) {
      await dismissInformationalEventIfNeeded(event)
    }
    .sheet(item: $actionNeedingText) { action in
      NavigationStack {
        Form {
          TextField(action.textPlaceholder ?? "Notes", text: $actionText, axis: .vertical)
            .lineLimit(4...8)
        }
        .navigationTitle(action.label)
        .toolbar {
          ToolbarItem(placement: .cancellationAction) {
            Button("Cancel") {
              actionNeedingText = nil
              actionText = ""
            }
          }
          ToolbarItem(placement: .confirmationAction) {
            Button("Submit") {
              let text = actionText
              Task {
                await store.submit(event: event, action: action, text: text)
                actionNeedingText = nil
                actionText = ""
              }
            }
          }
        }
      }
    }
  }

  private func perform(_ action: NodAction, text: String? = nil) {
    if let text {
      Task { await store.submit(event: event, action: action, text: text) }
      return
    }

    if action.requiresText {
      actionNeedingText = action
    } else {
      Task { await store.submit(event: event, action: action) }
    }
  }

  private func dismissInformationalEventIfNeeded(_ event: NodEvent) async {
    guard event.status == .pending, event.actions.isEmpty else {
      return
    }
    guard !autoDismissedEventIds.contains(event.id) else {
      return
    }
    autoDismissedEventIds.insert(event.id)
    await store.dismissIfInformational(event: event)
  }
}

private struct EventResultView: View {
  let result: NodEventResult

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text("Handled")
        .font(.headline)
      Text(result.actionLabel)
      if let text = result.text, !text.isEmpty {
        Text(text)
          .padding(10)
          .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
      }
    }
  }
}

private struct EventActionArea: View {
  let actions: [NodAction]
  let perform: (NodAction, String?) -> Void
  @State private var inlineText = ""

  private var approveAction: NodAction? {
    actions.first { $0.kind == .approve }
  }

  private var approveTextAction: NodAction? {
    actions.first { $0.kind == .approveWithText }
  }

  private var rejectAction: NodAction? {
    actions.first { $0.kind == .reject }
  }

  private var rejectTextAction: NodAction? {
    actions.first { $0.kind == .rejectWithText }
  }

  private var approveActions: [NodAction] {
    actions.filter { $0.kind == .approve || $0.kind == .approveWithText }
  }

  private var rejectActions: [NodAction] {
    actions.filter { $0.kind == .reject || $0.kind == .rejectWithText }
  }

  private var includesInlineText: Bool {
    approveTextAction != nil || rejectTextAction != nil
  }

  private var trimmedInlineText: String {
    inlineText.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  private var inlineTextLabel: String {
    approveTextAction == nil && rejectTextAction != nil ? "Rejection reason" : "Notes"
  }

  private var inlineTextPlaceholder: String {
    let preferredAction: NodAction?
    if approveTextAction == nil {
      preferredAction = rejectTextAction
    } else if rejectTextAction == nil {
      preferredAction = approveTextAction
    } else {
      preferredAction = nil
    }

    if let placeholder = preferredAction?.textPlaceholder?.trimmingCharacters(in: .whitespacesAndNewlines),
      !placeholder.isEmpty
    {
      return placeholder
    }

    return inlineTextLabel == "Rejection reason" ? "Add a reason" : "Add notes"
  }

  private var otherActions: [NodAction] {
    actions.filter { action in
      switch action.kind {
      case .approve, .approveWithText, .reject, .rejectWithText:
        return false
      case .dismiss, .open, .custom:
        return true
      }
    }
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      if !approveActions.isEmpty || !rejectActions.isEmpty {
        pairedApprovalActions
      }

      ForEach(otherActions) { action in
        Button(role: action.destructive ? .destructive : nil) {
          perform(action, nil)
        } label: {
          Text(action.label)
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.borderedProminent)
      }
    }
  }

  private var pairedApprovalActions: some View {
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
      actionColumns(columnWidth: columnWidth, centerGap: centerGap)

      if includesInlineText {
        inlineTextField(width: totalWidth)
      }
    }
  }

  private func actionColumns(columnWidth: CGFloat, centerGap: CGFloat) -> some View {
    HStack(alignment: .top, spacing: centerGap) {
      actionTile(kind: .approve, tint: .green, systemImage: "checkmark", width: columnWidth)
      actionTile(kind: .reject, tint: .red, systemImage: "xmark", width: columnWidth)
    }
  }

  @ViewBuilder
  private func actionTile(kind: NodActionKind, tint: Color, systemImage: String, width: CGFloat) -> some View {
    let height = width * 2.0 / 3.0
    if let label = label(for: kind) {
      Button(role: selectedAction(for: kind)?.destructive == true ? .destructive : nil) {
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
      .buttonStyle(EventActionTileButtonStyle(tint: tint))
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

  private func label(for kind: NodActionKind) -> String? {
    switch kind {
    case .approve:
      if let approveAction {
        return approveAction.label
      }
      return approveTextAction == nil ? nil : "Approve"
    case .reject:
      if let rejectAction {
        return rejectAction.label
      }
      return rejectTextAction == nil ? nil : "Reject"
    case .approveWithText, .rejectWithText, .dismiss, .open, .custom:
      return nil
    }
  }

  private func selectedAction(for kind: NodActionKind) -> NodAction? {
    switch kind {
    case .approve:
      return !trimmedInlineText.isEmpty ? approveTextAction ?? approveAction : approveAction ?? approveTextAction
    case .reject:
      return !trimmedInlineText.isEmpty ? rejectTextAction ?? rejectAction : rejectAction ?? rejectTextAction
    case .approveWithText, .rejectWithText, .dismiss, .open, .custom:
      return nil
    }
  }

  private func isDisabled(_ kind: NodActionKind) -> Bool {
    guard let action = selectedAction(for: kind) else {
      return true
    }

    return action.requiresText && trimmedInlineText.isEmpty
  }

  private func submit(_ kind: NodActionKind) {
    guard let action = selectedAction(for: kind) else {
      return
    }

    let supportsInlineText = action.kind == .approveWithText || action.kind == .rejectWithText || action.requiresText
    let text = supportsInlineText && !trimmedInlineText.isEmpty ? inlineText : nil
    perform(action, text)
  }
}

private struct EventActionTileButtonStyle: ButtonStyle {
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
