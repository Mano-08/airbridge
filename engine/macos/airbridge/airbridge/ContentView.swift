//
//  ContentView.swift
//  airbridge
//
//  Created by Mano W on 27/06/26.
//

import SwiftUI
import engineFFI

// MARK: - Screen state

enum ActiveAction {
    case create
    case join
}

struct AppConfig {
    static let port: UInt16 = {
        for arg in CommandLine.arguments {
            if arg.hasPrefix("--port=") {
                let value = arg.replacingOccurrences(of: "--port=", with: "")
                if let parsed = UInt16(value) {
                    return parsed
                }
            }
        }
        return 50002 // default if no --port= arg given
    }()
}

struct ContentView: View {
    @State private var rooms: [Room] = []
    @State private var selectedRoom: Room?

    @State private var showPasscodePrompt: Bool = false
    @State private var passcodeInput: String = ""
    @State private var roomCodeInput: String = ""
    @State private var pendingAction: ActiveAction? = nil

    @State private var errorMessage: String? = nil
    @State private var isLoading: Bool = false

    @State private var waitingRoomId: String? = nil
    @State private var waitingPasscode: String? = nil

    var body: some View {
        NavigationSplitView {
            // MARK: Sidebar — list of rooms
            List(rooms, id: \.roomId, selection: $selectedRoom) { room in
                VStack(alignment: .leading, spacing: 2) {
                    Text(room.fileName.isEmpty ? room.roomId : room.fileName)
                        .font(.system(.body, design: .monospaced))
                        .lineLimit(1)

                    if room.total > 0 {
                        ProgressView(value: Double(room.sent), total: Double(room.total))
                            .progressViewStyle(.linear)
                    }

                    Text("Peer: \(room.peerIp):\(String(room.peerPort))")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .tag(room)
            }
            .navigationTitle("Rooms")
            .toolbar {
                ToolbarItem {
                    Button {
                        loadRooms()
                    } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                }
            }
            .task {
                loadRooms()
            }
        } detail: {
            // MARK: Detail pane
            VStack(spacing: 20) {
                if let waitingRoomId, let waitingPasscode {
                    WaitingForFriendContent(roomId: waitingRoomId, passcode: waitingPasscode)
                } else if let room = selectedRoom {
                    RoomDetailContent(room: room)
                } else {
                    VStack(spacing: 16) {
                        Text("AirBridge")
                            .font(.largeTitle)
                            .bold()

                        Text("Select a room on the left, or start a new one")
                            .foregroundStyle(.secondary)

                        if isLoading {
                            ProgressView("Working...")
                        }

                        if let errorMessage {
                            Text(errorMessage)
                                .foregroundStyle(.red)
                                .font(.callout)
                                .multilineTextAlignment(.center)
                        }
                    }
                }
            }
            .padding()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button("Create Room") {
                        pendingAction = .create
                        passcodeInput = ""
                        showPasscodePrompt = true
                    }
                    .disabled(isLoading)
                }
                ToolbarItem(placement: .primaryAction) {
                    Button("Join Room") {
                        pendingAction = .join
                        passcodeInput = ""
                        roomCodeInput = ""
                        showPasscodePrompt = true
                    }
                    .disabled(isLoading)
                }
            }
        }
        .alert(
            pendingAction == .create ? "Enter a passcode to create a room" : "Join a room",
            isPresented: $showPasscodePrompt
        ) {
            if pendingAction == .join {
                TextField("Room code", text: $roomCodeInput)
            }
            TextField("Passcode", text: $passcodeInput)
            Button("Cancel", role: .cancel) {
                pendingAction = nil
            }
            Button("Continue") {
                handlePasscodeSubmit()
            }
        }
    }

    private func loadRooms() {
        do {
            rooms = try getRooms()
        } catch {
            errorMessage = "Couldn't load rooms: \(error.localizedDescription)"
        }
    }

    private func handlePasscodeSubmit() {
        guard let action = pendingAction else { return }
        let passcode = passcodeInput

        errorMessage = nil
        isLoading = true

        switch action {
        case .create:
            Task {
                do {
                    let roomId = try await createRoomSafely(passcode: passcode)
                    await MainActor.run {
                        isLoading = false
                        waitingRoomId = roomId
                        waitingPasscode = passcode
                        selectedRoom = nil
                        loadRooms()
                    }
                } catch {
                    await MainActor.run {
                        isLoading = false
                        errorMessage = "Couldn't create room: \(error.localizedDescription)"
                    }
                }
            }
            
        case .join:
            Task {
                do {
                    let roomId = roomCodeInput
                    let _ = try await joinRoomSafely(roomId: roomId, passcode: passcode)
                    await MainActor.run {
                        isLoading = false
                        waitingRoomId = roomId
                        waitingPasscode = passcode
                        selectedRoom = nil
                        loadRooms()
                    }
                } catch {
                    await MainActor.run {
                        isLoading = false
                        print("Full error: \(error)")
                        print("Error type: \(type(of: error))")
                        errorMessage = "Couldn't join room: \(error)"
                    }
                }
            }
        }
    }
    
    private func joinRoomSafely(roomId: String, passcode: String) async throws -> Bool {
        try await Task.detached(priority: .userInitiated) {
            try await joinRoom(roomId: roomId, passcode: passcode)
        }.value
    }

    private func createRoomSafely(passcode: String) async throws -> String {
        try await Task.detached(priority: .userInitiated) {
            try await createRoom(passcode: passcode)
        }.value
    }
}

