# Mobile generated-platform tweaks

This repo now carries **intentional manual edits** inside `src-tauri/gen/` to improve the default iOS/Android mobile shell experience.

Because `src-tauri/gen/android` and `src-tauri/gen/apple` are generated outputs, these edits may be overwritten the next time the platforms are regenerated.

## Files with manual mobile UX tweaks

### Android

- `src-tauri/gen/android/app/src/main/AndroidManifest.xml`
  - `android:windowSoftInputMode="adjustResize|stateHidden"`
  - Network permissions required by the mobile iroh client:
    `INTERNET`, network/Wi-Fi state, Wi-Fi multicast, and Android 13+
    `NEARBY_WIFI_DEVICES`.
  - Reason: lets the WebView resize more gracefully when the soft keyboard appears.
- `src-tauri/gen/android/app/src/main/java/com/sternelee/dumbpipex/MainActivity.kt`
  - Requests `NEARBY_WIFI_DEVICES` on Android 13+.
  - Reason: target SDK 36 treats nearby Wi-Fi access as a runtime permission,
    and the P2P client may need Wi-Fi discovery/network details while connecting.
- `src-tauri/gen/android/app/src/main/res/values/colors.xml`
  - App surface colors aligned to the app's dark terminal-first palette.
- `src-tauri/gen/android/app/src/main/res/values/themes.xml`
- `src-tauri/gen/android/app/src/main/res/values-night/themes.xml`
  - Transparent status/navigation bars
  - Dark window background
  - Dark system-bar icon expectations
  - Reason: better edge-to-edge presentation and less jarring shell chrome.

### iOS

- `src-tauri/gen/apple/LaunchScreen.storyboard`
  - Launch background changed from the default system background to the app's dark surface.
  - Reason: avoids white flash during startup before the WebView paints.

## After regenerating mobile projects

Re-check and, if necessary, re-apply the changes above.

Recommended verification pass after each regen:

1. Open Android app and focus a text field.
2. Confirm the keyboard does **not** cover the input / main terminal area.
3. Confirm status bar and nav bar still visually blend with the app.
4. On Android 13+, confirm the app requests nearby Wi-Fi access on first launch.
5. Connect to a running CLI agent from the Android app.
6. Launch iOS app and verify there is no bright launch flash.
7. Confirm dark shell colors match the WebView's first-painted frame.

## Why these are kept in docs

The current Tauri setup generates platform projects into `src-tauri/gen/`, so a small written contract is safer than relying on memory. If mobile shell customization grows further, the next step should be to move these tweaks into a more permanent generation/customization workflow.
