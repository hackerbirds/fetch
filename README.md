# Fetch 

Fetch is an app launcher for macOS. It is designed to be extremely quick to use and be as little in the way as possible: it has responsive keyboard navigation, and no perceptible latency when loading or searching an app. Switch between apps without thinking about it.

It is still a work in progress, though most functionality is implemented. We aim to support Windows and Linux in the future, and you can view our progress in the [roadmap](https://github.com/users/hackerbirds/projects/3). The current plan is to reach a stable 1.0 release, after which Fetch will be considered complete and will not receive any new features (besides bug fixes).

## Features

- **Fast**: Fetch launches instantly. Searches happen instantly.
- **Minimalistic**: it does one thing and does it really well.
- **Deterministic**: no unexpected behavior. The search algorithm learns from your past usage. Gone are the days of Spotlight failing on you.
- **Optimized for keyboard navigation**: You don't need to use your trackpad to switch between apps.

## Easy as 1, 2, 3

1. Press `Option+Space` (`⌥+Space`) anywhere to launch the app.
2. Search the app you want to open. Use `Tab` to navigate forward through the results, and `Shift+Tab` to go backwards.
3. Press `Enter` on the app you selected, and it'll open.

![Screenshot of app](app-screenshot.png)

### Configuring the app

While the search bar is active, press `Cmd+T` (`⌘+T`) to open the configuration file. The app requires a restart to update its configuration.

## Installation

There are two ways you can install Fetch:

1. [Download the pre-packaged release](https://github.com/hackerbirds/fetch/releases)
  - NOTE: Fetch is not notarized (I refuse to pay for an Apple developer certificate), so macOS will pretend it's "damaged" when opening the pre-packaged app. To fix this, you need to disable notarization quarantine from the terminal by running `xattr -d com.apple.quarantine /path/to/Fetch.app`. 

2. You can build the app yourself using `cargo bundle`. Run the following commands in your terminal:

```bash
git clone https://github.com/hackerbirds/fetch.git
cd fetch
cargo install cargo-bundle
chmod +x bundle.sh
./bundle.sh
```