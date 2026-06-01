import NodKit
import SwiftUI

struct RegistrationView: View {
  @Environment(\.dismiss) private var dismiss
  @EnvironmentObject private var store: NodStore

  var body: some View {
    Form {
      Section("Server") {
        TextField("Server URL", text: $store.baseURLString)
          #if os(iOS)
          .textInputAutocapitalization(.never)
          .autocorrectionDisabled(true)
          .keyboardType(.URL)
          #endif
      }

      Section("Device") {
        TextField("Device Name", text: $store.deviceName)
          #if os(iOS)
          .textInputAutocapitalization(.words)
          #endif
      }

      Section("Enrollment Code") {
        EnrollmentCodeInput(code: $store.enrollmentCode)
        Button {
          Task {
            await store.register()
            if store.isRegistered {
              dismiss()
            }
          }
        } label: {
          Label("Register Device", systemImage: "person.badge.key")
        }
        .buttonStyle(.borderedProminent)
        .disabled(
          store.enrollmentCode.count < 8 ||
            store.baseURLString.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ||
            store.deviceName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        )
      }
    }
    .formStyle(.grouped)
    .navigationTitle("Register Device")
  }
}

struct EnrollmentCodeInput: View {
  @Binding var code: String
  @FocusState private var focused: Bool
  private let length = 8

  var body: some View {
    ZStack {
      HStack(spacing: 8) {
        ForEach(0..<length, id: \.self) { index in
          Text(character(at: index))
            .font(.system(size: 22, weight: .semibold, design: .monospaced))
            .frame(width: 34, height: 42)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
            .overlay {
              RoundedRectangle(cornerRadius: 8)
                .stroke(focused ? Color.accentColor : Color.secondary.opacity(0.35), lineWidth: focused ? 2 : 1)
            }
        }
      }
      .contentShape(Rectangle())
      .onTapGesture {
        focused = true
      }

      TextField("", text: $code)
        .focused($focused)
        .textContentType(.oneTimeCode)
        #if os(iOS)
        .textInputAutocapitalization(.characters)
        .autocorrectionDisabled(true)
        .keyboardType(.asciiCapable)
        #endif
        .frame(width: 1, height: 1)
        .opacity(0.01)
        .onChange(of: code) { _, newValue in
          code = sanitized(newValue)
        }
    }
    .padding(.vertical, 4)
    .accessibilityLabel("Enrollment code")
  }

  private func character(at index: Int) -> String {
    let characters = Array(code)
    guard index < characters.count else {
      return ""
    }
    return String(characters[index])
  }

  private func sanitized(_ value: String) -> String {
    value.uppercased()
      .filter { $0.isLetter || $0.isNumber }
      .prefix(length)
      .map(String.init)
      .joined()
  }
}
