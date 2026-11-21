/*
 * Hust-C Preprocessor
 * Cleans and prepares Hust source code for parsing
 */

#ifndef PREPROCESSOR_H
#define PREPROCESSOR_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

#define MAX_LINE_LENGTH 1024
#define MAX_LINES 10000

/* Preprocessed source code structure */
typedef struct {
    char** lines;
    int line_count;
    int capacity;
} PreprocessedSource;

/* Function prototypes */
PreprocessedSource* preprocessor_new(void);
void preprocessor_free(PreprocessedSource* src);
int preprocessor_process_file(PreprocessedSource* src, const char* filename);
int preprocessor_process_string(PreprocessedSource* src, const char* code);
void preprocessor_remove_comments(PreprocessedSource* src);
void preprocessor_remove_empty_lines(PreprocessedSource* src);
void preprocessor_trim_whitespace(PreprocessedSource* src);
void preprocessor_print(const PreprocessedSource* src);

#endif /* PREPROCESSOR_H */
