import NodKit
import SwiftUI

struct ChannelEventsView: View {
  @EnvironmentObject private var store: NodStore
  let channelId: String
  @State private var pendingEventsExpanded = true
  @State private var handledEventsExpanded = false
  @State private var initializedSectionExpansion = false

  var body: some View {
    Group {
      if channelEvents.isEmpty {
        ContentUnavailableView("No Notifications", systemImage: "bell.slash")
      } else {
        List {
          eventSections { event in
            NavigationLink {
              EventDetailContainer(eventId: event.id)
            } label: {
              EventRow(event: event)
            }
          }
        }
        .refreshable {
          await store.refresh()
        }
      }
    }
    .navigationTitle(channelName)
    .task {
      if store.selectedChannelId != channelId {
        store.selectedChannelId = channelId
      }
      await store.refresh()
    }
    .onAppear {
      initializeSectionExpansionIfNeeded()
    }
    .onChange(of: channelEvents.isEmpty) {
      initializeSectionExpansionIfNeeded()
    }
    .onChange(of: pendingEvents.isEmpty) { _, hasNoPendingEvents in
      initializeSectionExpansionIfNeeded()
      if hasNoPendingEvents {
        handledEventsExpanded = true
      }
    }
  }

  private var channelEvents: [NodEvent] {
    NodEventInbox.newestFirst(store.events.filter { $0.channelId == channelId })
  }

  private var pendingEvents: [NodEvent] {
    channelEvents.filter { $0.status == .pending }
  }

  private var handledEvents: [NodEvent] {
    channelEvents.filter { $0.status != .pending }
  }

  private var channelName: String {
    store.channels.first(where: { $0.id == channelId })?.name ?? "Notifications"
  }

  @ViewBuilder
  private func eventSections<RowContent: View>(
    @ViewBuilder rowContent: @escaping (NodEvent) -> RowContent
  ) -> some View {
    if !pendingEvents.isEmpty {
      Section {
        if pendingEventsExpanded {
          ForEach(pendingEvents) { event in
            rowContent(event)
          }
        }
      } header: {
        EventSectionHeader(
          title: "Pending",
          count: pendingEvents.count,
          isExpanded: $pendingEventsExpanded
        )
      }
    }
    if !handledEvents.isEmpty {
      Section {
        if handledEventsExpanded {
          ForEach(handledEvents) { event in
            rowContent(event)
          }
        }
      } header: {
        EventSectionHeader(
          title: "Handled",
          count: handledEvents.count,
          isExpanded: $handledEventsExpanded
        )
      }
    }
  }

  private func initializeSectionExpansionIfNeeded() {
    guard !initializedSectionExpansion, !channelEvents.isEmpty else {
      return
    }
    handledEventsExpanded = pendingEvents.isEmpty
    initializedSectionExpansion = true
  }
}

struct EventListView: View {
  @EnvironmentObject private var store: NodStore
  @State private var pendingEventsExpanded = true
  @State private var handledEventsExpanded = false
  @State private var initializedSectionExpansion = false

  var body: some View {
    Group {
      if store.events.isEmpty {
        ContentUnavailableView("No Notifications", systemImage: "bell.slash")
      } else {
        List(selection: $store.selectedEventId) {
          eventSections { event in
            EventRow(event: event)
              .tag(Optional(event.id))
          }
        }
      }
    }
    .navigationTitle(selectedChannelName)
    .onAppear {
      initializeSectionExpansionIfNeeded()
    }
    .onChange(of: store.events.isEmpty) {
      initializeSectionExpansionIfNeeded()
    }
    .onChange(of: pendingEvents.isEmpty) { _, hasNoPendingEvents in
      initializeSectionExpansionIfNeeded()
      if hasNoPendingEvents {
        handledEventsExpanded = true
      }
    }
    .onChange(of: store.selectedChannelId) {
      pendingEventsExpanded = true
      handledEventsExpanded = false
      initializedSectionExpansion = false
      initializeSectionExpansionIfNeeded()
    }
  }

  private var selectedChannelName: String {
    store.channels.first(where: { $0.id == store.selectedChannelId })?.name ?? "Notifications"
  }

  private var pendingEvents: [NodEvent] {
    NodEventInbox.newestFirst(store.events.filter { $0.status == .pending })
  }

  private var handledEvents: [NodEvent] {
    NodEventInbox.newestFirst(store.events.filter { $0.status != .pending })
  }

  @ViewBuilder
  private func eventSections<RowContent: View>(
    @ViewBuilder rowContent: @escaping (NodEvent) -> RowContent
  ) -> some View {
    if !pendingEvents.isEmpty {
      Section {
        if pendingEventsExpanded {
          ForEach(pendingEvents) { event in
            rowContent(event)
          }
        }
      } header: {
        EventSectionHeader(
          title: "Pending",
          count: pendingEvents.count,
          isExpanded: $pendingEventsExpanded
        )
      }
    }
    if !handledEvents.isEmpty {
      Section {
        if handledEventsExpanded {
          ForEach(handledEvents) { event in
            rowContent(event)
          }
        }
      } header: {
        EventSectionHeader(
          title: "Handled",
          count: handledEvents.count,
          isExpanded: $handledEventsExpanded
        )
      }
    }
  }

  private func initializeSectionExpansionIfNeeded() {
    guard !initializedSectionExpansion, !store.events.isEmpty else {
      return
    }
    handledEventsExpanded = pendingEvents.isEmpty
    initializedSectionExpansion = true
  }
}

struct EventDetailContainer: View {
  @EnvironmentObject private var store: NodStore
  let eventId: String?

  var body: some View {
    if let event = event {
      EventDetail(event: event)
    } else {
      ContentUnavailableView("Select a Notification", systemImage: "rectangle.stack.badge.person.crop")
    }
  }

  private var event: NodEvent? {
    if let eventId, let event = store.events.first(where: { $0.id == eventId }) {
      return event
    }
    return store.events.first
  }
}

struct ChannelLabel: View {
  let channel: NodChannel

  var body: some View {
    Label(channel.name, systemImage: iconName(channel.icon))
  }
}

struct ChannelRow: View {
  let channel: NodChannel
  let pendingCount: Int

  var body: some View {
    HStack {
      ChannelLabel(channel: channel)
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

struct EventRow: View {
  let event: NodEvent

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack {
        Text(event.title)
          .font(.headline)
          .lineLimit(2)
        Spacer()
        StatusBadge(status: event.status)
      }
      Text(event.summary)
        .font(.subheadline)
        .foregroundStyle(.secondary)
        .lineLimit(2)
      Text(event.createdAt, style: .relative)
        .font(.caption)
        .foregroundStyle(.tertiary)
    }
    .padding(.vertical, 4)
  }
}

struct EventSectionHeader: View {
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
