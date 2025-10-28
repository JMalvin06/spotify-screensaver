# Spotify Screensaver
This is a simple little screensaver that displays a user's currently playing Spotify track as a bouncing square, reminiscent of the iconic DVD screensaver. Since it currently only utilizes the Spotify API to fetch current track, this screensaver will only work while you are connected to the internet. However, this allows compatibility across devices playing your music.

![demo](https://raw.githubusercontent.com/JMalvin06/spotify-screensaver/main/preview/demo.gif)

## Prerequisites
- Windows version requires Rust - [Install Rust](https://rust-lang.org/tools/install/)
- MacOS version requires the xcode command line tools - `xcode-select --install`

## Installation
This is built to run on your own Spotify application since there is not a very safe way to get it to run on 
a single central app. Luckily, Spotify makes it quite easy to create your own application from their website.

### Application setup
1. Go to the [Spotify for Developers](https://developer.spotify.com/) page
2. Log in, navigate to the dashboard tab, and select "Create app"
3. Under "Redirect URIs", enter http://127.0.0.1:8000/callback
4. Check the Web API option, and create the app

### Screensaver installation
1. Download the zip file from releases for your platform
2. Open the installer and input the client ID and secret of the app you just created
3. Log in with your Spotify account, and click "Agree"
4. Find the location of the screensaver folder, and set that as the build directory
    -  MacOS: `SpotifyScreensaver`
    -  Windows: `spotify_screensaver`
5. Select an output location for your build screensaver
6. Install your newly created screensaver
    - MacOS: Open the .saver file to install, set it as your default screensaver in settings
    - Windows: Right-click the .scr file and click install

