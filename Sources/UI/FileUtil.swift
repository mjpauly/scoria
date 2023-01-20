import SwiftUI


// SHARE SHEET
func shareFile(file: URL) {
    var filesToShare = [Any]()  // Create the Array which includes the files to share
    filesToShare.append(file)
    // Make the activityViewContoller which shows the share-view
    let activityViewController = UIActivityViewController(activityItems: filesToShare, applicationActivities: nil)
    
    // Show the share-view
    // Get the first window from the connectedScenes object
    if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene {
        // Get the root view controller of the first window
        let rootViewController = windowScene.windows.first?.rootViewController
        
        // Present the share sheet to the user
        rootViewController?.present(activityViewController, animated: true, completion: nil)
    }
}

func getDocumentsDirectory() -> URL {
    // find all possible documents directories for this user
    let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
    
    // just send back the first one, which ought to be the only one
    return paths[0]
}

// appendToFile tries to append data to a file, and if the file doesn't exist it creates it
func appendToFile(file: String, dataString: String) {
    /* try to append to file if file exists, otherwise create new file with data */
    let data = dataString.data(using: .utf8)!  // Encode String as bytes for file writing
    let fileManager = FileManager.default
    if fileManager.fileExists(atPath: file) {  // Check if the file exists
        let fileHandle = FileHandle(forWritingAtPath: file)  // Open the file for writing
        fileHandle?.seekToEndOfFile()  // Move to the end of the file
        fileHandle?.write(data)  // Append the data to the file
        fileHandle?.closeFile()  // Close the file
    } else {
        writeToNewFile(file: file, dataString: dataString)
    }
}

// writeToNewFile writes data to a file, creating it if it doesn't exist, or overwriting it if it does
func writeToNewFile(file: String, dataString: String) {
    /* Write the data to a new file, overwriting what was there before */
    do {
        try dataString.write(toFile: file, atomically: true, encoding: .utf8)
    } catch {
        print("Error writing to file: \(error)")
    }
}
