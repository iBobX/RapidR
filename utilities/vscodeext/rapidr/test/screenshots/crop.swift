import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

// crop <in.png> <out.png> <top pixels> <width>: cuts `top` pixels off the
// top (the window's title bar) and scales to `width` pixels wide.
let args = CommandLine.arguments
guard args.count == 5, let top = Int(args[3]), let width = Int(args[4]),
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[1]) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil),
      let cropped = image.cropping(to: CGRect(x: 0, y: top, width: image.width, height: image.height - top))
else {
    FileHandle.standardError.write("usage: crop in.png out.png top width\n".data(using: .utf8)!)
    exit(2)
}
let height = Int((Double(cropped.height) * Double(width) / Double(cropped.width)).rounded())
guard let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                              space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
else { exit(1) }
context.interpolationQuality = .high
context.draw(cropped, in: CGRect(x: 0, y: 0, width: width, height: height))
guard let scaled = context.makeImage(),
      let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: args[2]) as CFURL, UTType.png.identifier as CFString, 1, nil)
else { exit(1) }
CGImageDestinationAddImage(dest, scaled, nil)
exit(CGImageDestinationFinalize(dest) ? 0 : 1)
