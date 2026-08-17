# Camera Helper .NET Dependencies — Licensing Evidence

The camera helper is published self-contained for `win-x64`, so the .NET runtime itself ships inside
the installer alongside the helper's package dependencies.

## Package dependencies

Generated with:

```
dotnet list sidecar\canon-helper\src\CanonHelper\CanonHelper.csproj package --include-transitive
```

Result on 2026-08-17:

| Package | Version | Kind | License | Source |
| --- | --- | --- | --- | --- |
| `System.Drawing.Common` | 8.0.0 | direct | MIT | <https://github.com/dotnet/winforms> |
| `Microsoft.Win32.SystemEvents` | 8.0.0 | transitive | MIT | <https://github.com/dotnet/runtime> |

There are no other transitive packages. The dependency surface is deliberately this small: every
package added here becomes something the offline installer has to carry and this document has to
account for.

## .NET runtime

`--self-contained true` copies the .NET 8 runtime assemblies and host into the publish tree.

| Component | Version | License | Source |
| --- | --- | --- | --- |
| .NET runtime (`Microsoft.NETCore.App`) | 8.0.x, matching the build machine's SDK | MIT | <https://github.com/dotnet/runtime> |

The MIT license permits redistribution with the copyright notice retained. `LICENSE.TXT` and
`THIRD-PARTY-NOTICES.TXT` are produced by `dotnet publish` into `release/dist/canon-helper/` and are
covered by the `camera-helper` staged tree digest, so removing them fails inventory verification.

## Refresh rule

Re-run the `dotnet list package` command above whenever `CanonHelper.csproj` changes and update this
table in the same change. The `camera-helper` inventory entry points at this file, so a stale table
is a stale licensing claim.
