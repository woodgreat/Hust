/*
 * hust_preprocessor.h - Preprocessor for Hust-C
 * 
 * Cleans Hust source code: removes comments, empty lines, trims whitespace
 */

#ifndef HUST_PREPROCESSOR_H
#define HUST_PREPROCESSOR_H

#include "hust_ast.h"

/* Preprocessor structure */
typedef struct {
    char** lines;
    int line_count;
    int line_capacity;
} Preprocessor;

/* Function declarations */
Preprocessor* preprocessor_new(void);
void preprocessor_free(Preprocessor* pp);
int preprocessor_process_file(Preprocessor* pp, const char* filename);
int preprocessor_process_string(Preprocessor* pp, const char* source);
void preprocessor_add_line(Preprocessor* pp, const char* line);
void preprocessor_print(const Preprocessor* pp);

#endif /* HUST_PREPROCESSOR_H */
