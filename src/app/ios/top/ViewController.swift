//
//  ViewController.swift
//  storyboardtest
//
//  Created by Matthew Pauly on 1/23/23.
//

import UIKit
import WebKit

import Sensing

// The Sensing framework  is a dependency, and it defines the protocol both frameworks need to
// agree on for the class's type when passed to the Sensing framwork.
class ViewController: UIViewController, MyViewControllerProtocol, WKNavigationDelegate {

    override func viewDidLoad() {
        super.viewDidLoad()
        // Do any additional setup after loading the view.
        
        webView.navigationDelegate = self
        
        view.addSubview(webView)
        NSLayoutConstraint.activate([
            webView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            webView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            webView.bottomAnchor.constraint(equalTo: view.layoutMarginsGuide.bottomAnchor),
            webView.topAnchor.constraint(equalTo: view.layoutMarginsGuide.topAnchor)
        ])
        
        let contentController = self.webView.configuration.userContentController
        contentController.add(self, name: "pokeMessageHandler")
    }
    
    func reload() {
        // load the frontend webapp
        let port = server_port  // get server port from sensing module
        let scope = server_scope
        let urlstr = "http://127.0.0.1:\(port)/\(scope)/"
        // print_and_log(s: "Reloading UI with url \(urlstr)")
        let url = URL(string: urlstr)
        let req = URLRequest(url: url!)
        webView.load(req)
    }
    
    private lazy var webView: WKWebView = {
        let webConfiguration = WKWebViewConfiguration()
        // innocuous user agent addition to thwart malicious API use
        webConfiguration.applicationNameForUserAgent = "WebDriver/A118.35 (iPhone)"
        let webView = WKWebView(frame: .zero, configuration: webConfiguration)
        webView.translatesAutoresizingMaskIntoConstraints = false
        webView.scrollView.bounces = false
        webView.isOpaque = false
        webView.backgroundColor = UIColor.clear
        return webView
    }()
    
    //added for full screen
    override func viewWillLayoutSubviews() {
        super.viewWillLayoutSubviews()

        additionalSafeAreaInsets.bottom -= view.safeAreaInsets.bottom
        additionalSafeAreaInsets.top -= view.safeAreaInsets.top
        additionalSafeAreaInsets.left -= view.safeAreaInsets.left
        additionalSafeAreaInsets.right -= view.safeAreaInsets.right
    }

    // Determine which navigation actions should result in opening in the browser
    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
    ) {
        if navigationAction.navigationType == .linkActivated  {
            if let url = navigationAction.request.url,
               let host = url.host, !host.hasPrefix("127.0.0.1"),
               UIApplication.shared.canOpenURL(url) {
                // was a link to a url that's not on localhost -> open in browser
                UIApplication.shared.open(url)
                //print("Redirected to browser.")
                decisionHandler(.cancel)
                return
            } else {
                //print("Open it locally")
                decisionHandler(.allow)
                return
            }
        } else {
            //print("not a user click")
            decisionHandler(.allow)
            return
        }
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
        //guard let message = dict["message"] else {
        if dict["message"] == nil {
            return
        }
        
        // pass the poke to the sensing module
        handle_poke(viewController: self)
        
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

extension ViewController: UIDocumentPickerDelegate {
    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        guard let fileURL = urls.first else { return }
        
        // Process the imported file
        handle_import(fileURL: fileURL)
    }
    
    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        // Handle cancellation of the document picker
    }
}
