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
        case NODE_CLASS_DEF:
            translator_translate_class(translator, node);
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

/* ============================================
 * Expression Translation Helpers
 * ============================================ */

/* Helper: Check if line contains a function call */
static bool translator_is_function_call(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: identifier(...) */
    /* Skip if it's a control flow keyword */
    if (strncmp(line, "if", 2) == 0 ||
        strncmp(line, "while", 5) == 0 ||
        strncmp(line, "for", 3) == 0 ||
        strncmp(line, "return", 6) == 0 ||
        strncmp(line, "else", 4) == 0) {
        return false;
    }
    
    /* Look for identifier followed by ( */
    const char* p = line;
    while (*p) {
        /* Skip whitespace */
        while (*p == ' ' || *p == '\t') p++;
        
        /* Check for identifier start */
        if (isalpha(*p) || *p == '_') {
            /* Find end of identifier */
            const char* id_start = p;
            while (isalnum(*p) || *p == '_') p++;
            const char* id_end = p;
            
            /* Skip whitespace */
            while (*p == ' ' || *p == '\t') p++;
            
            /* Check if next char is ( */
            if (*p == '(') {
                /* Check if identifier is a type (i32, i64, etc.) */
                char identifier[32] = {0};
                size_t id_len = id_end - id_start;
                if (id_len < 32) {
                    strncpy(identifier, id_start, id_len);
                    identifier[id_len] = '\0';
                    
                    /* Not a type, so it's a function call */
                    if (get_c_type(identifier) == NULL) {
                        return true;
                    }
                }
            }
        } else {
            p++;
        }
    }
    
    return false;
}

/* Helper: Check if line contains a method call (obj.method(...)) */
static bool translator_is_method_call(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: identifier.identifier(...) */
    const char* p = line;
    while (*p) {
        /* Skip whitespace */
        while (*p == ' ' || *p == '\t') p++;
        
        /* Check for identifier start */
        if (isalpha(*p) || *p == '_') {
            /* Find end of identifier */
            while (isalnum(*p) || *p == '_') p++;
            
            /* Skip whitespace */
            while (*p == ' ' || *p == '\t') p++;
            
            /* Check if next char is . */
            if (*p == '.') {
                p++;
                
                /* Skip whitespace */
                while (*p == ' ' || *p == '\t') p++;
                
                /* Check for method name */
                if (isalpha(*p) || *p == '_') {
                    /* Find end of method name */
                    while (isalnum(*p) || *p == '_') p++;
                    
                    /* Skip whitespace */
                    while (*p == ' ' || *p == '\t') p++;
                    
                    /* Check if next char is ( */
                    if (*p == '(') {
                        return true;
                    }
                }
            }
        } else {
            p++;
        }
    }
    
    return false;
}

/* Helper: Extract object name and method call from line */
static bool translator_extract_method_call(const char* line, char* obj_name, size_t obj_size, char* method_call, size_t method_size) {
    if (!line || !obj_name || !method_call || obj_size == 0 || method_size == 0) return false;
    
    /* Find pattern: obj.method(args) */
    const char* dot = strchr(line, '.');
    if (!dot) return false;
    
    /* Extract object name (before dot) */
    const char* obj_start = line;
    while (*obj_start == ' ' || *obj_start == '\t') obj_start++;
    
    size_t obj_len = dot - obj_start;
    if (obj_len >= obj_size) obj_len = obj_size - 1;
    strncpy(obj_name, obj_start, obj_len);
    obj_name[obj_len] = '\0';
    
    /* Trim trailing whitespace from object name */
    char* obj_end = obj_name + strlen(obj_name) - 1;
    while (obj_end > obj_name && (*obj_end == ' ' || *obj_end == '\t')) {
        *obj_end = '\0';
        obj_end--;
    }
    
    /* Extract method call (after dot) */
    const char* method_start = dot + 1;
    size_t method_len = strlen(method_start);
    if (method_len >= method_size) method_len = method_size - 1;
    memcpy(method_call, method_start, method_len);
    method_call[method_len] = '\0';
    
    return true;
}

