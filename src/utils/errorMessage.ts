type ErrorRecord = Record<string, unknown>;

const nonEmptyString = (value: unknown): string | null =>
  typeof value === 'string' && value.trim() ? value.trim() : null;

export const getErrorDetail = (error: unknown): string | null => {
  if (error instanceof Error) {
    return nonEmptyString(error.message);
  }

  const directMessage = nonEmptyString(error);
  if (directMessage) {
    return directMessage;
  }

  if (!error || typeof error !== 'object') {
    return null;
  }

  const errorRecord = error as ErrorRecord;
  const message = nonEmptyString(errorRecord.message) ?? nonEmptyString(errorRecord.error);
  if (!message) {
    return null;
  }

  const errorCode = nonEmptyString(errorRecord.error_code);
  return errorCode ? `[${errorCode}] ${message}` : message;
};

export const formatErrorMessage = (error: unknown, operationMessage: string): string => {
  const detail = getErrorDetail(error);
  if (!detail || detail === operationMessage) {
    return operationMessage;
  }
  return `${operationMessage}: ${detail}`;
};
