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
})