/* Helper: Convert method call to C function call */
static void translator_convert_method_call(const char* obj_name, const char* method_call, char* c_call, size_t c_call_size) {
    if (!obj_name || !method_call || !c_call || c_call_size == 0) return;
    
    /* Extract method name and args from method_call */
    char method_name[64] = {0};
    char args[256] = {0};
    
    int i = 0, j = 0;
    while (method_call[i] && method_call[i] != '(' && j < 63) {
        method_name[j++] = method_call[i++];
    }
    method_name[j] = '\0';
    
    /* Skip ( */
    if (method_call[i] == '(') i++;
    
    /* Extract args */
    j = 0;
    while (method_call[i] && method_call[i] != ')' && j < 255) {
        args[j++] = method_call[i++];
    }
    args[j] = '\0';
    
    /* Build C function call: Type_method(obj, args) */
    /* For now, we use a generic naming convention: obj_method(args) */
    /* TODO: Need type information to generate proper Type_method() call */
    if (strlen(args) > 0) {
        snprintf(c_call, c_call_size, "%s_%s(%s, %s)", obj_name, method_name, obj_name, args);
    } else {
        snprintf(c_call, c_call_size, "%s_%s(%s)", obj_name, method_name, obj_name);
    }
}

/* Helper: Check if line is an array declaration */
static bool translator_is_array_declaration(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: type[N] name or type[] name */
    /* Check for [ after type */
    const char* bracket = strchr(line, '[');
    if (!bracket) return false;
    
    /* Check if it's a type declaration (starts with known type) */
    if (strncmp(line, "i32", 3) == 0 ||
        strncmp(line, "i64", 3) == 0 ||
        strncmp(line, "f32", 3) == 0 ||
        strncmp(line, "f64", 3) == 0 ||
        strncmp(line, "bool", 4) == 0 ||
        strncmp(line, "char", 4) == 0 ||
        strncmp(line, "String", 6) == 0) {
        return true;
    }
    
    return false;
}

/* Helper: Translate array declaration */
static void translator_translate_array_decl(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Parse: type[N] name = {init}; or type[] name; */
    char type[32] = {0};
    char size[32] = {0};
    char rest[256] = {0};
    int i = 0, j = 0;
    
    /* Extract type */
    while (line[i] && line[i] != '[' && j < 31) {
        type[j++] = line[i++];
    }
    type[j] = '\0';
    
    /* Skip [ */
    if (line[i] == '[') i++;
    
    /* Extract size (may be empty for dynamic arrays) */
    j = 0;
    while (line[i] && line[i] != ']' && j < 31) {
        size[j++] = line[i++];
    }
    size[j] = '\0';
    
    /* Skip ] */
    if (line[i] == ']') i++;
    
    /* Rest of line */
    strncpy(rest, &line[i], sizeof(rest) - 1);
    rest[sizeof(rest) - 1] = '\0';
    
    /* Get C type */
    const char* c_type = get_c_type(type);
    
    /* Output C declaration */
    translator_write_indent(translator);
    
    if (strlen(size) > 0) {
        /* Fixed-size array: type name[N] = {init}; */
        fprintf(translator->output, "%s %s", c_type, rest);
        /* Insert size before rest */
        /* Actually, we need to reconstruct: type name[size] = init; */
        /* For now, simple approach: type name rest, and add size later */
        /* Better approach: parse name and init from rest */
    } else {
        /* Dynamic array: HustVec_type* name; */
        fprintf(translator->output, "HustVec_%s* %s", c_type, rest);
    }
    
    /* Ensure semicolon */
    if (!strchr(rest, ';')) {
        fprintf(translator->output, ";");
    }
    
    fprintf(translator->output, "\n");
}

