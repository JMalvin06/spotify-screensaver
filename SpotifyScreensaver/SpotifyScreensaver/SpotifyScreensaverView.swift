//
//  SpotifyScreensaverView.swift
//  SpotifyScreensaver
//
//  Created by John Malvin on 9/8/25.
//

import ScreenSaver

class SpotifyScreensaverView: ScreenSaverView {
    private var squarePosition: CGPoint = .zero
    private let squareSize = NSSize(width: 250, height: 250)
    private var squareVelocity: CGVector = .zero
    private let constantVelocity = 5;
    private var cachedImage: NSImage?
    private var token: String = ""
    
    private var ID: String = ""
    private var SECRET: String = ""
    private var REFRESH: String = ""
    
    struct Player: Decodable{
        let item: Track
    }
    
    struct Track: Decodable {
        let name: String
        let artists: [Artist]
        let album: Album
    }
    
    struct Album: Decodable {
        let images: [Image]
        let name: String
    }
    
    struct Artist: Decodable {
        let name: String
    }
    
    struct Image: Decodable {
        let url: String
    }
    
    struct Access: Decodable {
        let access_token: String
    }
    
    struct User: Decodable {
        let name: String
        let refresh: String
    }
    
    
    override init?(frame: NSRect, isPreview: Bool) {
        super.init(frame: frame, isPreview: isPreview)
        squarePosition = CGPoint(x: frame.width / 2, y: frame.height / 2)
        squareVelocity = CGVector(dx: 2 , dy: 2)
        animationTimeInterval = 1.0/60
        let saverBundle = Bundle(for: type(of: self))
        
        if let url = saverBundle.url(forResource: "constants", withExtension: "json") {
            do{
                let data = try Data(contentsOf: url, options: .mappedIfSafe)
                let jsonResult = try JSONSerialization.jsonObject(with: data, options: .mutableLeaves)
                if let jsonResult = jsonResult as? Dictionary<String, AnyObject>,
                    let id = jsonResult["id"] as? String,
                    let secret = jsonResult["secret"] as? String {
                    self.ID = id
                    self.SECRET = secret
                }
            } catch {}
        }
        
        if let url = saverBundle.url(forResource: "user", withExtension: "json") {
            do{
                let data = try Data(contentsOf: url, options: .mappedIfSafe)
                let jsonResult = try JSONSerialization.jsonObject(with: data, options: .mutableLeaves)
                if let jsonResult = jsonResult as? Dictionary<String, AnyObject>, let refresh = jsonResult["refresh"] as? String {
                    self.REFRESH = refresh
                }
            } catch {}
        }
        
        Timer.scheduledTimer(withTimeInterval: 2, repeats: true) { _ in
            Task {
                await self.loadImage()
            }
        }
    }
    
    override func startAnimation() {
        super.startAnimation()
        let saverBundle = Bundle(for: type(of: self))
        if let url = saverBundle.url(forResource: "placeholder", withExtension: "png") {
            let image = NSImage(byReferencing: url)
            Task {
                @MainActor in self.cachedImage = image
            }
            Task{ await generateToken()}
        }
    }
    
    
    private func checkBounds() -> (xAxis: Bool, yAxis: Bool) {
        let xAxis = squarePosition.x - squareSize.width/2 <= 0 ||
        squarePosition.x + squareSize.width/2 >= bounds.width
        let yAxis = squarePosition.y - squareSize.height/2 <= 0 ||
        squarePosition.y + squareSize.height/2 >= bounds.height
        return (xAxis, yAxis)
    }
    
    @available(*, unavailable)
    required init?(coder decoder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
    
    
    override func draw(_ rect: NSRect) {
        drawSquare()
    }
    
    func setToken(token: String){
        self.token = token
    }
    
    func generateToken() async {
        let loginString = String(format: "%@:%@", self.ID, self.SECRET)
        let loginData = loginString.data(using: String.Encoding.utf8)!
        let base64LoginString = loginData.base64EncodedString()
        
        if let url = URL(string: "https://accounts.spotify.com/api/token"){
            var request = URLRequest(url: url)
            request.httpMethod = "POST"
            request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
            request.setValue("Basic \(base64LoginString)", forHTTPHeaderField: "Authorization")
            
            let body = "grant_type=refresh_token&refresh_token=\(self.REFRESH)"
            request.httpBody = body.data(using: .utf8)
            let task = URLSession.shared.dataTask(with: request) { data, response, error in
                if let error = error {
                    print("Error: ", error)
                    let errorString = String(describing: error)
                    Task{@MainActor in self.setToken(token: errorString)}
                    Task{@MainActor in await self.loadImage()}
                    return
                }
                
                guard let data = data else {
                        return
                }
                
                do {
                    let post = try JSONDecoder().decode(Access.self, from: data) // Since the JSON in the URL
                    Task{@MainActor in self.setToken(token: post.access_token)}
                } catch let jsonError {
                    print("Failed to decode json", jsonError)
                }

            }
            task.resume()
        }
    }


    
    func loadImage() async {
        let url = URL(string: "https://api.spotify.com/v1/me/player")!
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(self.token)", forHTTPHeaderField: "Authorization")
        
        let task = URLSession.shared.dataTask(with: request) { data, response, error in
            if let error = error {
                print("Error: ", error)
                return
            }
            
            guard let data = data else {
                return
            }
            
            
            do {
                let post = try JSONDecoder().decode(Player.self, from: data) // Since the JSON in the URL
                guard let url = URL(string: post.item.album.images[0].url) else {return}
                URLSession.shared.dataTask(with: url) { data, _, error in
                    if let data = data, let image = NSImage(data: data) {
                        Task {
                            @MainActor in self.cachedImage = image
                        }
                    }
                }.resume()
            } catch let jsonError {
                print("Failed to decode json", jsonError)
            }
        }
        task.resume()
    }
    
    private func drawSquare() {
        
        let squareDrawing = NSRect(x: squarePosition.x - squareSize.width / 2,
                                   y: squarePosition.y - squareSize.height / 2,
                                   width: squareSize.width,
                                   height: squareSize.height)
        
        let square = NSBezierPath(rect: squareDrawing)
    
        square.fill()
        
        if let image = cachedImage {
            image.draw(in: squareDrawing,
                       from: NSRect(origin: .zero, size: image.size),
                       operation: .sourceOver,
                       fraction: 1.0)
        }
    }
    
    override func animateOneFrame() {
        super.animateOneFrame()
        
        let outAxis = checkBounds()
        if outAxis.xAxis {
            squareVelocity.dx *= -1
        }
        
        if outAxis.yAxis {
            squareVelocity.dy *= -1
        }
        
        squarePosition.x += squareVelocity.dx
        squarePosition.y += squareVelocity.dy
        
        setNeedsDisplay(bounds)
    }
}
