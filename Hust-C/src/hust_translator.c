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

/* ============================================
 * Function Translation Helpers
 * ============================================ */

/* Helper: Check if function is main */
static bool translator_is_main_function(const char* line) {
    if (!line) return false;
    return (strncmp(line, "void main()", 11) == 0);
}

/* Helper: Remove 'public ' prefix from line */
static const char* translator_remove_public_prefix(const char* line) {
    if (!line) return NULL;
    if (strncmp(line, "public ", 7) == 0) {
        return line + 7;
    }
    return line;
}

/* Helper: Parse function signature into components */
typedef struct {
    char return_type[32];
    char func_name[64];
    char params[128];
} FunctionSignature;

static bool translator_parse_function_signature(const char* line, FunctionSignature* sig) {
    if (!line || !sig) return false;
    
    int i = 0, j = 0;
    
    /* Extract return type */
    while (line[i] && line[i] != ' ' && line[i] != '\t' && j < 31) {
        sig->return_type[j++] = line[i++];
    }
    sig->return_type[j] = '\0';
    
    /* Skip whitespace */
    while (line[i] == ' ' || line[i] == '\t') i++;
    
    /* Extract function name */
    j = 0;
    while (line[i] && line[i] != '(' && j < 63) {
        sig->func_name[j++] = line[i++];
    }
    sig->func_name[j] = '\0';
    
    /* Skip ( */
    if (line[i] == '(') i++;
    
    /* Extract parameters */
    j = 0;
    while (line[i] && line[i] != ')' && j < 127) {
        sig->params[j++] = line[i++];
    }
    sig->params[j] = '\0';
    
    return true;
}

/* Helper: Convert a single parameter from Hust to C */
static void translator_convert_param(const char* param, char* c_param, size_t c_param_size) {
    if (!param || !c_param || c_param_size == 0) return;
    
    char p_type[32] = {0};
    char p_name[32] = {0};
    int k = 0, l = 0;
    
    /* Trim leading whitespace */
    while (*param == ' ' || *param == '\t') param++;
    
    /* Extract type */
    while (param[k] && param[k] != ' ' && param[k] != '\t' && l < 31) {
        p_type[l++] = param[k++];
    }
    p_type[l] = '\0';
    
    /* Skip whitespace */
    while (param[k] == ' ' || param[k] == '\t') k++;
    
    /* Extract name */
    l = 0;
    while (param[k] && param[k] != ' ' && param[k] != '\t' && param[k] != ',' && l < 31) {
        p_name[l++] = param[k++];
    }
    p_name[l] = '\0';
    
    /* Convert type */
    const char* c_type = get_c_type(p_type);
    
    /* Build C parameter */
    snprintf(c_param, c_param_size, "%s %s", c_type, p_name);
}

/* Helper: Convert all parameters from Hust to C */
static void translator_convert_params(const char* params, char* c_params, size_t c_params_size) {
    if (!params || !c_params || c_params_size == 0) return;
    
    c_params[0] = '\0';
    
    if (strlen(params) == 0) {
        strncpy(c_params, "void", c_params_size - 1);
        c_params[c_params_size - 1] = '\0';
        return;
    }
    
    /* Parse each parameter: type name */
    char param_copy[128];
    strncpy(param_copy, params, sizeof(param_copy) - 1);
    param_copy[sizeof(param_copy) - 1] = '\0';
    
    char* token = strtok(param_copy, ",");
    bool first = true;
    
    while (token) {
        char c_param[64] = {0};
        translator_convert_param(token, c_param, sizeof(c_param));
        
        if (!first) {
            strncat(c_params, ", ", c_params_size - strlen(c_params) - 1);
        }
        strncat(c_params, c_param, c_params_size - strlen(c_params) - 1);
        
        first = false;
        token = strtok(NULL, ",");
    }
}

/* Helper: Output function signature to file */
static void translator_output_function_signature(Translator* translator, const char* return_type, const char* func_name, const char* params) {
    if (!translator || !return_type || !func_name || !params) return;
    
    fprintf(translator->output, "%s %s(%s) {\n", return_type, func_name, params);
    translator->indent_level++;
}

/* Translate function definition - refactored */
void translator_translate_function(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    char line[256];
    strncpy(line, node->value, sizeof(line) - 1);
    line[sizeof(line) - 1] = '\0';
    
    /* Check for main function */
    if (translator_is_main_function(line)) {
        fprintf(translator->output, "void main(void) {\n");
        translator->indent_level++;
        return;
    }
    
    /* Remove 'public ' prefix */
    const char* func_start = translator_remove_public_prefix(line);
    
    /* Parse function signature */
    FunctionSignature sig = {0};
    if (!translator_parse_function_signature(func_start, &sig)) {
        fprintf(stderr, "Error: Failed to parse function signature\n");
        return;
    }
    
    /* Convert return type */
    const char* c_return_type = get_c_type(sig.return_type);
    
    /* Convert parameters */
    char c_params[256] = {0};
    translator_convert_params(sig.params, c_params, sizeof(c_params));
    
    /* Output function signature */
    translator_output_function_signature(translator, c_return_type, sig.func_name, c_params);
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