/* Helper: Check if line is a string declaration */
static bool translator_is_string_declaration(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: String name = "..." */
    if (strncmp(line, "String ", 7) == 0) {
        return true;
    }
    
    return false;
}

/* Helper: Translate string declaration */
static void translator_translate_string_decl(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Parse: String name = "value"; or String name; */
    char name[64] = {0};
    char value[256] = {0};
    int i = 7, j = 0;  /* Skip "String " */
    
    /* Extract name */
    while (line[i] && line[i] != ' ' && line[i] != '\t' && line[i] != '=' && line[i] != ';' && j < 63) {
        name[j++] = line[i++];
    }
    name[j] = '\0';
    
    /* Skip whitespace and = */
    while (line[i] == ' ' || line[i] == '\t') i++;
    if (line[i] == '=') i++;
    while (line[i] == ' ' || line[i] == '\t') i++;
    
    /* Extract value (if any) */
    if (line[i] == '"') {
        i++;  /* Skip opening quote */
        j = 0;
        while (line[i] && line[i] != '"' && j < 255) {
            value[j++] = line[i++];
        }
        value[j] = '\0';
    }
    
    /* Output C declaration */
    translator_write_indent(translator);
    
    if (strlen(value) > 0) {
        /* String with initialization: HustString* name = HustString_new("value"); */
        fprintf(translator->output, "HustString* %s = HustString_new(\"%s\");\n", name, value);
    } else {
        /* String without initialization: HustString* name = NULL; */
        fprintf(translator->output, "HustString* %s = NULL;\n", name);
    }
}

/* ============================================
 * Class Translation
 * ============================================ */

/* Helper: Check if line is a class definition */
static bool translator_is_class_definition(const char* line) {
    if (!line) return false;
    return (strncmp(line, "class ", 6) == 0);
}

/* Helper: Check if line is an interface definition */
static bool translator_is_interface_definition(const char* line) {
    if (!line) return false;
    return (strncmp(line, "interface ", 10) == 0);
}

/* Helper: Extract class name from class definition line */
static bool translator_extract_class_name(const char* line, char* class_name, size_t name_size) {
    if (!line || !class_name || name_size == 0) return false;
    
    /* Skip "class " */
    const char* p = line + 6;
    
    /* Skip whitespace */
    while (*p == ' ' || *p == '\t') p++;
    
    /* Extract name */
    int j = 0;
    while (*p && *p != ' ' && *p != '\t' && *p != '{' && j < (int)name_size - 1) {
        class_name[j++] = *p++;
    }
    class_name[j] = '\0';
    
    return (j > 0);
}

/* Helper: Check if line is a field declaration (inside class) */
static bool translator_is_field_declaration(const char* line) {
    if (!line) return false;
    
    /* Check if line starts with a type */
    return (strncmp(line, "i32 ", 4) == 0 ||
            strncmp(line, "i64 ", 4) == 0 ||
            strncmp(line, "f32 ", 4) == 0 ||
            strncmp(line, "f64 ", 4) == 0 ||
            strncmp(line, "bool ", 5) == 0 ||
            strncmp(line, "char ", 5) == 0 ||
            strncmp(line, "String ", 7) == 0);
}

/* Helper: Check if line is a method definition (inside class) */
static bool translator_is_method_definition(const char* line) {
    if (!line) return false;
    
    /* Method definitions have type + name + (params) pattern */
    /* Similar to function definition but inside class */
    
    /* Check for return type */
    const char* p = line;
    
    /* Skip public prefix */
    if (strncmp(p, "public ", 7) == 0) {
        p += 7;
    }
    
    /* Check for type */
    if (strncmp(p, "i32 ", 4) == 0 ||
        strncmp(p, "i64 ", 4) == 0 ||
        strncmp(p, "f32 ", 4) == 0 ||
        strncmp(p, "f64 ", 4) == 0 ||
        strncmp(p, "bool ", 5) == 0 ||
        strncmp(p, "char ", 5) == 0 ||
        strncmp(p, "String ", 7) == 0 ||
        strncmp(p, "void ", 5) == 0) {
        /* Look for ( after method name */
        const char* paren = strchr(p, '(');
        if (paren) {
            return true;
        }
    }
    
    return false;
}

