// <app> — the error code contract.
//
// The zod union both ends of the message protocol validate with. It is
// derived from the shared list, so a code can never be sent that the
// receiver does not know.

import { z } from "zod";
import { ERROR_CODES } from "../shared-kernel/errors";

export const ErrorCodeSchema = z.enum(ERROR_CODES);

export type ErrorCodeValue = z.infer<typeof ErrorCodeSchema>;
