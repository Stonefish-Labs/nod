import Combine
import Foundation

public struct NodNotificationOpenRequest: Identifiable, Equatable, Sendable {
  public let id = UUID()
  public let requestId: String?
  public let channelId: String?
}

/// The SwiftUI-facing store. After the cutover onto the shared Rust runtime
/// (`nod-client-core`), this is a thin facade over `NodRuntimeClient`: the API
/// client, sync socket, persistence, and state machine all live once in Rust
/// (shared with the TUI + desktop). `NodStore` mirrors the runtime's emitted
/// `ClientState` into its existing `@Published` surface so the 9 SwiftUI views
/// compile unchanged, owns the UI-only inputs (URL/device-name/code drafts,
/// notification-permission status), and forwards user actions in as RPCs.
@MainActor
public final class NodStore: ObservableObject {
  // MARK: Data-plane (mirrored from the runtime's ClientState)
  @Published public var servers: [NodServerProfile] = []
  @Published public var selectedServerId: String?
  @Published public var currentUser: NodUser?
  @Published public var registeredDevices: [NodUserDevice] = []
  @Published public var channels: [NodChannel] = []
  @Published public var pendingCountsByChannel: [String: Int] = [:]
  @Published public var requests: [NodRequest] = []
  @Published public var notificationSound: String
  @Published public var lastError: String?
  @Published public var isRegistered: Bool = false
  @Published public var isSyncConnected: Bool = false
  @Published public internal(set) var notificationDeliveryMode: NodNotificationDeliveryMode = .push

  // The selection is owned by the UI (views bind to it directly) but is also
  // forwarded into the runtime so the shared state machine tracks it. The
  // `didSet` guards against feedback loops when we mirror runtime state back in.
  @Published public var selectedChannelId: String? {
    didSet {
      guard !isApplyingRuntimeState, selectedChannelId != oldValue else { return }
      if let selectedChannelId {
        let channelId = selectedChannelId
        Task { try? await runtime.selectChannel(channelId) }
      }
      recomputeVisibleRequests()
    }
  }
  @Published public var selectedRequestId: String? {
    didSet {
      guard !isApplyingRuntimeState, selectedRequestId != oldValue else { return }
      if let selectedRequestId {
        let requestId = selectedRequestId
        Task { try? await runtime.selectRequest(requestId) }
      }
    }
  }

  // MARK: UI-only inputs (never sourced from the runtime)
  @Published public var baseURLString: String
  @Published public var deviceName: String
  @Published public var enrollmentCode: String = ""
  @Published public var notificationPermissionIssue: String?
  @Published public var notificationAuthorizationStatus: NodNotificationAuthorizationStatus =
    .notDetermined
  @Published public var notificationOpenRequest: NodNotificationOpenRequest?
  @Published public var registrationPromptRequestId: UUID?
  @Published public internal(set) var reEnrollmentServerId: String?
  @Published public internal(set) var serverConnectionIssuesById: [String: String] = [:]

  public let platform: NodDevicePlatform
  public var presentLocalNotifications: Bool

  public var selectedServer: NodServerProfile? {
    guard let selectedServerId else {
      return servers.first
    }
    return servers.first { $0.id == selectedServerId } ?? servers.first
  }

  public var subscribedChannels: [NodChannel] {
    channels.filter(\.subscribed)
  }

  public var totalPendingCount: Int {
    pendingCountsByChannel.values.reduce(0, +)
  }

  public var alertMessage: String? {
    lastError ?? notificationPermissionIssue
  }

  public var canReEnrollInvalidSession: Bool {
    reEnrollmentServerId != nil
  }

  public func connectionIssue(for server: NodServerProfile) -> String? {
    serverConnectionIssuesById[server.id]
  }

  public static let notificationSoundOptions: [NodNotificationSoundOption] = [
    NodNotificationSoundOption(id: "default", label: "Default"),
    NodNotificationSoundOption(id: "nod_ping.wav", label: "Ping"),
    NodNotificationSoundOption(id: "nod_chime.wav", label: "Chime"),
    NodNotificationSoundOption(id: "nod_low.wav", label: "Low"),
    NodNotificationSoundOption(id: "silent", label: "Silent"),
  ]

  static let applePushProvider = "apple_apns"

  let signingKeys = NodSigningKeyStore()
  let appAttest: NodAppAttestationProviding
  let defaults = UserDefaults.standard

