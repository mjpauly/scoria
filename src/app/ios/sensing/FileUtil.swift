import SwiftUI
import UniformTypeIdentifiers


// Ordinary share sheet that shares the file with its existing name
func shareFile(file: URL, viewController: UIViewController) {
    // Make the activityViewContoller which shows the share-view
    let activityViewController = UIActivityViewController(activityItems: [file], applicationActivities: nil)

    // Present the sharing activity view controller
    viewController.present(activityViewController, animated: true, completion: nil)
}

// Share sheet which renames the file (as a temporary file) and shares that.
// Falls back to the ordinary share sheet if this fails (like if there's no
// storage space on the device to create a duplicate of the log).
func shareFileWithDifferentName(originalURL: URL, desiredFilename: String, viewController: UIViewController) {
    // Create a temporary file URL with the desired filename
    let temporaryDirectory = FileManager.default.temporaryDirectory
    let temporaryURL = temporaryDirectory.appendingPathComponent(desiredFilename)
    
    do {
        // Copy the original file to the temporary location with the desired filename
        try FileManager.default.copyItem(at: originalURL, to: temporaryURL)
        
        // Create a sharing activity view controller
        let activityViewController = UIActivityViewController(activityItems: [temporaryURL], applicationActivities: nil)

        // Present the sharing activity view controller
        viewController.present(activityViewController, animated: true, completion: nil)

        // Remove the temporary file after sharing is complete
        activityViewController.completionWithItemsHandler = { _, _, _, _ in
            do {
                try FileManager.default.removeItem(at: temporaryURL)
            } catch {
                print("Error removing temporary file: \(error)")
            }
        }
    } catch {
        print_and_log(s: "Error copying export file: \(error)")
        print_and_log(s: "Falling back to sharing the unrenamed file.")
        shareFile(file: originalURL, viewController: viewController)
    }
}

func importFile(viewController: MyViewControllerProtocol) {
    let supportedTypes: [UTType] = [UTType.data]
    let documentPicker = UIDocumentPickerViewController(forOpeningContentTypes: supportedTypes)
    documentPicker.delegate = viewController
    viewController.present(documentPicker, animated: true, completion: nil)
}

func getDocumentsDirectory() -> URL {
    // find all possible documents directories for this user
    let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
    
    // just send back the first one, which ought to be the only one
    return paths[0]
}

func getLibraryDirectory() -> URL {
    let paths = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask)
    return paths[0]
}

func getTemporaryDirectoryPath() -> String {
    return NSTemporaryDirectory()
}

func getBundlePath() -> String {
    return Bundle.main.bundlePath
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
