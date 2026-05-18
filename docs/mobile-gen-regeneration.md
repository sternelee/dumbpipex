# Mobile generated-platform tweaks

This repo now carries **intentional manual edits** inside `src-tauri/gen/` to improve the default iOS/Android mobile shell experience.

Because `src-tauri/gen/android` and `src-tauri/gen/apple` are generated outputs, these edits may be overwritten the next time the platforms are regenerated.

## Files with manual mobile UX tweaks

### Android

- `src-tauri/gen/android/app/src/main/AndroidManifest.xml`
  - `android:windowSoftInputMode="adjustResize|stateHidden"`
  - Reason: lets the WebView resize more gracefully when the soft keyboard appears.
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
4. Launch iOS app and verify there is no bright launch flash.
5. Confirm dark shell colors match the WebView's first-painted frame.

## Why these are kept in docs

The current Tauri setup generates platform projects into `src-tauri/gen/`, so a small written contract is safer than relying on memory. If mobile shell customization grows further, the next step should be to move these tweaks into a more permanent generation/customization workflow.