  /// The shared Rust runtime that now owns the entire client.
  let runtime: NodRuntimeClient
  private var cancellables = Set<AnyCancellable>()
  /// All requests visible across channels (the runtime's full snapshot), kept so
  /// `selectedChannelId` changes can re-filter without a round trip.
  private var allVisibleRequests: [NodRequest] = []
  /// Set while mirroring runtime state in, so `selectedChannelId`/`Id` `didSet`
  /// observers don't echo the change back into the runtime.
  private var isApplyingRuntimeState = false
  private var hasStarted = false
  /// Pending request ids already turned into a local notification, so a backlog
  /// isn't replayed as a burst. The runtime de-dups candidates, this guards a
  /// second time across app restarts within a session.
  var presentedNotificationRequestIds = Set<String>()
  /// The latest APNs token, cached so re-enrollment can forward it again.
  var pushToken: String?

  public init(
    platform: NodDevicePlatform,
    defaultDeviceName: String,
    presentLocalNotifications: Bool,
    appAttest: NodAppAttestationProviding = NodAppAttestationStore(),
    configureNotificationController: Bool = true
  ) {
    self.platform = platform
    self.presentLocalNotifications = presentLocalNotifications
    self.appAttest = appAttest

    let savedDeviceName = defaults.string(forKey: "nod.deviceName") ?? defaultDeviceName
    self.baseURLString = ""
    self.deviceName = savedDeviceName
    self.notificationSound = defaults.string(forKey: "nod.notificationSound") ?? "default"

    self.runtime = NodRuntimeClient(signer: SecureEnclaveDeviceSigner(store: signingKeys))

    if configureNotificationController {
      NodNotificationController.shared.configure(
        onOpen: { [weak self] requestId, channelId in
          Task { @MainActor in
            await self?.openNotification(requestId: requestId, channelId: channelId)
          }
        },
        onOption: { [weak self] requestId, optionId, text in
          await self?.submitNotificationOption(requestId: requestId, optionId: optionId, text: text)
        }
      )
    }

    subscribeToRuntime()
    Task { await self.startRuntimeIfNeeded() }
  }

  // MARK: - Runtime lifecycle

  func startRuntimeIfNeeded() async {
    guard !hasStarted else { return }
    hasStarted = true
    do {
      try await runtime.start()
      try? await runtime.refresh()
    } catch {
      lastError = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
    }
  }

  private func subscribeToRuntime() {
    runtime.$state
      .receive(on: RunLoop.main)
      .sink { [weak self] state in
        guard let self, let state else { return }
        self.apply(runtimeState: state)
      }
      .store(in: &cancellables)

    runtime.$notificationCandidates
      .receive(on: RunLoop.main)
      .sink { [weak self] candidates in
        guard let self, !candidates.isEmpty else { return }
        let drained = self.runtime.takeNotificationCandidates()
        Task { @MainActor in await self.presentNotificationCandidates(drained) }
      }
      .store(in: &cancellables)

    runtime.$removedNotificationRequestIds
      .receive(on: RunLoop.main)
      .sink { [weak self] removed in
        guard let self, !removed.isEmpty else { return }
        let drained = self.runtime.takeRemovedNotificationRequestIds()
        for requestId in drained {
          self.presentedNotificationRequestIds.remove(requestId)
        }
      }
      .store(in: &cancellables)

    runtime.$authRevoked
      .receive(on: RunLoop.main)
      .sink { [weak self] revoked in
        guard let self, revoked else { return }
        self.handleAuthRevoked()
      }
      .store(in: &cancellables)

    runtime.$lastTransientError
      .receive(on: RunLoop.main)
      .compactMap { $0 }
      .sink { [weak self] message in
        self?.lastError = message
      }
      .store(in: &cancellables)
  }

  /// Mirror the runtime's `ClientState` into the view-facing surface.
  private func apply(runtimeState state: NodRuntimeState) {
    isApplyingRuntimeState = true
    defer { isApplyingRuntimeState = false }

    servers = state.servers.map { profile in
      NodServerProfile(
        id: profile.id,
        name: profile.name,
        baseURLString: profile.baseUrlString,
        deviceName: profile.deviceName,
        deviceId: profile.deviceId,
        userId: profile.userId,
        userName: profile.userName
      )
    }
    selectedServerId = state.selectedServerId
    currentUser = state.currentUser
    registeredDevices = state.devices
    channels = state.channels
    pendingCountsByChannel = state.pendingCountsByChannel
    notificationSound = state.notificationSound
    notificationDeliveryMode = state.notificationDeliveryMode
    isRegistered = state.isRegistered
    isSyncConnected = state.isSyncConnected

    if selectedChannelId != state.selectedChannelId {
      selectedChannelId = state.selectedChannelId
    }
    if selectedRequestId != state.selectedRequestId {
      selectedRequestId = state.selectedRequestId
    }

    allVisibleRequests = state.requests
    recomputeVisibleRequests()

    if let error = state.lastError {
      lastError = error
    }
  }

