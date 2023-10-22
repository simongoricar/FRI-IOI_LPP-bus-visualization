import { ResponseContentError } from "./errors.ts";

/**
 * Check whether the given input is an object (not including null or arrays).
 * *This might not cover edge cases - its intended use is for type-guarding JSON-decoded content.*
 *
 * Source: https://stackoverflow.com/questions/64951319/type-guard-to-make-sure-a-variable-is-an-object-in-typescript
 */
export function isObject(input: unknown): input is Record<string, unknown> {
    return typeof input === 'object'
      && input !== null
      && !Array.isArray(input);
}

/**
 * Returns the value from the payload at the given field,
 * throws an error otherwise.
 *
 * @param payload JSON-decoded payload to get the field from.
 * @param field_name Field to get from the payload.
 * @throws ResponseContentError The requested field is missing.
 */
export function getRequiredField(
  payload: Record<string, any>,
  field_name: string,
): any {
    if (!Object.hasOwn(payload, field_name)) {
        throw new ResponseContentError(`Missing required field: ${field_name}.`);
    }

    return payload[field_name];
}

export function getOptionalField(
  payload: Record<string, any>,
  field_name: string,
  fallback_value: any = null,
): any {
    if (!Object.hasOwn(payload, field_name)) {
        return fallback_value;
    }

    return payload[field_name];
}

