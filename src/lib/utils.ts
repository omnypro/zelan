import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

/**
 * Combines class names with tailwind-merge for conflicts
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Determines if code is running in the renderer process
 */
export function isRenderer(): boolean {
  // Running in a web browser
  if (typeof process === 'undefined') return true;
  
  // Check for node integration
  if (process?.type === 'renderer') return true;
  
  // Check for Electron specific variables
  if (typeof window !== 'undefined' && 
      typeof window.process === 'object' && 
      window.process.type === 'renderer') {
    return true;
  }
  
  return false;
}
