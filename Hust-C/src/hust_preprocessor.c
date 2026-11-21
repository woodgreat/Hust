/*
 * hust_preprocessor.c - Preprocessor implementation for Hust-C
 * 
 * Cleans Hust source code: removes comments, empty lines, trims whitespace
 */

#include "../include/hust_preprocessor.h"
#include <ctype.h>

/* Create a new preprocessor */
Preprocessor* preprocessor_new(void) {
    Preprocessor* pp = (Preprocessor*)malloc(sizeof(Preprocessor));
    if (!pp) return NULL;
    
    pp->lines = NULL;
    pp->line_count = 0;
    pp->line_capacity = 0;
    
    return pp;
}

/* Free preprocessor */
void preprocessor_free(Preprocessor* pp) {
    if (!pp) return;
    
    /* Free all lines */
    for (int i = 0; i < pp->line_count; i++) {
        free(pp->lines[i]);
    }
    
    /* Free lines array */
    free(pp->lines);
    
    /* Free preprocessor */
    free(pp);
}

/* Add a line to preprocessor */
void preprocessor_add_line(Preprocessor* pp, const char* line) {
    if (!pp || !line) return;
    
    /* Expand lines array if needed */
    if (pp->line_count >= pp->line_capacity) {
        int new_capacity = pp->line_capacity == 0 ? 16 : pp->line_capacity * 2;
        char** new_lines = (char**)realloc(pp->lines, new_capacity * sizeof(char*));
        if (!new_lines) return;
        
        pp->lines = new_lines;
        pp->line_capacity = new_capacity;
    }
    
    pp->lines[pp->line_count++] = strdup(line);
}

/* Process a file: read, clean, and store lines */
int preprocessor_process_file(Preprocessor* pp, const char* filename) {
    if (!pp || !filename) return -1;
    
    FILE* file = fopen(filename, "r");
    if (!file) {
        fprintf(stderr, "Error: Cannot open file '%s'\n", filename);
        return -1;
    }
    
    char line[1024];
    while (fgets(line, sizeof(line), file)) {
        /* Remove newline */
        size_t len = strlen(line);
        if (len > 0 && line[len-1] == '\n') {
            line[len-1] = '\0';
        }
        
        /* Trim leading whitespace */
        char* start = line;
        while (*start && isspace((unsigned char)*start)) {
            start++;
        }
        
        /* Skip empty lines and comments */
        if (*start == '\0' || *start == '/' || *start == '#') {
            continue;
        }
        
        /* Trim trailing whitespace */
        char* end = start + strlen(start) - 1;
        while (end > start && isspace((unsigned char)*end)) {
            *end-- = '\0';
        }
        
        /* Add cleaned line */
        preprocessor_add_line(pp, start);
    }
    
    fclose(file);
    return 0;
}

/* Process a string: clean and store lines */
int preprocessor_process_string(Preprocessor* pp, const char* source) {
    if (!pp || !source) return -1;
    
    /* Make a copy to avoid modifying const input */
    char* copy = strdup(source);
    if (!copy) return -1;
    
    char* line = strtok(copy, "\n");
    while (line) {
        /* Trim leading whitespace */
        char* start = line;
        while (*start && isspace((unsigned char)*start)) {
            start++;
        }
        
        /* Skip empty lines and comments */
        if (*start != '\0' && *start != '/' && *start != '#') {
            /* Trim trailing whitespace */
            char* end = start + strlen(start) - 1;
            while (end > start && isspace((unsigned char)*end)) {
                *end-- = '\0';
            }
            
            preprocessor_add_line(pp, start);
        }
        
        line = strtok(NULL, "\n");
    }
    
    free(copy);
    return 0;
}

/* Print preprocessed lines */
void preprocessor_print(const Preprocessor* pp) {
    if (!pp) return;
    
    printf("=== Preprocessed Lines (%d) ===\n", pp->line_count);
    for (int i = 0; i < pp->line_count; i++) {
        printf("%3d: %s\n", i + 1, pp->lines[i]);
    }
}
