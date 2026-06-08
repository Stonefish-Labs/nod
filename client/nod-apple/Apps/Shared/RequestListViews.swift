import NodKit
import SwiftUI

struct SourceRequestsView: View {
  @EnvironmentObject private var store: NodStore
  let sourceId: String
  @State private var pendingRequestsExpanded = true
  @State private var handledRequestsExpanded = false
  @State private var initializedSectionExpansion = false

  var body: some View {
    Group {
      if sourceRequests.isEmpty {
        ContentUnavailableView("No Requests", systemImage: "bell.slash")
      } else {
        List {
          requestSections { request in
            NavigationLink {
              RequestDetailContainer(requestId: request.id)
            } label: {
              RequestRow(request: request)
            }
          }
        }
        .refreshable {
          await store.refresh()
        }
      }
    }
    .navigationTitle(sourceName)
    .task {
      if store.selectedSourceId != sourceId {
        store.selectedSourceId = sourceId
      }
      await store.refresh()
    }
    .onAppear {
      initializeSectionExpansionIfNeeded()
    }
    .onChange(of: sourceRequests.isEmpty) {
      initializeSectionExpansionIfNeeded()
    }
    .onChange(of: pendingRequests.isEmpty) { _, hasNoPendingRequests in
      initializeSectionExpansionIfNeeded()
      if hasNoPendingRequests {
        handledRequestsExpanded = true
      }
    }
  }

  private var sourceRequests: [NodRequest] {
    NodRequestInbox.newestFirst(store.requests.filter { $0.sourceId == sourceId })
  }

  private var pendingRequests: [NodRequest] {
    sourceRequests.filter { $0.status == .pending }
  }

  private var handledRequests: [NodRequest] {
    sourceRequests.filter { $0.status != .pending }
  }

  private var sourceName: String {
    store.sources.first(where: { $0.id == sourceId })?.name ?? "Requests"
  }

  @ViewBuilder
  private func requestSections<RowContent: View>(
    @ViewBuilder rowContent: @escaping (NodRequest) -> RowContent
  ) -> some View {
    if !pendingRequests.isEmpty {
      Section {
        if pendingRequestsExpanded {
          ForEach(pendingRequests) { request in
            rowContent(request)
          }
        }
      } header: {
        RequestSectionHeader(
          title: "Pending",
          count: pendingRequests.count,
          isExpanded: $pendingRequestsExpanded
        )
      }
    }
    if !handledRequests.isEmpty {
      Section {
        if handledRequestsExpanded {
          ForEach(handledRequests) { request in
            rowContent(request)
          }
        }
      } header: {
        RequestSectionHeader(
          title: "Handled",
          count: handledRequests.count,
          isExpanded: $handledRequestsExpanded
        )
      }
    }
  }

  private func initializeSectionExpansionIfNeeded() {
    guard !initializedSectionExpansion, !sourceRequests.isEmpty else {
      return
    }
    handledRequestsExpanded = pendingRequests.isEmpty
    initializedSectionExpansion = true
  }
}

struct RequestListView: View {
  @EnvironmentObject private var store: NodStore
  @State private var pendingRequestsExpanded = true
  @State private var handledRequestsExpanded = false
  @State private var initializedSectionExpansion = false

  var body: some View {
    Group {
      if store.requests.isEmpty {
        ContentUnavailableView("No Requests", systemImage: "bell.slash")
      } else {
        List(selection: $store.selectedRequestId) {
          requestSections { request in
            RequestRow(request: request)
              .tag(Optional(request.id))
          }
        }
      }
    }
    .navigationTitle(selectedSourceName)
    .onAppear {
      initializeSectionExpansionIfNeeded()
    }
    .onChange(of: store.requests.isEmpty) {
      initializeSectionExpansionIfNeeded()
    }
    .onChange(of: pendingRequests.isEmpty) { _, hasNoPendingRequests in
      initializeSectionExpansionIfNeeded()
      if hasNoPendingRequests {
        handledRequestsExpanded = true
      }
    }
    .onChange(of: store.selectedSourceId) {
      pendingRequestsExpanded = true
      handledRequestsExpanded = false
      initializedSectionExpansion = false
      initializeSectionExpansionIfNeeded()
    }
  }

