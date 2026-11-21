/*
 * Hust-C Preprocessor Implementation
 * Cleans and prepares Hust source code for parsing
 */

#define _POSIX_C_SOURCE 200809L

#include "preprocessor.h"

/* Create a new preprocessed source */
PreprocessedSource* preprocessor_new(void) {
    PreprocessedSource* src = (PreprocessedSource*)malloc(sizeof(PreprocessedSource));
    if (!src) return NULL;
    
    src->lines = (char**)malloc(MAX_LINES * sizeof(char*));
    if (!src->lines) {
        free(src);
        return NULL;
    }
    
    src->line_count = 0;
    src->capacity = MAX_LINES;
    
    return src;
}

/* Free preprocessed source */
void preprocessor_free(PreprocessedSource* src) {
    if (!src) return;
    
    for (int i = 0; i < src->line_count; i++) {
        free(src->lines[i]);
    }
    free(src->lines);
    free(src);
}

/* Process a file and store cleaned lines */
int preprocessor_process_file(PreprocessedSource* src, const char* filename) {
    if (!src || !filename) return -1;
    
    FILE* file = fopen(filename, "r");
    if (!file) {
        fprintf(stderr, "Error: Cannot open file '%s'\n", filename);
        return -1;
    }
    
    char line[MAX_LINE_LENGTH];
    while (fgets(line, sizeof(line), file) && src->line_count < src->capacity) {
        /* Remove newline */
        line[strcspn(line, "\n")] = 0;
        
        /* Allocate and store line */
        src->lines[src->line_count] = strdup(line);
        if (!src->lines[src->line_count]) {
            fclose(file);
            return -1;
        }
        src->line_count++;
    }
    
    fclose(file);
    
    /* Clean the source */
    preprocessor_remove_comments(src);
    preprocessor_remove_empty_lines(src);
    preprocessor_trim_whitespace(src);
    
    return src->line_count;
}

/* Process a string and store cleaned lines */
int preprocessor_process_string(PreprocessedSource* src, const char* code) {
    if (!src || !code) return -1;
    
    /* Make a copy for tokenization */
    char* copy = strdup(code);
    if (!copy) return -1;
    
    /* Split by newlines */
    char* line = strtok(copy, "\n");
    while (line && src->line_count < src->capacity) {
        src->lines[src->line_count] = strdup(line);
        if (!src->lines[src->line_count]) {
            free(copy);
            return -1;
        }
        src->line_count++;
        line = strtok(NULL, "\n");
    }
    
    free(copy);
    
    /* Clean the source */
    preprocessor_remove_comments(src);
    preprocessor_remove_empty_lines(src);
    preprocessor_trim_whitespace(src);
    
    return src->line_count;
}

/* Remove comments (// and //! style) */
void preprocessor_remove_comments(PreprocessedSource* src) {
    if (!src) return;
    
    for (int i = 0; i < src->line_count; i++) {
        char* line = src->lines[i];
        
        /* Check for comment markers */
        char* comment = strstr(line, "//");
        if (comment) {
            /* Check if it's a documentation comment //! */
            if (comment > line && *(comment - 1) == '!') {
                /* Remove the entire line */
                free(src->lines[i]);
                /* Shift remaining lines */
                for (int j = i; j < src->line_count - 1; j++) {
                    src->lines[j] = src->lines[j + 1];
                }
                src->line_count--;
                i--; /* Recheck this index */
            } else {
                /* Just truncate at the comment */
                *comment = '\0';
            }
        }
    }
}

/* Remove empty lines */
void preprocessor_remove_empty_lines(PreprocessedSource* src) {
    if (!src) return;
    
    int write_idx = 0;
    for (int read_idx = 0; read_idx < src->line_count; read_idx++) {
        char* line = src->lines[read_idx];
        
        /* Check if line is empty or only whitespace */
        bool is_empty = true;
        for (char* p = line; *p; p++) {
            if (*p != ' ' && *p != '\t' && *p != '\r') {
                is_empty = false;
                break;
            }
        }
        
        if (!is_empty) {
            src->lines[write_idx++] = line;
        } else {
            free(line);
        }
    }
    
    src->line_count = write_idx;
}

/* Trim leading and trailing whitespace */
void preprocessor_trim_whitespace(PreprocessedSource* src) {
    if (!src) return;
    
    for (int i = 0; i < src->line_count; i++) {
        char* line = src->lines[i];
        
        /* Trim leading whitespace */
        char* start = line;
        while (*start == ' ' || *start == '\t') {
            start++;
        }
        
        /* Trim trailing whitespace */
        char* end = start + strlen(start) - 1;
        while (end > start && (*end == ' ' || *end == '\t' || *end == '\r')) {
            *end-- = '\0';
        }
        
        /* Move trimmed line to front if needed */
        if (start != line) {
            memmove(line, start, strlen(start) + 1);
        }
    }
}

/* Print preprocessed source for debugging */
void preprocessor_print(const PreprocessedSource* src) {
    if (!src) return;
    
    printf("=== Preprocessed Source (%d lines) ===\n", src->line_count);
    for (int i = 0; i < src->line_count; i++) {
        printf("%3d: %s\n", i + 1, src->lines[i]);
    }
    printf("=== End ===\n");
}
