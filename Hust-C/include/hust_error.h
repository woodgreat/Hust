#ifndef HUST_ERROR_H
#define HUST_ERROR_H

#include <stdint.h>
#include <stdbool.h>

/* Error codes - POSIX style */
typedef enum {
    HUST_OK = 0,
    HUST_ERR_DIVIDE_BY_ZERO = -1,
    HUST_ERR_NULL_POINTER = -2,
    HUST_ERR_OUT_OF_RANGE = -3,
    HUST_ERR_INVALID_ARG = -4,
    HUST_ERR_MEMORY_ALLOC = -5,
    HUST_ERR_FILE_NOT_FOUND = -6,
    HUST_ERR_IO = -7,
    HUST_ERR_UNKNOWN = -99
} HustError;

/* Global error info - for backtrace */
extern const char* g_hust_error_msg;
extern const char* g_hust_error_file;
extern int g_hust_error_line;

/* Set error with location info */
#define HUST_SET_ERROR(code, msg) \
    g_hust_error_msg = msg; \
    g_hust_error_file = __FILE__; \
    g_hust_error_line = __LINE__; \
    return code;

/* Clear error */
#define HUST_CLEAR_ERROR() \
    g_hust_error_msg = NULL; \
    g_hust_error_file = NULL; \
    g_hust_error_line = 0;

/* Check and return error (similar to Rust ? operator) */
#define HUST_TRY(result) \
    if ((result) != HUST_OK) { \
        return result; \
    }

/* Get error string */
const char* hust_error_string(HustError err);

/* Print error with location */
void hust_error_print(HustError err);

#endif /* HUST_ERROR_H */
