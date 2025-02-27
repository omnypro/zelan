import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ZelanError } from '../types';

interface UseTauriCommandOptions {
  onError?: (error: ZelanError | string) => void;
}

/**
 * A hook for safely invoking Tauri commands with error handling
 */
export const useTauriCommand = (options?: UseTauriCommandOptions) => {
  // Execute a Tauri command safely
  const safeInvoke = useCallback(async <T,>(
    command: string,
    args?: Record<string, unknown>
  ): Promise<T> => {
    try {
      return await invoke<T>(command, args);
    } catch (error) {
      // Handle error and pass to optional error handler
      if (options?.onError) {
        if (typeof error === 'object' && error !== null) {
          options.onError(error as ZelanError);
        } else {
          options.onError(String(error));
        }
      }
      throw error;
    }
  }, [options]);

  return { invoke: safeInvoke };
};