//
//  ViewController.swift
//  storyboardtest
//
//  Created by Matthew Pauly on 1/23/23.
//

import UIKit
import WebKit

import Sensing

class ViewController: UIViewController {

    override func viewDidLoad() {
        super.viewDidLoad()
        // Do any additional setup after loading the view.
        
        view.addSubview(webView)
        NSLayoutConstraint.activate([
            webView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            webView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            webView.bottomAnchor.constraint(equalTo: view.layoutMarginsGuide.bottomAnchor),
            webView.topAnchor.constraint(equalTo: view.layoutMarginsGuide.topAnchor)
        ])
        
        let contentController = self.webView.configuration.userContentController
        contentController.add(self, name: "pokeMessageHandler")
        
        let port = server_port;  // get server port from sensing module
        let url = URL(string: "http://127.0.0.1:\(port)")
        let req = URLRequest(url: url!)
        webView.load(req)
    }
    
    private lazy var webView: WKWebView = {
        let webView = WKWebView()
        webView.translatesAutoresizingMaskIntoConstraints = false
        webView.scrollView.bounces = false
        return webView
    }()
    
    //added for full screen
    override func viewWillLayoutSubviews() {
        super.viewWillLayoutSubviews()

        additionalSafeAreaInsets.bottom -= view.safeAreaInsets.bottom
        additionalSafeAreaInsets.top -= view.safeAreaInsets.top
    }
}

extension ViewController: WKScriptMessageHandler{
    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        // receive message from webapp
        guard let dict = message.body as? [String : AnyObject] else {
            return
        }
        //print(dict)  // show the dictionary containing the message we received
        
        // send the message we received back to the webapp by changing the page's text
        guard let message = dict["message"] else {
            return
        }
        
        // pass the poke to the sensing module
        update_sensor_config()
        
        return
        
        // if we want bidirectional communication we can use this
        /*
        let script = "document.getElementById('value').innerText = \"\(message)\""

        webView.evaluateJavaScript(script) { (result, error) in
            if let result = result {
                print("Label is updated with message: \(result)")
            } else if let error = error {
                print("An error occurred: \(error)")
            }
        }
        */
    }
}
