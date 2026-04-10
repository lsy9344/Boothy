declare module '../../scripts/prepare-windows-signing.mjs' {
  export interface SigningInputsResult {
    certificateBase64: string | null
    certificatePassword: string | null
    mode: 'unsigned-draft' | 'signing-inputs-present'
    source: 'path' | 'base64' | null
    timestampUrl: string | null
  }

  export interface ResolveSigningInputsOptions {
    allowUnsigned?: boolean
    readFileSync?: (path: string) => string | Uint8Array
  }

  export function resolveSigningInputs(
    env?: Record<string, string | undefined>,
    options?: ResolveSigningInputsOptions,
  ): SigningInputsResult

  export function main(
    argv?: string[],
    env?: Record<string, string | undefined>,
  ): Promise<SigningInputsResult>
}
