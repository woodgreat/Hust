/*
 * hust_translator.h - Translator for Hust-C
 * 
 * Converts Hust AST to C code
 */

#ifndef HUST_TRANSLATOR_H
#define HUST_TRANSLATOR_H

#include "hust_ast.h"

#define HUST_MAX_ARRAYS 128
#define HUST_MAX_STRINGS 128
#define HUST_MAX_OBJECTS 128
#define HUST_MAX_CLASSES 128

typedef struct {
    char name[64];
    char elem_type[32];
    int dims;
    bool is_dynamic;
} ArrayInfo;

typedef struct {
    char name[64];
} StringInfo;

typedef struct {
    char name[64];
    char class_name[64];
} ObjectInfo;

typedef struct {
    char name[64];
    char parent[64];
    bool is_enum;
} ClassInfo;

/* Translator structure */
typedef struct {
    ASTNode* ast;
    FILE* output;
    int indent_level;
    int error_count;
    ArrayInfo arrays[HUST_MAX_ARRAYS];
    int array_count;
    StringInfo strings[HUST_MAX_STRINGS];
    int string_count;
    ObjectInfo objects[HUST_MAX_OBJECTS];
    int object_count;
    ClassInfo classes[HUST_MAX_CLASSES];
    int class_count;
    bool in_match_block;
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
