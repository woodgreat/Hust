/*
 * Hust-C Translator Implementation
 * Converts Hust AST to C code
 */

#include "translator.h"
#include "c_mappings.h"
#include <stdlib.h>
#include <string.h>

/* Create a new translator */
Translator* translator_new(ASTNode* ast, FILE* output) {
    Translator* translator = (Translator*)malloc(sizeof(Translator));
    if (!translator) return NULL;
    
    translator->ast = ast;
    translator->output = output ? output : stdout;
    translator->indent_level = 0;
    
    return translator;
}

/* Free translator */
void translator_free(Translator* translator) {
    if (!translator) return;
    free(translator);
}

/* Write indentation */
void translator_write_indent(Translator* translator) {
    for (int i = 0; i < translator->indent_level; i++) {
        fprintf(translator->output, "    ");
    }
}

/* Translate AST to C code */
int translator_translate(Translator* translator) {
    if (!translator || !translator->ast) return -1;
    
    /* Write C header */
    fprintf(translator->output, "#include <stdbool.h>\n");
    fprintf(translator->output, "#include <stdint.h>\n");
    fprintf(translator->output, "#include <stdio.h>\n");
    fprintf(translator->output, "#include <stdlib.h>\n");
    fprintf(translator->output, "#include <string.h>\n");
    fprintf(translator->output, "\n");
    
    /* Translate root node */
    translator_translate_node(translator, translator->ast);
    
    return 0;
}

/* Translate a single AST node */
void translator_translate_node(Translator* translator, ASTNode* node) {
    if (!translator || !node) return;
    
    /* Check for closing brace */
    if (node->value && strcmp(node->value, "}") == 0) {
        translator->indent_level--;
        translator_write_indent(translator);
        fprintf(translator->output, "}\n");
        return;
    }
    
    switch (node->type) {
        case NODE_PROGRAM:
            translator_translate_program(translator, node);
            break;
        case NODE_FUNCTION_DEF:
            translator_translate_function(translator, node);
            break;
        case NODE_VARIABLE_DECL:
            translator_translate_variable(translator, node);
            break;
        case NODE_CONTROL_FLOW:
            translator_translate_control_flow(translator, node);
            break;
        default:
            /* Copy as-is for now */
            translator_write_indent(translator);
            fprintf(translator->output, "%s\n", node->value ? node->value : "");
            break;
    }
}

/* Translate program node (root) */
void translator_translate_program(Translator* translator, ASTNode* node) {
    if (!translator || !node) return;
    
    /* Translate all children */
    for (int i = 0; i < node->child_count; i++) {
        translator_translate_node(translator, node->children[i]);
    }
}

/* Translate function definition */
void translator_translate_function(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    /* Parse function signature */
    /* For now, just copy the line and fix main() */
    if (strncmp(node->value, "void main()", 11) == 0) {
        fprintf(translator->output, "void main(void) {\n");
        translator->indent_level++;
    } else {
        fprintf(translator->output, "%s\n", node->value);
    }
}

/* Translate variable declaration */
void translator_translate_variable(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    /* Parse: type name = value; or type name; */
    char type[32], rest[256];
    int i = 0;
    
    /* Extract type */
    while (node->value[i] && node->value[i] != ' ' && node->value[i] != '\t' && i < 31) {
        type[i] = node->value[i];
        i++;
    }
    type[i] = '\0';
    
    /* Skip whitespace */
    while (node->value[i] == ' ' || node->value[i] == '\t') i++;
    
    /* Rest of line */
    strcpy(rest, &node->value[i]);
    
    /* Get C type */
    const char* c_type = get_c_type(type);
    
    /* Output C declaration */
    translator_write_indent(translator);
    fprintf(translator->output, "%s %s", c_type, rest);
    
    /* Ensure semicolon */
    if (!strchr(rest, ';')) {
        fprintf(translator->output, ";");
    }
    
    fprintf(translator->output, "\n");
}

/* Translate control flow */
void translator_translate_control_flow(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    /* For now, just copy the line */
    translator_write_indent(translator);
    fprintf(translator->output, "%s\n", node->value);
}
