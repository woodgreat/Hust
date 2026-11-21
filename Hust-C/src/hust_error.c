#include "hust_error.h"
#include <stdio.h>

/* Global error info - for backtrace */
const char* g_hust_error_msg = NULL;
const char* g_hust_error_file = NULL;
int g_hust_error_line = 0;

/* Get error string */
const char* hust_error_string(HustError err) {
    switch (err) {
        case HUST_OK: return "Success";
        case HUST_ERR_DIVIDE_BY_ZERO: return "Division by zero";
        case HUST_ERR_NULL_POINTER: return "Null pointer";
        case HUST_ERR_OUT_OF_RANGE: return "Out of range";
        case HUST_ERR_INVALID_ARG: return "Invalid argument";
        case HUST_ERR_MEMORY_ALLOC: return "Memory allocation failed";
        case HUST_ERR_FILE_NOT_FOUND: return "File not found";
        case HUST_ERR_IO: return "I/O error";
        case HUST_ERR_UNKNOWN: return "Unknown error";
        default: return "Unrecognized error code";
    }
}

/* Print error with location */
void hust_error_print(HustError err) {
    if (err == HUST_OK) {
        printf("Success\n");
        return;
    }
    
    fprintf(stderr, "Error [%d]: %s\n", err, hust_error_string(err));
    
    if (g_hust_error_msg) {
        fprintf(stderr, "  Message: %s\n", g_hust_error_msg);
    }
    if (g_hust_error_file) {
        fprintf(stderr, "  Location: %s:%d\n", g_hust_error_file, g_hust_error_line);
    }
}
