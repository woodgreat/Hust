/*
 * hust_translator.c - Translator implementation for Hust-C
 * 
 * Converts Hust AST to C code
 */

#include "../include/hust_translator.h"
#include "../include/hust_mappings.h"
#include <ctype.h>

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
    if (!translator) return;
    
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
        case NODE_RETURN:
        case NODE_STATEMENT:
            translator_translate_statement(translator, node);
            break;
        case NODE_BLOCK:
            /* Handle closing brace */
            if (node->value && strcmp(node->value, "}") == 0) {
                translator->indent_level--;
                if (translator->indent_level < 0) translator->indent_level = 0;
                translator_write_indent(translator);
                fprintf(translator->output, "}\n");
            }
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
    
    char line[256];
    strncpy(line, node->value, sizeof(line) - 1);
    line[sizeof(line) - 1] = '\0';
    
    /* Check for main function */
    if (strncmp(line, "void main()", 11) == 0) {
        fprintf(translator->output, "void main(void) {\n");
        translator->indent_level++;
        return;
    }
    
    /* Remove 'public ' prefix */
    char* func_start = line;
    if (strncmp(line, "public ", 7) == 0) {
        func_start = line + 7;
    }
    
    /* Parse function signature: return_type name(params) { */
    char return_type[32] = {0};
    char func_name[64] = {0};
    char params[128] = {0};
    int i = 0, j = 0;
    
    /* Extract return type */
    while (func_start[i] && func_start[i] != ' ' && func_start[i] != '\t' && j < 31) {
        return_type[j++] = func_start[i++];
    }
    return_type[j] = '\0';
    
    /* Skip whitespace */
    while (func_start[i] == ' ' || func_start[i] == '\t') i++;
    
    /* Extract function name */
    j = 0;
    while (func_start[i] && func_start[i] != '(' && j < 63) {
        func_name[j++] = func_start[i++];
    }
    func_name[j] = '\0';
    
    /* Skip ( */
    if (func_start[i] == '(') i++;
    
    /* Extract parameters */
    j = 0;
    while (func_start[i] && func_start[i] != ')' && j < 127) {
        params[j++] = func_start[i++];
    }
    params[j] = '\0';
    
    /* Convert return type */
    const char* c_return_type = get_c_type(return_type);
    
    /* Convert parameters */
    char c_params[256] = {0};
    if (strlen(params) > 0) {
        /* Parse each parameter: type name */
        char param_copy[128];
        strncpy(param_copy, params, sizeof(param_copy) - 1);
        param_copy[sizeof(param_copy) - 1] = '\0';
        
        char* token = strtok(param_copy, ",");
        bool first = true;
        
        while (token) {
            /* Trim whitespace */
            while (*token == ' ' || *token == '\t') token++;
            
            /* Parse type and name */
            char p_type[32] = {0};
            char p_name[32] = {0};
            int k = 0, l = 0;
            
            /* Extract type */
            while (token[k] && token[k] != ' ' && token[k] != '\t' && l < 31) {
                p_type[l++] = token[k++];
            }
            p_type[l] = '\0';
            
            /* Skip whitespace */
            while (token[k] == ' ' || token[k] == '\t') k++;
            
            /* Extract name */
            l = 0;
            while (token[k] && token[k] != ' ' && token[k] != '\t' && token[k] != ',' && l < 31) {
                p_name[l++] = token[k++];
            }
            p_name[l] = '\0';
            
            /* Convert type */
            const char* c_type = get_c_type(p_type);
            
            /* Add to params */
            if (!first) {
                strncat(c_params, ", ", sizeof(c_params) - strlen(c_params) - 1);
            }
            strncat(c_params, c_type, sizeof(c_params) - strlen(c_params) - 1);
            strncat(c_params, " ", sizeof(c_params) - strlen(c_params) - 1);
            strncat(c_params, p_name, sizeof(c_params) - strlen(c_params) - 1);
            
            first = false;
            token = strtok(NULL, ",");
        }
    } else {
        strncpy(c_params, "void", sizeof(c_params) - 1);
    }
    
    /* Output function signature */
    fprintf(translator->output, "%s %s(%s) {\n", c_return_type, func_name, c_params);
    translator->indent_level++;
}

/* Translate variable declaration */
void translator_translate_variable(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    /* Parse: type name = value; or type name; */
    char type[32] = {0};
    char rest[256] = {0};
    int i = 0, j = 0;
    
    /* Extract type */
    while (node->value[i] && node->value[i] != ' ' && node->value[i] != '\t' && j < 31) {
        type[j++] = node->value[i++];
    }
    type[j] = '\0';
    
    /* Skip whitespace */
    while (node->value[i] == ' ' || node->value[i] == '\t') i++;
    
    /* Rest of line */
    strncpy(rest, &node->value[i], sizeof(rest) - 1);
    rest[sizeof(rest) - 1] = '\0';
    
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

/* Translate statement */
void translator_translate_statement(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    translator_write_indent(translator);
    fprintf(translator->output, "%s\n", node->value);
}

/* Translate control flow */
void translator_translate_control_flow(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    /* For now, just copy the line */
    translator_write_indent(translator);
    fprintf(translator->output, "%s\n", node->value);
}