  /// The views expect `requests` to be the selected channel's visible items.
  private func recomputeVisibleRequests() {
    guard let selectedChannelId else {
      requests = []
      return
    }
    requests = NodRequestInbox.visibleRequests(
      allVisibleRequests.filter { $0.channelId == selectedChannelId }
    )
  }

  // MARK: - View-facing actions

  public func dismissAlertMessage() {
    if lastError != nil {
      lastError = nil
      reEnrollmentServerId = nil
    } else {
      notificationPermissionIssue = nil
    }
  }

  public func selectServer(_ serverId: String) {
    guard selectedServerId != serverId else {
      return
    }
    selectedServerId = serverId
    Task { try? await runtime.selectServer(serverId) }
  }

  public func refresh() async {
    do {
      try await runtime.refresh()
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  /// Account/device metadata lives in the same `ClientState`, so a plain refresh
  /// is enough; kept as a separate method to preserve the view surface.
  public func refreshAccount() async {
    await refresh()
  }

  public func resumeFromForeground() async {
    guard isRegistered else {
      return
    }
    await refresh()
    connectSync()
  }

  public func connectSync() {
    Task { try? await runtime.connectSync() }
  }

  public func disconnectSync() {
    Task { try? await runtime.disconnectSync() }
  }

  public func submit(request: NodRequest, option: NodRequestOption, text: String? = nil) async {
    do {
      try await runtime.submitOption(requestId: request.id, optionId: option.id, text: text)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  public func dismissIfInformational(request: NodRequest) async {
    // Informational items (pending with no options) are dismissed on open as a
    // read receipt. Best-effort: a failed acknowledgement should not surface.
    guard request.status == .pending, request.options.isEmpty else {
      return
    }
    try? await runtime.submitOption(requestId: request.id, optionId: "dismiss", text: nil)
  }

  public func setSubscription(channelId: String, subscribed: Bool) async {
    do {
      try await runtime.setSubscription(channelId: channelId, subscribed: subscribed)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  public func setNotificationSound(_ sound: String) async {
    notificationSound = sound
    defaults.set(sound, forKey: "nod.notificationSound")
    do {
      try await runtime.setNotificationPreference(sound: sound)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  public func clearSelectedChannel() async {
    guard let selectedChannelId else {
      return
    }
    do {
      try await runtime.clearChannel(selectedChannelId)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  public func renameDevice(_ device: NodUserDevice, name: String) async {
    let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else {
      return
    }
    do {
      try await runtime.renameDevice(deviceId: device.id, name: trimmed)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  public func revokeDevice(_ device: NodUserDevice) async {
    do {
      try await runtime.revokeDevice(device.id)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  public func revokeCurrentDevice() async {
    guard let server = selectedServer, let deviceId = server.deviceId else {
      return
    }
    do {
      try await runtime.revokeDevice(deviceId)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  public func forgetServers(_ serverIds: [String]) {
    for serverId in serverIds {
      try? appAttest.delete(account: Self.appAttestKeyAccount(for: serverId))
    }
    Task {
      for serverId in serverIds {
        try? await runtime.forgetServer(serverId)
      }
    }
  }

  public func beginInvalidSessionReEnrollment() {
    guard
      let serverId = reEnrollmentServerId,
      let server = servers.first(where: { $0.id == serverId })
    else {
      reEnrollmentServerId = nil
      return
    }

    let shouldPromptRegistration = servers.contains { $0.id != server.id }
    baseURLString = server.baseURLString
    deviceName = server.deviceName
    enrollmentCode = ""
    lastError = nil
    reEnrollmentServerId = nil
    try? appAttest.delete(account: Self.appAttestKeyAccount(for: server.id))
    Task { try? await runtime.forgetServer(server.id) }

    if shouldPromptRegistration {
      registrationPromptRequestId = UUID()
    }
  }

  /// Enroll the current draft (URL/device-name/code) with the server.
  ///
  /// App Attest still runs natively here, before handing off to the runtime: the
  /// Secure Enclave decision-signing key is provisioned locally (the runtime's
  /// signer callback uses the same keychain account, so it sees the same key),
  /// its public key feeds the App Attest `clientDataHash`, and the resulting
  /// attestation blob is forwarded to the runtime's `enroll` RPC.
  public func register(pushToken: String? = nil) async {
    await startRuntimeIfNeeded()
    do {
      guard !baseURLString.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
        throw NodStoreError.invalidServerURL
      }

      let normalizedURL = NodServerAddress.normalizedBaseURL(baseURLString)
      guard URL(string: normalizedURL) != nil else {
        throw NodStoreError.invalidServerURL
      }

      let registrationPushToken = pushToken ?? self.pushToken
      let nativeAppId = try Self.nativeAppId(requiredForPushToken: registrationPushToken)
      let profileId = NodServerAddress.profileId(for: normalizedURL)
      // Load-or-create the Secure Enclave signing key for this profile. The
      // runtime's signer callback keys off the same account, so this mints the
      // exact key the runtime will use to sign decisions.
      let signingKey = try signingKeys.signingKey(account: Self.signingKeyAccount(for: profileId))
      let attestationRequest = NodAppAttestationRequest(
        code: normalizedEnrollmentCode,
        deviceName: deviceName,
        platform: platform,
        pushProvider: registrationPushToken == nil ? nil : Self.applePushProvider,
        pushToken: registrationPushToken,
        signingKey: signingKey,
        account: Self.appAttestKeyAccount(for: profileId)
      )
      // App Attest hardens enrollment when Apple can issue an attestation, but the
      // Secure Enclave decision-signing key remains the required device identity.
      let attestation = try? await appAttest.enrollmentAttestation(for: attestationRequest)

      try await runtime.enroll(
        baseURL: normalizedURL,
        deviceName: deviceName,
        code: normalizedEnrollmentCode,
        notificationSound: notificationSound,
        platform: platform,
        nativeAppId: nativeAppId,
        pushProvider: registrationPushToken == nil ? nil : Self.applePushProvider,
        pushToken: registrationPushToken,
        attestation: attestation.map(Self.attestationDictionary)
      )
      enrollmentCode = ""
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  public func registerPushToken(_ token: String) async {
    pushToken = token
    guard let nativeAppId = try? Self.nativeAppId(requiredForPushToken: token) else {
      lastError = NodStoreError.missingNativeAppId.localizedDescription
      return
    }
    await startRuntimeIfNeeded()
    do {
      try await runtime.registerPushToken(
        provider: Self.applePushProvider, nativeAppId: nativeAppId, token: token)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  private static func attestationDictionary(_ attestation: NodDeviceAttestation) -> [String: Any] {
    [
      "provider": attestation.provider,
      "key_id": attestation.keyId,
      "attestation_object": attestation.attestationObject,
    ]
  }

  func submitNotificationOption(requestId: String, optionId: String, text: String?) async {
    do {
      try await runtime.submitOption(requestId: requestId, optionId: optionId, text: text)
      lastError = nil
    } catch {
      mapRuntimeError(error)
    }
  }

  // MARK: - Error mapping / auth

  func mapRuntimeError(_ error: Error) {
    switch error {
    case NodRuntimeError.rpc(let message):
      lastError = message
    default:
      lastError = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
    }
  }

  private func handleAuthRevoked() {
    if let server = selectedServer {
      let message =
        "Your Nod session with \(server.name) is no longer valid. Re-enroll this device to continue."
      reEnrollmentServerId = server.id
      var issues = serverConnectionIssuesById
      issues[server.id] = message
      serverConnectionIssuesById = issues
      lastError = message
    } else {
      lastError = "Your Nod session is no longer valid. Re-enroll this device to continue."
      reEnrollmentServerId = nil
    }
  }

  // MARK: - Keychain namespaces

  static func signingKeyAccount(for serverId: String) -> String {
    "decisionSigningKey.\(serverId)"
  }

  static func appAttestKeyAccount(for serverId: String) -> String {
    "appAttestKey.\(serverId)"
  }

  static func nativeAppId(requiredForPushToken pushToken: String?) throws -> String? {
    let nativeAppId = Bundle.main.bundleIdentifier?
      .trimmingCharacters(in: .whitespacesAndNewlines)
    guard let nativeAppId, !nativeAppId.isEmpty else {
      if pushToken == nil {
        return nil
      }
      throw NodStoreError.missingNativeAppId
    }
    return nativeAppId
  }

  var normalizedEnrollmentCode: String {
    enrollmentCode.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
  }
}

public enum NodStoreError: Error, LocalizedError {
  case missingNativeAppId
  case invalidServerURL

  public var errorDescription: String? {
    switch self {
    case .missingNativeAppId:
      return "This app is missing a bundle identifier for push registration."
    case .invalidServerURL:
      return "The Nod server URL is invalid."
    }
  }
}
