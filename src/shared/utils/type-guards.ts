/**
 * Type guard utilities for runtime validation
 */

/**
 * Checks if a value is a non-null object
 */
export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

/**
 * Checks if a value is a string
 */
export function isString(value: unknown): value is string {
  return typeof value === 'string'
}

/**
 * Checks if a value is a number
 */
export function isNumber(value: unknown): value is number {
  return typeof value === 'number' && !Number.isNaN(value)
}

/**
 * Checks if a value is a boolean
 */
export function isBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean'
}

/**
 * Checks if a value is an array
 */
export function isArray(value: unknown): value is unknown[] {
  return Array.isArray(value)
}

/**
 * Checks if a value is an array of strings
 */
export function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
}

/**
 * Type guard for checking if an object has a specific property
 */
export function hasProperty<K extends string>(obj: unknown, prop: K): obj is { [P in K]: unknown } {
  return isObject(obj) && prop in obj
}

/**
 * Type guard for checking if an object has a specific property of a specific type
 */
export function hasPropertyOfType<K extends string, T>(
  obj: unknown,
  prop: K,
  check: (val: unknown) => val is T
): obj is { [P in K]: T } {
  return hasProperty(obj, prop) && check(obj[prop])
}

/**
 * Creates an object validator that checks for required properties with specific types
 */
export function createObjectValidator<T>(validators: {
  [K in keyof T]?: (val: unknown) => boolean
}): (obj: unknown) => obj is T {
  return (obj: unknown): obj is T => {
    if (!isObject(obj)) return false

    // Check each required property
    for (const key in validators) {
      if (Object.prototype.hasOwnProperty.call(validators, key)) {
        const validator = validators[key]

        // Skip if property doesn't have a validator
        if (!validator) continue

        // Check if property exists in object
        if (!(key in obj)) return false

        // Get the property value and validate it
        const value = obj[key as keyof typeof obj]
        if (!validator(value)) return false
      }
    }

    return true
  }
}
