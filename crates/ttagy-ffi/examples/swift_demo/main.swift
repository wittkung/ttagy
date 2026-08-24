import Foundation
import ttagy

@main
struct SwiftDemo {
    static func main() async {
        print("⚡ Connecting to TTAgy via Swift Clang Importer...")

        guard let client = ttagy_client_create() else {
            print("Failed to initialize client")
            return
        }
        defer { ttagy_client_free(client) }

        var resp: UnsafeMutablePointer<ttagy_response_t>? = nil
        let status = ttagy_client_chat(client, "Hello from modern Swift 6 concurrency!", &resp)

        if status == 0, let r = resp {
            let content = String(cString: r.pointee.content)
            print("✅ Response: \(content) (Elapsed: \(r.pointee.elapsed_ms)ms)")
            ttagy_response_free(r)
        }
    }
}
