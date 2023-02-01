import Foundation
import SwiftUI
import RustLib

struct AnalyzeView: View {
    
    @State private var showWebView = false
  
    var body: some View {
        VStack {
            Button(action: genWeekView) {
              Text("Generate Past Week Visualization")
            }
            .padding(.bottom, 20)
            Button {
                showWebView.toggle()
            } label: {
                Text("Show Visualization")
            }
            .sheet(isPresented: $showWebView) {
                WebView(url: getDocumentsDirectory().appendingPathComponent("week.html"))
            }
        }
    }
    
    func genWeekView() {
        GenPastWeekViz()
    }
}

struct AnalyzeView_Previews: PreviewProvider {
    static var previews: some View {
        AnalyzeView()
    }
}
