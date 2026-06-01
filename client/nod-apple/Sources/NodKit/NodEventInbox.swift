import Foundation

public enum NodEventInbox {
  public static let handledEventDisplayLimit = 500

  public static func pendingCountsByChannel(in events: [NodEvent]) -> [String: Int] {
    Dictionary(
      grouping: events.filter { $0.status == .pending },
      by: \.channelId
    ).mapValues(\.count)
  }

  public static func newestFirst(_ events: [NodEvent]) -> [NodEvent] {
    events.sorted { lhs, rhs in
      if lhs.createdAt == rhs.createdAt {
        return lhs.id > rhs.id
      }
      return lhs.createdAt > rhs.createdAt
    }
  }

  public static func pendingFirst(_ events: [NodEvent]) -> [NodEvent] {
    events.sorted { lhs, rhs in
      if lhs.status == .pending, rhs.status != .pending {
        return true
      }
      if lhs.status != .pending, rhs.status == .pending {
        return false
      }
      if lhs.createdAt == rhs.createdAt {
        return lhs.id > rhs.id
      }
      return lhs.createdAt > rhs.createdAt
    }
  }

  public static func visibleEvents(
    _ events: [NodEvent],
    handledLimit: Int = handledEventDisplayLimit
  ) -> [NodEvent] {
    var handledCount = 0
    // Pending items are never hidden; the cap only trims older handled history.
    return pendingFirst(events).filter { event in
      guard event.status != .pending else {
        return true
      }
      handledCount += 1
      return handledCount <= handledLimit
    }
  }
}