/* Helper: Translate class field to C struct member */
static void translator_translate_field(Translator* translator, const char* line, FILE* struct_output) {
    if (!translator || !line || !struct_output) return;
    
    /* Parse: type name; or type name = value; */
    char type[32] = {0};
    char rest[256] = {0};
    int i = 0, j = 0;
    
    /* Skip public prefix */
    if (strncmp(line, "public ", 7) == 0) {
        i = 7;
    }
    
    /* Extract type */
    while (line[i] && line[i] != ' ' && line[i] != '\t' && j < 31) {
        type[j++] = line[i++];
    }
    type[j] = '\0';
    
    /* Skip whitespace */
    while (line[i] == ' ' || line[i] == '\t') i++;
    
    /* Rest of line */
    strncpy(rest, &line[i], sizeof(rest) - 1);
    rest[sizeof(rest) - 1] = '\0';
    
    /* Get C type */
    const char* c_type = get_c_type(type);
    
    /* Output struct member */
    fprintf(struct_output, "    %s %s", c_type, rest);
    if (!strchr(rest, ';')) {
        fprintf(struct_output, ";");
    }
    fprintf(struct_output, "\n");
}

/* Helper: Translate class method to C function */
static void translator_translate_method(Translator* translator, const char* line, const char* class_name) {
    if (!translator || !line || !class_name) return;
    
    /* For now, output method as comment */
    /* TODO: Full method translation with self parameter */
    translator_write_indent(translator);
    fprintf(translator->output, "/* Method: %s */\n", line);
}

/* Translate class definition */
void translator_translate_class(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    const char* line = node->value;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Check for class definition */
    if (!translator_is_class_definition(line)) {
        /* Not a class, copy as-is */
        translator_write_indent(translator);
        fprintf(translator->output, "%s\n", line);
        return;
    }
    
    /* Extract class name */
    char class_name[64] = {0};
    if (!translator_extract_class_name(line, class_name, sizeof(class_name))) {
        fprintf(stderr, "Error: Failed to extract class name\n");
        return;
    }
    
    /* Output struct definition */
    translator_write_indent(translator);
    fprintf(translator->output, "typedef struct {\n");
    translator->indent_level++;
    
    /* Parse class body (multi-line) */
    /* Split node->value by newlines and process each line */
    char* body = strdup(node->value);
    if (body) {
        char* saveptr = NULL;
        char* body_line = strtok_r(body, "\n", &saveptr);
        bool first_line = true;
        
        while (body_line) {
            /* Skip first line (class declaration) */
            if (first_line) {
                first_line = false;
                body_line = strtok_r(NULL, "\n", &saveptr);
                continue;
            }
            
            /* Skip empty lines and closing brace */
            char* trimmed = body_line;
            while (*trimmed == ' ' || *trimmed == '\t') trimmed++;
            
            if (*trimmed == '\0' || *trimmed == '}') {
                body_line = strtok_r(NULL, "\n", &saveptr);
                continue;
            }
            
            /* Check if it's a field declaration */
            if (translator_is_field_declaration(trimmed)) {
                /* Parse field: type name; or type name = value; */
                char field_type[32] = {0};
                char field_rest[256] = {0};
                int i = 0, j = 0;
                
                /* Skip public prefix */
                if (strncmp(trimmed, "public ", 7) == 0) {
                    i = 7;
                }
                
                /* Extract type */
                while (trimmed[i] && trimmed[i] != ' ' && trimmed[i] != '\t' && j < 31) {
                    field_type[j++] = trimmed[i++];
                }
                field_type[j] = '\0';
                
                /* Skip whitespace */
                while (trimmed[i] == ' ' || trimmed[i] == '\t') i++;
                
                /* Rest of line */
                strncpy(field_rest, &trimmed[i], sizeof(field_rest) - 1);
                field_rest[sizeof(field_rest) - 1] = '\0';
                
                /* Get C type */
                const char* c_type = get_c_type(field_type);
                
                /* Output struct member */
                translator_write_indent(translator);
                fprintf(translator->output, "%s %s", c_type, field_rest);
                if (!strchr(field_rest, ';')) {
                    fprintf(translator->output, ";");
                }
                fprintf(translator->output, "\n");
            }
            /* Check if it's a method definition */
            else if (translator_is_method_definition(trimmed)) {
                /* For now, output method as comment */
                /* TODO: Full method translation with self parameter */
                translator_write_indent(translator);
                fprintf(translator->output, "/* Method: %s */\n", trimmed);
            }
            
            body_line = strtok_r(NULL, "\n", &saveptr);
        }
        
        free(body);
    }
    
    translator->indent_level--;
    translator_write_indent(translator);
    fprintf(translator->output, "} %s;\n", class_name);
    fprintf(translator->output, "\n");
}

