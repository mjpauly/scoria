//
//  SceneDelegate.swift
//  storyboardtest
//
//  Created by Matthew Pauly on 1/23/23.
//

import UIKit
import Sensing

class SceneDelegate: UIResponder, UIWindowSceneDelegate {

    var window: UIWindow?


    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options connectionOptions: UIScene.ConnectionOptions) {
        // Use this method to optionally configure and attach the UIWindow `window` to the provided UIWindowScene `scene`.
        // If using a storyboard, the `window` property will automatically be initialized and attached to the scene.
        // This delegate does not imply the connecting scene or session are new (see `application:configurationForConnectingSceneSession` instead).
        print_and_log(s: "SceneDelegate: scene")
        guard let windowScene = (scene as? UIWindowScene) else { return }

        // Storyboard-free launch
        // https://ericgustin.medium.com/swift-5-how-to-set-up-your-initial-viewcontroller-without-a-storyboard-in-xcode-cd5615182c9d
        window = UIWindow(frame: windowScene.coordinateSpace.bounds)
        window?.windowScene = windowScene
        window?.makeKeyAndVisible()
        let vc = ViewController()
        window?.rootViewController = vc
    }

    func sceneDidDisconnect(_ scene: UIScene) {
        // Called as the scene is being released by the system.
        // This occurs shortly after the scene enters the background, or when its session is discarded.
        // Release any resources associated with this scene that can be re-created the next time the scene connects.
        // The scene may re-connect later, as its session was not necessarily discarded (see `application:didDiscardSceneSessions` instead).
        print_and_log(s: "SceneDelegate: sceneDidDisconnect")
    }

    func sceneDidBecomeActive(_ scene: UIScene) {
        // Called when the scene has moved from an inactive state to an active state.
        // Use this method to restart any tasks that were paused (or not yet started) when the scene was inactive.
        print_and_log(s: "SceneDelegate: sceneDidBecomeActive")
    }

    func sceneWillResignActive(_ scene: UIScene) {
        // Called when the scene will move from an active state to an inactive state.
        // This may occur due to temporary interruptions (ex. an incoming phone call).
        print_and_log(s: "SceneDelegate: sceneWillResignActive")
    }

    func sceneWillEnterForeground(_ scene: UIScene) {
        // Called as the scene transitions from the background to the foreground.
        // Use this method to undo the changes made on entering the background.
        print_and_log(s: "SceneDelegate: sceneWillEnterForeground")
        handle_foreground() // restart UI server
        // reload the UI with the new key and port
        guard let windowScene = (scene as? UIWindowScene) else { return }
        guard let vc = (windowScene.keyWindow?.rootViewController as? ViewController) else { return }
        vc.reload()
    }

    func sceneDidEnterBackground(_ scene: UIScene) {
        // Called as the scene transitions from the foreground to the background.
        // Use this method to save data, release shared resources, and store enough scene-specific state information
        // to restore the scene back to its current state.
        print_and_log(s: "SceneDelegate: sceneDidEnterBackground")
        handle_background()
    }


}