  private var selectedSourceName: String {
    store.sources.first(where: { $0.id == store.selectedSourceId })?.name ?? "Requests"
  }

  private var pendingRequests: [NodRequest] {
    NodRequestInbox.newestFirst(store.requests.filter { $0.status == .pending })
  }

  private var handledRequests: [NodRequest] {
    NodRequestInbox.newestFirst(store.requests.filter { $0.status != .pending })
  }

  @ViewBuilder
  private func requestSections<RowContent: View>(
    @ViewBuilder rowContent: @escaping (NodRequest) -> RowContent
  ) -> some View {
    if !pendingRequests.isEmpty {
      Section {
        if pendingRequestsExpanded {
          ForEach(pendingRequests) { request in
            rowContent(request)
          }
        }
      } header: {
        RequestSectionHeader(
          title: "Pending",
          count: pendingRequests.count,
          isExpanded: $pendingRequestsExpanded
        )
      }
    }
    if !handledRequests.isEmpty {
      Section {
        if handledRequestsExpanded {
          ForEach(handledRequests) { request in
            rowContent(request)
          }
        }
      } header: {
        RequestSectionHeader(
          title: "Handled",
          count: handledRequests.count,
          isExpanded: $handledRequestsExpanded
        )
      }
    }
  }

  private func initializeSectionExpansionIfNeeded() {
    guard !initializedSectionExpansion, !store.requests.isEmpty else {
      return
    }
    handledRequestsExpanded = pendingRequests.isEmpty
    initializedSectionExpansion = true
  }
}

struct RequestDetailContainer: View {
  @EnvironmentObject private var store: NodStore
  let requestId: String?

  var body: some View {
    if let request = request {
      RequestDetail(request: request)
    } else {
      ContentUnavailableView("Select a Request", systemImage: "rectangle.stack.badge.person.crop")
    }
  }

  private var request: NodRequest? {
    if let requestId, let request = store.requests.first(where: { $0.id == requestId }) {
      return request
    }
    return store.requests.first
  }
}

struct SourceLabel: View {
  let source: NodSource

  var body: some View {
    Label(source.name, systemImage: iconName(source.icon))
  }
}

struct SourceRow: View {
  let source: NodSource
  let pendingCount: Int

  var body: some View {
    HStack {
      SourceLabel(source: source)
      Spacer()
      if pendingCount > 0 {
        Text(pendingCount, format: .number)
          .font(.caption)
          .fontWeight(.semibold)
          .monospacedDigit()
          .padding(.horizontal, 7)
          .padding(.vertical, 3)
          .background(Color.accentColor, in: Capsule())
          .foregroundStyle(.white)
          .accessibilityLabel("\(pendingCount) pending")
      }
    }
  }
}

struct RequestRow: View {
  let request: NodRequest

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack {
        Text(request.title)
          .font(.headline)
          .lineLimit(2)
        Spacer()
        StatusBadge(status: request.status)
      }
      Text(request.summary)
        .font(.subheadline)
        .foregroundStyle(.secondary)
        .lineLimit(2)
      Text(request.createdAt, style: .relative)
        .font(.caption)
        .foregroundStyle(.tertiary)
    }
    .padding(.vertical, 4)
  }
}

struct RequestSectionHeader: View {
  let title: String
  let count: Int
  @Binding var isExpanded: Bool

  var body: some View {
    Button {
      withAnimation(.default) {
        isExpanded.toggle()
      }
    } label: {
      HStack(spacing: 6) {
        Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
          .font(.caption.weight(.semibold))
          .frame(width: 12)
        Text(title)
        Text(count, format: .number)
          .monospacedDigit()
          .foregroundStyle(.secondary)
        Spacer()
      }
      .contentShape(Rectangle())
    }
    .buttonStyle(.plain)
    .accessibilityLabel("\(title), \(count) notifications")
    .accessibilityValue(isExpanded ? "Expanded" : "Collapsed")
  }
}
