import Foundation

public enum NodRequestInbox {
  public static let handledRequestDisplayLimit = 500

  public static func pendingCountsBySource(in requests: [NodRequest]) -> [String: Int] {
    Dictionary(
      grouping: requests.filter { $0.status == .pending },
      by: \.sourceId
    ).mapValues(\.count)
  }

  public static func newestFirst(_ requests: [NodRequest]) -> [NodRequest] {
    requests.sorted { lhs, rhs in
      if lhs.createdAt == rhs.createdAt {
        return lhs.id > rhs.id
      }
      return lhs.createdAt > rhs.createdAt
    }
  }

  public static func pendingFirst(_ requests: [NodRequest]) -> [NodRequest] {
    requests.sorted { lhs, rhs in
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

  public static func visibleRequests(
    _ requests: [NodRequest],
    handledLimit: Int = handledRequestDisplayLimit
  ) -> [NodRequest] {
    var handledCount = 0
    // Pending items are never hidden; the cap only trims older handled history.
    return pendingFirst(requests).filter { request in
      guard request.status != .pending else {
        return true
      }
      handledCount += 1
      return handledCount <= handledLimit
    }
  }
}