// MARK: - Room detail (when a room in the sidebar is selected)

struct RoomDetailContent: View {
    let room: Room

    private var progressFraction: Double {
        room.total > 0 ? Double(room.sent) / Double(room.total) : 0
    }

    var body: some View {
        VStack(spacing: 16) {
            Text(room.fileName.isEmpty ? "Room" : room.fileName)
                .font(.title2)
                .bold()

            if room.total > 0 {
                VStack(spacing: 6) {
                    ProgressView(value: progressFraction)
                    Text("\(formatBytes(room.sent)) / \(formatBytes(room.total))")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            VStack(alignment: .leading, spacing: 8) {
                LabeledContent("Room ID", value: room.roomId)
                LabeledContent("Peer IP", value: room.peerIp)
                LabeledContent("Peer Port", value: String(room.peerPort))
                LabeledContent("Fingerprint", value: room.certFingerprint)
                if !room.fileHash.isEmpty {
                    LabeledContent("File hash", value: room.fileHash)
                }
                LabeledContent("Created", value: room.createdAt.formatted(date: .abbreviated, time: .shortened))
            }
            .padding()
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
            .textSelection(.enabled)
        }
        .frame(maxWidth: 400)
    }

    private func formatBytes(_ value: UInt32) -> String {
        ByteCountFormatter.string(fromByteCount: Int64(value), countStyle: .file)
    }
}

// MARK: - Waiting for friend (shown right after creating a room)

struct WaitingForFriendContent: View {
    let roomId: String
    let passcode: String

    @State private var didCopy: Bool = false

    var body: some View {
        VStack(spacing: 20) {
            ProgressView()
                .scaleEffect(1.5)

            Text("Waiting for friend...")
                .font(.title2)
                .bold()

            VStack(spacing: 10) {
                VStack(spacing: 4) {
                    Text("Room code")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(roomId)
                        .font(.system(.title3, design: .monospaced))
                        .textSelection(.enabled)
                }

                VStack(spacing: 4) {
                    Text("Passcode")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(passcode)
                        .font(.system(.title3, design: .monospaced))
                        .textSelection(.enabled)
                }
            }
            .padding()
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))

            Button {
                copyDetailsToClipboard()
            } label: {
                Label(didCopy ? "Copied!" : "Copy invite", systemImage: didCopy ? "checkmark" : "doc.on.doc")
            }
            .buttonStyle(.bordered)
        }
    }

    private func copyDetailsToClipboard() {
        let text = "room: \(roomId) passcode: \(passcode)"
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)

        didCopy = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            didCopy = false
        }
    }
}

#Preview {
    ContentView()
}
