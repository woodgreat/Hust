/*
 * Hust-C Translator
 * Converts Hust AST to C code
 */

#ifndef TRANSLATOR_H
#define TRANSLATOR_H

#include "parser.h"
#include <stdio.h>

/* Translator structure */
typedef struct {
    ASTNode* ast;
    FILE* output;
    int indent_level;
} Translator;

/* Function prototypes */
Translator* translator_new(ASTNode* ast, FILE* output);
void translator_free(Translator* translator);
int translator_translate(Translator* translator);
void translator_write_indent(Translator* translator);
void translator_translate_node(Translator* translator, ASTNode* node);
void translator_translate_program(Translator* translator, ASTNode* node);
void translator_translate_function(Translator* translator, ASTNode* node);
void translator_translate_variable(Translator* translator, ASTNode* node);
void translator_translate_control_flow(Translator* translator, ASTNode* node);

#endif /* TRANSLATOR_H */
