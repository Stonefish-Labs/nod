import NodKit
import SwiftUI

struct SubscriptionSheet: View {
  @Environment(\.dismiss) private var dismiss
  @EnvironmentObject private var store: NodStore

  var body: some View {
    NavigationStack {
      Form {
        Section("Notification Sound") {
          Picker("Sound", selection: Binding(
            get: { store.notificationSound },
            set: { sound in
              Task {
                await store.setNotificationSound(sound)
              }
            }
          )) {
            ForEach(NodStore.notificationSoundOptions) { option in
              Text(option.label).tag(option.id)
            }
          }
          #if os(macOS)
          Button {
            Task { await store.requestAndTestNotifications() }
          } label: {
            Label("Request/Test Notifications", systemImage: "bell.badge")
          }
          Button {
            openNodNotificationSettings()
          } label: {
            Label("Open Notification Settings", systemImage: "gear")
          }
          #endif
        }

        Section("Sources") {
          ForEach(store.sources) { source in
            Toggle(isOn: Binding(
              get: { store.sources.first(where: { $0.id == source.id })?.subscribed ?? false },
              set: { subscribed in
                Task {
                  await store.setSubscription(sourceId: source.id, subscribed: subscribed)
                }
              }
            )) {
              SourceLabel(source: source)
            }
          }
        }
      }
      .navigationTitle("Subscriptions")
      .toolbar {
        ToolbarItem(placement: .confirmationAction) {
          Button("Done") {
            dismiss()
          }
        }
      }
    }
  }
}
