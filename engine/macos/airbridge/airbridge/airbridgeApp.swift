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
            do {
                try configurePort(port: AppConfig.port)
            } catch {
                print("Failed to configure engine port: \(error)")
            }
        }
    
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
