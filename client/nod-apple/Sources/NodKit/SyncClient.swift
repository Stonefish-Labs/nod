import Foundation

public final class NodSyncClient: @unchecked Sendable {
    public var onConnected: (@Sendable () -> Void)?
    public var onEnvelope: (@Sendable (NodSyncEnvelope) -> Void)?
    public var onError: (@Sendable (Error) -> Void)?

    private var task: URLSessionWebSocketTask?
    private let session: URLSession
    private let decoder = JSONDecoder.nod

    public init(session: URLSession = .shared) {
        self.session = session
    }

    public func connect(url: URL) {
        disconnect()
        let task = session.webSocketTask(with: url)
        self.task = task
        task.resume()
        task.sendPing { [weak self] error in
            guard let self else {
                return
            }
            guard self.task === task else {
                return
            }
            if let error {
                self.task = nil
                task.cancel(with: .goingAway, reason: nil)
                onError?(error)
            } else {
                onConnected?()
            }
        }
        receiveLoop(task)
    }

    public func disconnect() {
        let currentTask = task
        task = nil
        currentTask?.cancel(with: .goingAway, reason: nil)
    }

    private func receiveLoop(_ task: URLSessionWebSocketTask) {
        task.receive { [weak self] result in
            guard let self else {
                return
            }
            guard self.task === task else {
                return
            }
            switch result {
            case let .success(message):
                do {
                    let data: Data
                    switch message {
                    case let .string(text):
                        data = Data(text.utf8)
                    case let .data(raw):
                        data = raw
                    @unknown default:
                        data = Data()
                    }
                    if !data.isEmpty {
                        let envelope = try decoder.decode(NodSyncEnvelope.self, from: data)
                        onEnvelope?(envelope)
                    }
                } catch {
                    onError?(error)
                }
                receiveLoop(task)
            case let .failure(error):
                self.task = nil
                onError?(error)
            }
        }
    }
}
