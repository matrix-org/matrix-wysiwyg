//
// Copyright 2022 The Matrix.org Foundation C.I.C
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

import Foundation

enum BuildHtmlAttributedError: LocalizedError {
    case dataError(encoding: String.Encoding)

    var errorDescription: String? {
        switch self {
        case .dataError(encoding: let encoding):
            return "Unable to encode string with \(encoding)"
        }
    }
}

extension NSAttributedString {
    convenience init(html: String, encoding: String.Encoding = .utf16) throws {
        guard let data = html.data(using: encoding, allowLossyConversion: false) else {
            throw BuildHtmlAttributedError.dataError(encoding: encoding)
        }
        try self.init(data: data,
                      options: [.documentType: NSAttributedString.DocumentType.html],
                      documentAttributes: nil)
    }
}
