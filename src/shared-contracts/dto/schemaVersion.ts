import { z } from 'zod'

export const schemaVersions = {
  contract: 'boothy.camera.contract.v1',
  errorEnvelope: 'boothy.camera.error-envelope.v1',
  manifest: 1,
  protocol: 'boothy.camera.protocol.v1',
} as const

export const contractSchemaVersionSchema = z.literal(schemaVersions.contract)
export const errorEnvelopeSchemaVersionSchema = z.literal(schemaVersions.errorEnvelope)
export const manifestSchemaVersionSchema = z.literal(schemaVersions.manifest)
export const protocolSchemaVersionSchema = z.literal(schemaVersions.protocol)
