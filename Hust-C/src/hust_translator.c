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

/* ============================================
 * Control Flow Translation Helpers
 * ============================================ */

/* Helper: Extract condition from control flow statement */
static bool translator_extract_condition(const char* line, const char* keyword, char* condition, size_t cond_size) {
    if (!line || !keyword || !condition || cond_size == 0) return false;
    
    /* Find keyword */
    const char* kw_pos = strstr(line, keyword);
    if (!kw_pos) return false;
    
    /* Skip keyword and whitespace */
    kw_pos += strlen(keyword);
    while (*kw_pos == ' ' || *kw_pos == '\t') kw_pos++;
    
    /* Expect opening paren */
    if (*kw_pos != '(') return false;
    kw_pos++;
    
    /* Find closing paren (matching) */
    int paren_count = 1;
    const char* end = kw_pos;
    while (*end && paren_count > 0) {
        if (*end == '(') paren_count++;
        else if (*end == ')') paren_count--;
        if (paren_count > 0) end++;
    }
    
    if (paren_count != 0) return false;  /* Unbalanced parens */
    
    /* Extract condition */
    size_t len = end - kw_pos;
    if (len >= cond_size) len = cond_size - 1;
    strncpy(condition, kw_pos, len);
    condition[len] = '\0';
    
    return true;
}

/* Helper: Convert Hust types in condition to C types */
static void translator_convert_condition_types(char* condition, size_t cond_size) {
    if (!condition || cond_size == 0) return;
    
    /* For now, handle simple type conversions in condition */
    /* TODO: More sophisticated type conversion */
    (void)cond_size;  /* Unused for now */
}

/* Helper: Translate if statement */
static void translator_translate_if(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    char condition[256] = {0};
    if (!translator_extract_condition(line, "if", condition, sizeof(condition))) {
        /* Fallback: copy as-is */
        translator_write_indent(translator);
        fprintf(translator->output, "%s\n", line);
        return;
    }
    
    translator_convert_condition_types(condition, sizeof(condition));
    
    translator_write_indent(translator);
    fprintf(translator->output, "if (%s) {\n", condition);
    translator->indent_level++;
}

/* Helper: Translate while statement */
static void translator_translate_while(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    char condition[256] = {0};
    if (!translator_extract_condition(line, "while", condition, sizeof(condition))) {
        /* Fallback: copy as-is */
        translator_write_indent(translator);
        fprintf(translator->output, "%s\n", line);
        return;
    }
    
    translator_convert_condition_types(condition, sizeof(condition));
    
    translator_write_indent(translator);
    fprintf(translator->output, "while (%s) {\n", condition);
    translator->indent_level++;
}

/* Helper: Translate for statement */
static void translator_translate_for(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Extract the three parts of for loop: init; condition; increment */
    const char* for_pos = strstr(line, "for");
    if (!for_pos) {
        translator_write_indent(translator);
        fprintf(translator->output, "%s\n", line);
        return;
    }
    
    /* Skip 'for' and whitespace */
    for_pos += 3;
    while (*for_pos == ' ' || *for_pos == '\t') for_pos++;
    
    /* Expect opening paren */
    if (*for_pos != '(') {
        translator_write_indent(translator);
        fprintf(translator->output, "%s\n", line);
        return;
    }
    for_pos++;
    
    /* Find the three parts separated by semicolons */
    char init[128] = {0};
    char condition[128] = {0};
    char increment[128] = {0};
    
    int part = 0;
    int j = 0;
    const char* p = for_pos;
    
    while (*p && *p != ')' && part < 3) {
        if (*p == ';') {
            part++;
            j = 0;
            p++;
            continue;
        }
        
        if (part == 0 && j < 127) {
            init[j++] = *p;
        } else if (part == 1 && j < 127) {
            condition[j++] = *p;
        } else if (part == 2 && j < 127) {
            increment[j++] = *p;
        }
        p++;
    }
    
    /* Null terminate */
    if (part >= 0) init[strlen(init)] = '\0';
    if (part >= 1) condition[strlen(condition)] = '\0';
    if (part >= 2) increment[strlen(increment)] = '\0';
    
    /* Trim whitespace */
    char* init_trim = init;
    char* cond_trim = condition;
    char* incr_trim = increment;
    
    while (*init_trim == ' ' || *init_trim == '\t') init_trim++;
    while (*cond_trim == ' ' || *cond_trim == '\t') cond_trim++;
    while (*incr_trim == ' ' || *incr_trim == '\t') incr_trim++;
    
    /* Convert init part (may contain type declaration) */
    char c_init[256] = {0};
    if (strlen(init_trim) > 0) {
        /* Check if it starts with a type */
        if (strncmp(init_trim, "i32 ", 4) == 0) {
            snprintf(c_init, sizeof(c_init), "int32_t %s", init_trim + 4);
        } else if (strncmp(init_trim, "i64 ", 4) == 0) {
            snprintf(c_init, sizeof(c_init), "int64_t %s", init_trim + 4);
        } else if (strncmp(init_trim, "f32 ", 4) == 0) {
            snprintf(c_init, sizeof(c_init), "float %s", init_trim + 4);
        } else if (strncmp(init_trim, "f64 ", 4) == 0) {
            snprintf(c_init, sizeof(c_init), "double %s", init_trim + 4);
        } else if (strncmp(init_trim, "bool ", 5) == 0) {
            snprintf(c_init, sizeof(c_init), "bool %s", init_trim + 5);
        } else if (strncmp(init_trim, "char ", 5) == 0) {
            snprintf(c_init, sizeof(c_init), "char %s", init_trim + 5);
        } else {
            strncpy(c_init, init_trim, sizeof(c_init) - 1);
        }
    }
    
    /* Output for loop */
    translator_write_indent(translator);
    fprintf(translator->output, "for (%s; %s; %s) {\n", c_init, cond_trim, incr_trim);
    translator->indent_level++;
}

/* Translate control flow */
void translator_translate_control_flow(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    const char* line = node->value;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Check for else */
    if (strncmp(line, "else", 4) == 0) {
        translator->indent_level--;
        translator_write_indent(translator);
        
        /* Check for else if */
        if (strncmp(line, "else if", 7) == 0) {
            /* Handle as if with else prefix */
            char condition[256] = {0};
            if (translator_extract_condition(line, "if", condition, sizeof(condition))) {
                fprintf(translator->output, "else if (%s) {\n", condition);
                translator->indent_level++;
                return;
            }
        } else {
            /* Plain else */
            fprintf(translator->output, "else {\n");
            translator->indent_level++;
            return;
        }
    }
    
    /* Dispatch to specific control flow translators */
    if (strncmp(line, "if", 2) == 0) {
        translator_translate_if(translator, line);
    } else if (strncmp(line, "while", 5) == 0) {
        translator_translate_while(translator, line);
    } else if (strncmp(line, "for", 3) == 0) {
        translator_translate_for(translator, line);
    } else {
        /* Fallback: copy as-is */
        translator_write_indent(translator);
        fprintf(translator->output, "%s\n", line);
    }
}
