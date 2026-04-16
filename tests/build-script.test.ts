import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

describe('src-tauri build script', () => {
  it('reruns when the resident dedicated renderer source changes', () => {
    const buildScript = readFileSync(resolve('src-tauri/build.rs'), 'utf8')

    expect(buildScript).toContain(
      'cargo:rerun-if-changed=../sidecar/dedicated-renderer/main.rs',
    )
  })

  it('avoids prebuilding the dedicated renderer in tauri command hooks', () => {
    const tauriConfig = JSON.parse(
      readFileSync(resolve('src-tauri/tauri.conf.json'), 'utf8'),
    ) as {
      build?: {
        beforeBuildCommand?: string
        beforeDevCommand?: string
      }
    }

    expect(tauriConfig.build?.beforeBuildCommand).not.toContain(
      'prepare:dedicated-renderer',
    )
    expect(tauriConfig.build?.beforeDevCommand).not.toContain(
      'prepare:dedicated-renderer',
    )
  })
})
