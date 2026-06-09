import Foundation

public enum NodAPIError: Error, LocalizedError {
    case missingToken
    case badURL
    case missingNativeAppId
    case badStatus(Int, String)

    public var errorDescription: String? {
        switch self {
        case .missingToken:
            return "This device is not registered yet."
        case .badURL:
            return "The Nod server URL is invalid."
        case .missingNativeAppId:
            return "This app is missing a bundle identifier for push registration."
        case let .badStatus(status, body):
            return "Nod request failed with \(status): \(body)"
        }
    }
}

public final class NodAPI: @unchecked Sendable {
    public var baseURL: URL
    public var token: String?

    private let session: URLSession
    private let decoder: JSONDecoder
    private let encoder: JSONEncoder

    public init(baseURL: URL, token: String? = nil, session: URLSession = .shared) {
        self.baseURL = baseURL
        self.token = token
        self.session = session
        self.decoder = JSONDecoder.nod
        self.encoder = JSONEncoder.nod
    }

    public func enroll(_ enrollment: NodEnrollmentRequest) async throws -> EnrollDeviceResponse {
        return try await request(
            .post,
            path: "/api/v1/enroll",
            body: enrollment,
            requiresToken: false
        )
    }

    public func currentUser() async throws -> CurrentUserResponse {
        try await request(.get, path: "/api/v1/users/me")
    }

    public func devices() async throws -> [NodUserDevice] {
        let response: UserDevicesResponse = try await request(.get, path: "/api/v1/users/me/devices")
        return response.devices
    }

    public func renameDevice(id: String, name: String) async throws -> NodUserDevice {
        struct Body: Encodable {
            let name: String
        }
        let response: UserDeviceResponse = try await request(
            .put,
            path: "/api/v1/users/me/devices/\(id)",
            body: Body(name: name)
        )
        return response.device
    }

    public func revokeDevice(id: String) async throws {
        try await requestEmpty(.delete, path: "/api/v1/users/me/devices/\(id)")
    }

    public func sources() async throws -> [NodSource] {
        let response: SourcesResponse = try await request(.get, path: "/api/v1/sources")
        return response.sources
    }

    public func requests(_ query: NodRequestQuery = .activeOnly) async throws -> [NodRequest] {
        var queryItems: [URLQueryItem] = [
            URLQueryItem(name: "include_cleared", value: query.includeCleared ? "true" : "false")
        ]
        if let sourceId = query.sourceId {
            queryItems.append(URLQueryItem(name: "source_id", value: sourceId))
        }
        if let limit = query.limit {
            queryItems.append(URLQueryItem(name: "limit", value: String(limit)))
        }
        let response: RequestsResponse = try await request(.get, path: "/api/v1/requests", query: queryItems)
        return response.requests
    }

    public func request(id: String) async throws -> NodRequest {
        let response: RequestResponse = try await request(.get, path: "/api/v1/requests/\(id)")
        return response.request
    }

    public func submit(
        requestId: String,
        optionId: String,
        text: String? = nil,
        signature: NodDecisionSignature
    ) async throws -> NodRequest {
        struct Body: Encodable {
            let text: String?
            let signature: NodDecisionSignature
        }
        let response: RequestResponse = try await request(
            .post,
            path: "/api/v1/requests/\(requestId.pathComponentEscaped)/options/\(optionId.pathComponentEscaped)",
            body: Body(text: text, signature: signature)
        )
        return response.request
    }

    public func clear(sourceId: String) async throws {
        try await requestEmpty(.post, path: "/api/v1/devices/me/sources/\(sourceId)/clear")
    }

    public func updateSubscription(sourceId: String, subscribed: Bool) async throws {
        struct Body: Encodable {
            let subscribed: Bool
        }
        try await requestEmpty(
            .put,
            path: "/api/v1/devices/me/subscriptions/\(sourceId)",
            body: Body(subscribed: subscribed)
        )
    }

    public func updatePushToken(provider: String, nativeAppId: String, token: String) async throws {
        struct Body: Encodable {
            let provider: String
            let nativeAppId: String
            let token: String

            enum CodingKeys: String, CodingKey {
                case provider, token
                case nativeAppId = "native_app_id"
            }
        }
        try await requestEmpty(
            .put,
            path: "/api/v1/devices/me/push-token",
            body: Body(provider: provider, nativeAppId: nativeAppId, token: token)
        )
    }

    public func updateDevicePreferences(notificationSound: String) async throws {
        struct Body: Encodable {
            let notificationSound: String

            enum CodingKeys: String, CodingKey {
                case notificationSound = "notification_sound"
            }
        }
        try await requestEmpty(
            .put,
            path: "/api/v1/devices/me/preferences",
            body: Body(notificationSound: notificationSound)
        )
    }

    public func websocketURL() throws -> URL {
        guard let token else {
            throw NodAPIError.missingToken
        }
        var components = URLComponents(url: baseURL, resolvingAgainstBaseURL: false)
        components?.scheme = baseURL.scheme == "https" ? "wss" : "ws"
        components?.path = joinedPath("/api/v1/sync")
        components?.queryItems = [URLQueryItem(name: "token", value: token)]
        guard let url = components?.url else {
            throw NodAPIError.badURL
        }
        return url
    }

    private func request<Response: Decodable>(
        _ method: NodHTTPMethod,
        path: String,
        query: [URLQueryItem] = [],
        requiresToken: Bool = true
    ) async throws -> Response {
        try await request(method, path: path, query: query, bodyData: nil, requiresToken: requiresToken)
    }

