import { z } from 'zod';

/**
 * Schema for auth tokens with validation
 */
export const TokenSchema = z.object({
  accessToken: z.string(),
  refreshToken: z.string().optional(),
  expiresAt: z.number(),
  scopes: z.array(z.string()).default([]),
  tokenType: z.string().default('bearer'),
});

export type Token = z.infer<typeof TokenSchema>;

/**
 * Schema for token store
 */
export const TokenStoreSchema = z.record(z.string(), TokenSchema);