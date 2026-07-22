//
//  airbridgeApp.swift
//  airbridge
//
//  Created by Mano W on 27/06/26.
//

import SwiftUI

@main
struct airbridgeApp: App {
    init() {
        let logPath = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("airbridge_\(AppConfig.port).log")
            .path

        do {
            try configureLogging(logPath: logPath)
            try configurePort(port: AppConfig.port)
            print("Logging to: \(logPath)")
        } catch {
            print("Startup configuration failed: \(error)")
        }
    }
    
    var body: some Scene {
        WindowGroup {
            ContentView()
            
        }
    }
}