    private func request<Body: Encodable, Response: Decodable>(
        _ method: NodHTTPMethod,
        path: String,
        query: [URLQueryItem] = [],
        body: Body,
        requiresToken: Bool = true
    ) async throws -> Response {
        try await request(
            method,
            path: path,
            query: query,
            bodyData: encoder.encode(body),
            requiresToken: requiresToken
        )
    }

    private func request<Response: Decodable>(
        _ method: NodHTTPMethod,
        path: String,
        query: [URLQueryItem],
        bodyData: Data?,
        requiresToken: Bool
    ) async throws -> Response {
        var components = URLComponents(url: baseURL, resolvingAgainstBaseURL: false)
        components?.path = joinedPath(path)
        components?.queryItems = query.isEmpty ? nil : query
        guard let url = components?.url else {
            throw NodAPIError.badURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = method.rawValue
        if let bodyData {
            request.httpBody = bodyData
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        if requiresToken {
            guard let token else {
                throw NodAPIError.missingToken
            }
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }

        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw NodAPIError.badStatus(-1, "Missing HTTP response")
        }
        guard (200..<300).contains(http.statusCode) else {
            throw NodAPIError.badStatus(http.statusCode, String(data: data, encoding: .utf8) ?? "")
        }
        return try decoder.decode(Response.self, from: data)
    }

    private func requestEmpty(
        _ method: NodHTTPMethod,
        path: String,
        query: [URLQueryItem] = [],
        requiresToken: Bool = true
    ) async throws {
        try await requestRaw(
            method,
            path: path,
            query: query,
            bodyData: nil,
            requiresToken: requiresToken
        )
    }

    private func requestEmpty<Body: Encodable>(
        _ method: NodHTTPMethod,
        path: String,
        query: [URLQueryItem] = [],
        body: Body,
        requiresToken: Bool = true
    ) async throws {
        try await requestRaw(
            method,
            path: path,
            query: query,
            bodyData: encoder.encode(body),
            requiresToken: requiresToken
        )
    }

    private func requestRaw(
        _ method: NodHTTPMethod,
        path: String,
        query: [URLQueryItem],
        bodyData: Data?,
        requiresToken: Bool
    ) async throws {
        var components = URLComponents(url: baseURL, resolvingAgainstBaseURL: false)
        components?.path = joinedPath(path)
        components?.queryItems = query.isEmpty ? nil : query
        guard let url = components?.url else {
            throw NodAPIError.badURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = method.rawValue
        if let bodyData {
            request.httpBody = bodyData
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        if requiresToken {
            guard let token else {
                throw NodAPIError.missingToken
            }
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }

        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw NodAPIError.badStatus(-1, "Missing HTTP response")
        }
        guard (200..<300).contains(http.statusCode) else {
            throw NodAPIError.badStatus(http.statusCode, String(data: data, encoding: .utf8) ?? "")
        }
    }

    private func joinedPath(_ path: String) -> String {
        let basePath = baseURL.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        let childPath = path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        let parts = [basePath, childPath].filter { !$0.isEmpty }
        return "/" + parts.joined(separator: "/")
    }
}

private enum NodHTTPMethod: String {
    case delete = "DELETE"
    case get = "GET"
    case post = "POST"
    case put = "PUT"
}

private extension String {
    var pathComponentEscaped: String {
        addingPercentEncoding(
            withAllowedCharacters: .urlPathAllowed.subtracting(CharacterSet(charactersIn: "/"))
        ) ?? self
    }
}

public struct EmptyResponse: Codable, Sendable {
    public init() {}
}

extension JSONDecoder {
    static var nod: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .custom { decoder in
            let container = try decoder.singleValueContainer()
            let value = try container.decode(String.self)
            if let date = NodDateParser.date(from: value) {
                return date
            }
            throw DecodingError.dataCorruptedError(in: container, debugDescription: "Invalid ISO8601 date: \(value)")
        }
        return decoder
    }
}

extension JSONEncoder {
    static var nod: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        return encoder
    }
}

private enum NodDateParser {
    static func date(from value: String) -> Date? {
        if let date = iso8601Date(from: value, fractionalSeconds: true) {
            return date
        }
        if let normalized = normalizedMillisecondsTimestamp(value),
           let date = iso8601Date(from: normalized, fractionalSeconds: true) {
            return date
        }
        return iso8601Date(from: value, fractionalSeconds: false)
    }

    private static func iso8601Date(from value: String, fractionalSeconds: Bool) -> Date? {
        let formatter = ISO8601DateFormatter()
        if fractionalSeconds {
            formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        } else {
            formatter.formatOptions = [.withInternetDateTime]
        }
        return formatter.date(from: value)
    }

    private static func normalizedMillisecondsTimestamp(_ value: String) -> String? {
        guard let dot = value.firstIndex(of: ".") else {
            return nil
        }

        var cursor = value.index(after: dot)
        var digits = ""
        while cursor < value.endIndex, value[cursor].isNumber {
            digits.append(value[cursor])
            cursor = value.index(after: cursor)
        }

        guard !digits.isEmpty, cursor < value.endIndex else {
            return nil
        }

        let milliseconds = String(digits.prefix(3)).padding(toLength: 3, withPad: "0", startingAt: 0)
        return "\(value[..<dot]).\(milliseconds)\(value[cursor...])"
    }
}