/* Translate interface definition */
void translator_translate_interface(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    const char* line = node->value;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Check for interface definition */
    if (!translator_is_interface_definition(line)) {
        /* Not an interface, copy as-is */
        translator_write_indent(translator);
        fprintf(translator->output, "%s\n", line);
        return;
    }
    
    /* Extract interface name */
    char interface_name[64] = {0};
    const char* p = line + 10;  /* Skip "interface " */
    while (*p == ' ' || *p == '\t') p++;
    
    int j = 0;
    while (*p && *p != ' ' && *p != '\t' && *p != '{' && j < 63) {
        interface_name[j++] = *p++;
    }
    interface_name[j] = '\0';
    
    /* Output vtable struct */
    translator_write_indent(translator);
    fprintf(translator->output, "/* Interface vtable: %s */\n", interface_name);
    translator_write_indent(translator);
    fprintf(translator->output, "typedef struct {\n");
    translator->indent_level++;
    
    /* TODO: Parse interface body and generate vtable entries */
    translator_write_indent(translator);
    fprintf(translator->output, "/* vtable entries */\n");
    
    translator->indent_level--;
    translator_write_indent(translator);
    fprintf(translator->output, "} %s_vtable;\n", interface_name);
    fprintf(translator->output, "\n");
}

/* Translate statement */
void translator_translate_statement(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    const char* line = node->value;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Check for string declaration */
    if (translator_is_string_declaration(line)) {
        translator_translate_string_decl(translator, line);
        return;
    }
    
    /* Check for array declaration */
    if (translator_is_array_declaration(line)) {
        translator_translate_array_decl(translator, line);
        return;
    }
    
    /* Check for method call */
    if (translator_is_method_call(line)) {
        char obj_name[64] = {0};
        char method_call[256] = {0};
        char c_call[512] = {0};
        
        if (translator_extract_method_call(line, obj_name, sizeof(obj_name), method_call, sizeof(method_call))) {
            translator_convert_method_call(obj_name, method_call, c_call, sizeof(c_call));
            translator_write_indent(translator);
            fprintf(translator->output, "%s;\n", c_call);
            return;
        }
    }
    
    /* Check for function call */
    if (translator_is_function_call(line)) {
        /* Function calls in Hust and C have the same syntax */
        /* Just copy as-is, but ensure semicolon */
        translator_write_indent(translator);
        fprintf(translator->output, "%s", line);
        if (!strchr(line, ';')) {
            fprintf(translator->output, ";");
        }
        fprintf(translator->output, "\n");
        return;
    }
    
    /* Default: copy as-is */
    translator_write_indent(translator);
    fprintf(translator->output, "%s\n", line);
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
