# Microsoft Edge WebView2 Runtime — Licensing Evidence

## What Boothy ships

`bundle.windows.webviewInstallMode` is set to `offlineInstaller`, so the Tauri NSIS bundler embeds
the Microsoft-provided WebView2 offline installer inside `Boothy_<version>_x64-setup.exe` and runs it
silently during installation. A booth PC therefore gets the WebView2 runtime without any internet
access.

The runtime is not extracted into `<install root>` as a Boothy file; it is installed by Microsoft's
own installer. That is why the `webview2-runtime` inventory entry is recorded as embedded in the
installer rather than as a staged tree with a digest of its own.

## License

The WebView2 Runtime is redistributable under the Microsoft Edge WebView2 Runtime distribution
terms, which permit bundling the runtime with an application and installing it as part of that
application's setup.

- Distribution terms: <https://developer.microsoft.com/microsoft-edge/webview2/>
- Redistribution mechanism: the Evergreen offline installer embedded by the Tauri bundler.

## Version cannot be pinned by the installer

The Tauri v2 configuration schema offers four WebView2 install modes — `skip`,
`downloadBootstrapper`, `embedBootstrapper`, and `offlineInstaller`. The v1-era `fixedRuntime` mode
does **not** exist in v2, so there is no supported way to pin an exact WebView2 build into the
installer.

The consequence is recorded rather than hidden: **every hardware validation run must record the
WebView2 runtime version it actually ran on.** `boothy.exe --self-check` reads the installed version
into the `webview2Version` field, `tests/hardware/installer/hv-18a/environment.md` records it, and
Story 7.8 needs it to compare present timing across runs.

## Attribution

Microsoft, Microsoft Edge, and WebView2 are trademarks of Microsoft Corporation. Boothy is not
affiliated with or endorsed by Microsoft Corporation.
