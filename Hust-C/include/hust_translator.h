/*
 * hust_translator.h - Translator for Hust-C
 * 
 * Converts Hust AST to C code
 */

#ifndef HUST_TRANSLATOR_H
#define HUST_TRANSLATOR_H

#include "hust_ast.h"

/* Translator structure */
typedef struct {
    ASTNode* ast;
    FILE* output;
    int indent_level;
} Translator;

/* Function declarations */
Translator* translator_new(ASTNode* ast, FILE* output);
void translator_free(Translator* translator);
int translator_translate(Translator* translator);
void translator_write_indent(Translator* translator);
void translator_translate_node(Translator* translator, ASTNode* node);
void translator_translate_program(Translator* translator, ASTNode* node);
void translator_translate_function(Translator* translator, ASTNode* node);
void translator_translate_variable(Translator* translator, ASTNode* node);
void translator_translate_statement(Translator* translator, ASTNode* node);
void translator_translate_control_flow(Translator* translator, ASTNode* node);
void translator_translate_class(Translator* translator, ASTNode* node);
void translator_translate_interface(Translator* translator, ASTNode* node);
void translator_translate_class(Translator* translator, ASTNode* node);
void translator_translate_interface(Translator* translator, ASTNode* node);
void translator_translate_class(Translator* translator, ASTNode* node);
void translator_translate_interface(Translator* translator, ASTNode* node);

#endif /* HUST_TRANSLATOR_H */
