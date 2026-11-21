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
    
    /* Parse method signature: [public] return_type method_name(params) { */
    char return_type[32] = {0};
    char method_name[64] = {0};
    char params[256] = {0};
    int i = 0, j = 0;
    
    /* Skip public prefix */
    if (strncmp(line, "public ", 7) == 0) {
        i = 7;
    }
    
    /* Extract return type */
    while (line[i] && line[i] != ' ' && line[i] != '\t' && j < 31) {
        return_type[j++] = line[i++];
    }
    return_type[j] = '\0';
    
    /* Skip whitespace */
    while (line[i] == ' ' || line[i] == '\t') i++;
    
    /* Extract method name */
    j = 0;
    while (line[i] && line[i] != '(' && line[i] != ' ' && line[i] != '\t' && j < 63) {
        method_name[j++] = line[i++];
    }
    method_name[j] = '\0';
    
    /* Skip ( */
    if (line[i] == '(') i++;
    
    /* Extract params */
    j = 0;
    while (line[i] && line[i] != ')' && j < 255) {
        params[j++] = line[i++];
    }
    params[j] = '\0';
    
    /* Check if this is a constructor (method name == class name) */
    bool is_constructor = (strcmp(method_name, class_name) == 0);
    
    /* Convert return type */
    const char* c_return_type = get_c_type(return_type);
    
    /* Build C function signature */
    translator_write_indent(translator);
    
    if (is_constructor) {
        /* Constructor: ClassName* ClassName_new(params) */
        fprintf(translator->output, "%s* %s_new(%s* self", class_name, class_name, class_name);
        
        /* Add params if any */
        if (strlen(params) > 0) {
            /* Parse and convert params */
            char param_copy[256];
            strncpy(param_copy, params, sizeof(param_copy) - 1);
            param_copy[sizeof(param_copy) - 1] = '\0';
            
            char* token = strtok(param_copy, ",");
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
                
                /* Add param */
                fprintf(translator->output, ", %s %s", c_type, p_name);
                
                token = strtok(NULL, ",");
            }
        }
        
        fprintf(translator->output, ") {\n");
    } else {
        /* Regular method: return_type ClassName_methodName(ClassName* self, params) */
        fprintf(translator->output, "%s %s_%s(%s* self", c_return_type, class_name, method_name, class_name);
        
        /* Add params if any */
        if (strlen(params) > 0) {
            /* Parse and convert params */
            char param_copy[256];
            strncpy(param_copy, params, sizeof(param_copy) - 1);
            param_copy[sizeof(param_copy) - 1] = '\0';
            
            char* token = strtok(param_copy, ",");
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
                
                /* Add param */
                fprintf(translator->output, ", %s %s", c_type, p_name);
                
                token = strtok(NULL, ",");
            }
        }
        
        fprintf(translator->output, ") {\n");
    }
    
    translator->indent_level++;
}

/* Helper: Check if class has extends clause */
static bool translator_class_has_extends(const char* line, char* parent_class, size_t parent_size) {
    if (!line || !parent_class || parent_size == 0) return false;
    
    /* Look for "extends" keyword */
    const char* extends_pos = strstr(line, "extends");
    if (!extends_pos) return false;
    
    /* Skip "extends" and whitespace */
    extends_pos += 7;
    while (*extends_pos == ' ' || *extends_pos == '\t') extends_pos++;
    
    /* Extract parent class name */
    int j = 0;
    while (*extends_pos && *extends_pos != ' ' && *extends_pos != '\t' && *extends_pos != '{' && j < (int)parent_size - 1) {
        parent_class[j++] = *extends_pos++;
    }
    parent_class[j] = '\0';
    
    return (j > 0);
}

/* Helper: Extract class name and parent class from class definition */
static bool translator_extract_class_info(const char* line, char* class_name, size_t class_size, char* parent_class, size_t parent_size) {
    if (!line || !class_name || !parent_class || class_size == 0 || parent_size == 0) return false;
    
    /* Skip "class " */
    const char* p = line + 6;
    while (*p == ' ' || *p == '\t') p++;
    
    /* Extract class name */
    int j = 0;
    while (*p && *p != ' ' && *p != '\t' && *p != '{' && j < (int)class_size - 1) {
        class_name[j++] = *p++;
    }
    class_name[j] = '\0';
    
    /* Check for extends */
    parent_class[0] = '\0';
    translator_class_has_extends(line, parent_class, parent_size);
    
    return (j > 0);
}

/* Check if class implements interface */
static bool translator_class_implements_interface(const char* line, char* interface_name, size_t interface_size) {
    if (!line || !interface_name || interface_size == 0) return false;
    
    const char* impl = strstr(line, "implements");
    if (!impl) return false;
    
    impl += 10;  /* Skip "implements" */
    while (*impl == ' ' || *impl == '\t') impl++;
    
    int j = 0;
    while (*impl && *impl != ' ' && *impl != '\t' && *impl != '{' && j < (int)interface_size - 1) {
        interface_name[j++] = *impl++;
    }
    interface_name[j] = '\0';
    
    return (j > 0);
}

/* Extract class name, parent class, and interface from class definition */
static bool translator_extract_class_full_info(const char* line, 
    char* class_name, size_t class_size,
    char* parent_class, size_t parent_size,
    char* interface_name, size_t interface_size) {
    if (!line || !class_name || !parent_class || !interface_name || 
        class_size == 0 || parent_size == 0 || interface_size == 0) return false;
    
    /* Skip "class " */
    const char* p = line + 6;
    while (*p == ' ' || *p == '\t') p++;
    
    /* Extract class name */
    int j = 0;
    while (*p && *p != ' ' && *p != '\t' && *p != '{' && j < (int)class_size - 1) {
        class_name[j++] = *p++;
    }
    class_name[j] = '\0';
    
    /* Check for extends */
    parent_class[0] = '\0';
    translator_class_has_extends(line, parent_class, parent_size);
    
    /* Check for implements */
    interface_name[0] = '\0';
    translator_class_implements_interface(line, interface_name, interface_size);
    
    return (j > 0);
}

/* Translate class definition with inheritance support */
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
    
    /* Extract class name, parent class, and interface */
    char class_name[64] = {0};
    char parent_class[64] = {0};
    char interface_name[64] = {0};
    if (!translator_extract_class_full_info(line, class_name, sizeof(class_name), 
                                            parent_class, sizeof(parent_class),
                                            interface_name, sizeof(interface_name))) {
        fprintf(stderr, "Error: Failed to extract class name\n");
        return;
    }
    
    /* Output struct definition */
    translator_write_indent(translator);
    fprintf(translator->output, "typedef struct {\n");
    translator->indent_level++;
    
    /* If has parent class, embed parent struct as first member */
    if (strlen(parent_class) > 0) {
        translator_write_indent(translator);
        fprintf(translator->output, "%s %s;  /* Parent class */\n", parent_class, parent_class);
    }
    
    /* If implements interface, add vtable pointer */
    if (strlen(interface_name) > 0) {
        translator_write_indent(translator);
        fprintf(translator->output, "%sVTable* vtable;  /* Interface vtable */\n", interface_name);
    }
    
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
                /* Translate method to C function */
                translator_translate_method(translator, trimmed, class_name);
            }
            
            body_line = strtok_r(NULL, "\n", &saveptr);
        }
        
        free(body);
    }
    
    translator->indent_level--;
    translator_write_indent(translator);
    fprintf(translator->output, "} %s;\n", class_name);
    fprintf(translator->output, "\n");
    
    /* Output constructor with parent initialization */
    if (strlen(parent_class) > 0) {
        translator_write_indent(translator);
        fprintf(translator->output, "/* Constructor with parent initialization */\n");
        translator_write_indent(translator);
        fprintf(translator->output, "%s* %s_new() {\n", class_name, class_name);
        translator->indent_level++;
        translator_write_indent(translator);
        fprintf(translator->output, "%s* self = malloc(sizeof(%s));\n", class_name, class_name);
        translator_write_indent(translator);
        fprintf(translator->output, "/* Initialize parent */\n");
        translator_write_indent(translator);
        fprintf(translator->output, "%s_init(&self->%s);\n", parent_class, parent_class);
        translator_write_indent(translator);
        fprintf(translator->output, "return self;\n");
        translator->indent_level--;
        translator_write_indent(translator);
        fprintf(translator->output, "}\n\n");
    }
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
    
    /* Parse interface body and generate vtable entries */
    char* body = strdup(node->value);
    if (body) {
        char* saveptr = NULL;
        char* body_line = strtok_r(body, "\n", &saveptr);
        bool first_line = true;
        
        while (body_line) {
            /* Skip first line (interface declaration) */
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
            
            /* Parse method signature: return_type method_name(params); */
            char ret_type[32] = {0};
            char method_name[64] = {0};
            char params[256] = {0};
            int i = 0, j = 0;
            
            /* Extract return type */
            while (trimmed[i] && trimmed[i] != ' ' && trimmed[i] != '\t' && j < 31) {
                ret_type[j++] = trimmed[i++];
            }
            ret_type[j] = '\0';
            
            /* Skip whitespace */
            while (trimmed[i] == ' ' || trimmed[i] == '\t') i++;
            
            /* Extract method name */
            j = 0;
            while (trimmed[i] && trimmed[i] != '(' && trimmed[i] != ' ' && trimmed[i] != '\t' && j < 63) {
                method_name[j++] = trimmed[i++];
            }
            method_name[j] = '\0';
            
            /* Skip to parameters */
            while (trimmed[i] && trimmed[i] != '(') i++;
            if (trimmed[i] == '(') i++;
            
            /* Extract parameters */
            j = 0;
            while (trimmed[i] && trimmed[i] != ')' && j < 255) {
                params[j++] = trimmed[i++];
            }
            params[j] = '\0';
            
            /* Get C return type */
            const char* c_ret_type = get_c_type(ret_type);
            
            /* Generate function pointer: return_type (*method_name)(self, params) */
            translator_write_indent(translator);
            if (strlen(params) > 0) {
                fprintf(translator->output, "%s (*%s)(void* self, %s);\n", c_ret_type, method_name, params);
            } else {
                fprintf(translator->output, "%s (*%s)(void* self);\n", c_ret_type, method_name);
            }
            
            body_line = strtok_r(NULL, "\n", &saveptr);
        }
        free(body);
    }
    
    translator->indent_level--;
    translator_write_indent(translator);
    fprintf(translator->output, "} %sVTable;\n", interface_name);
    fprintf(translator->output, "\n");
}

/* Helper: Check if line contains array length method call */
static bool translator_is_array_length_call(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: arr.len() or arr.length() */
    if (strstr(line, ".len()") || strstr(line, ".length()")) {
        return true;
    }
    
    return false;
}

/* Helper: Translate array length call */
static void translator_translate_array_length(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Extract array name from arr.len() or arr.length() */
    char arr_name[64] = {0};
    const char* p = line;
    
    /* Skip leading whitespace */
    while (*p == ' ' || *p == '\t') p++;
    
    /* Extract array name (before .len() or .length()) */
    int j = 0;
    while (*p && *p != '.' && *p != ' ' && *p != '\t' && j < 63) {
        arr_name[j++] = *p++;
    }
    arr_name[j] = '\0';
    
    /* Check if this is an assignment: x = arr.len(); */
    const char* eq = strchr(line, '=');
    if (eq) {
        /* Extract left side */
        char left[64] = {0};
        const char* left_start = line;
        while (*left_start == ' ' || *left_start == '\t') left_start++;
        
        int k = 0;
        while (left_start < eq && *left_start != ' ' && *left_start != '\t' && k < 63) {
            left[k++] = *left_start++;
        }
        left[k] = '\0';
        
        /* Output: sizeof(arr)/sizeof(arr[0]) */
        translator_write_indent(translator);
        fprintf(translator->output, "%s = sizeof(%s)/sizeof(%s[0]);\n", left, arr_name, arr_name);
    } else {
        /* Just output the expression */
        translator_write_indent(translator);
        fprintf(translator->output, "sizeof(%s)/sizeof(%s[0])\n", arr_name, arr_name);
    }
}

/* Helper: Check if line contains array push method call */
static bool translator_is_array_push_call(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: arr.push(value) */
    if (strstr(line, ".push(")) {
        return true;
    }
    
    return false;
}

/* Helper: Translate array push call */
static void translator_translate_array_push(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Extract array name and value from arr.push(value) */
    char arr_name[64] = {0};
    char value[256] = {0};
    const char* p = line;
    
    /* Skip leading whitespace */
    while (*p == ' ' || *p == '\t') p++;
    
    /* Extract array name (before .push) */
    int j = 0;
    while (*p && *p != '.' && *p != ' ' && *p != '\t' && j < 63) {
        arr_name[j++] = *p++;
    }
    arr_name[j] = '\0';
    
    /* Skip .push( */
    const char* push_pos = strstr(line, ".push(");
    if (!push_pos) return;
    
    push_pos += 6;  /* Skip ".push(" */
    
    /* Extract value */
    j = 0;
    while (*push_pos && *push_pos != ')' && j < 255) {
        value[j++] = *push_pos++;
    }
    value[j] = '\0';
    
    /* Trim whitespace from value */
    char* val_trim = value;
    while (*val_trim == ' ' || *val_trim == '\t') val_trim++;
    
    /* For now, output as comment (need HustVec implementation) */
    translator_write_indent(translator);
    fprintf(translator->output, "/* TODO: %s.push(%s) - requires HustVec implementation */\n", arr_name, val_trim);
}

/* Helper: Check if line contains string concatenation */
static bool translator_is_string_concat(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: String var = s1 + s2 or var = s1 + s2 */
    /* Check if line contains + and String type or string variables */
    if (strchr(line, '+') && (strstr(line, "String ") || strstr(line, "= "))) {
        return true;
    }
    
    return false;
}

/* Helper: Translate string concatenation */
static void translator_translate_string_concat(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Parse: String result = s1 + s2 + s3... */
    /* or: result = s1 + s2 */
    
    char result_var[64] = {0};
    
    /* Check if this is a declaration or assignment */
    const char* eq = strchr(line, '=');
    if (!eq) return;
    
    /* Extract result variable */
    const char* var_start = line;
    while (*var_start == ' ' || *var_start == '\t') var_start++;
    
    /* Skip "String " if present */
    if (strncmp(var_start, "String ", 7) == 0) {
        var_start += 7;
    }
    
    int j = 0;
    while (var_start < eq && *var_start != ' ' && *var_start != '\t' && j < 63) {
        result_var[j++] = *var_start++;
    }
    result_var[j] = '\0';
    
    /* Extract operands (after =) */
    const char* op_start = eq + 1;
    while (*op_start == ' ' || *op_start == '\t') op_start++;
    
    /* Parse operands separated by + */
    /* For now, simple approach: split by + and create nested concat calls */
    
    /* Count operands */
    int operand_count = 1;
    for (const char* p = op_start; *p; p++) {
        if (*p == '+') operand_count++;
    }
    
    /* Build concat expression */
    /* For 2 operands: HustString_concat(s1, s2) */
    /* For 3+ operands: HustString_concat(HustString_concat(s1, s2), s3) */
    
    char concat_expr[512] = {0};
    char* expr_ptr = concat_expr;
    size_t remaining = sizeof(concat_expr) - 1;
    
    /* Parse first operand */
    char operand[64] = {0};
    j = 0;
    const char* p = op_start;
    while (*p && *p != '+' && *p != ';' && j < 63) {
        operand[j++] = *p++;
    }
    operand[j] = '\0';
    
    /* Trim whitespace */
    char* op_trim = operand;
    while (*op_trim == ' ' || *op_trim == '\t') op_trim++;
    size_t op_len = strlen(op_trim);
    while (op_len > 0 && (op_trim[op_len-1] == ' ' || op_trim[op_len-1] == '\t')) {
        op_trim[--op_len] = '\0';
    }
    
    /* Check if operand is a string literal */
    if (op_trim[0] == '"') {
        snprintf(expr_ptr, remaining, "HustString_new(%s)", op_trim);
    } else {
        snprintf(expr_ptr, remaining, "%s", op_trim);
    }
    
    expr_ptr += strlen(expr_ptr);
    remaining -= strlen(expr_ptr);
    
    /* Process remaining operands */
    while (*p && *p != ';') {
        if (*p == '+') {
            p++;
            while (*p == ' ' || *p == '\t') p++;
            
            /* Parse next operand */
            j = 0;
            while (*p && *p != '+' && *p != ';' && j < 63) {
                operand[j++] = *p++;
            }
            operand[j] = '\0';
            
            /* Trim whitespace */
            op_trim = operand;
            while (*op_trim == ' ' || *op_trim == '\t') op_trim++;
            op_len = strlen(op_trim);
            while (op_len > 0 && (op_trim[op_len-1] == ' ' || op_trim[op_len-1] == '\t')) {
                op_trim[--op_len] = '\0';
            }
            
            /* Build concat call */
            char temp[512] = {0};
            if (op_trim[0] == '"') {
                snprintf(temp, sizeof(temp), "HustString_concat(%s, HustString_new(%s))", concat_expr, op_trim);
            } else {
                snprintf(temp, sizeof(temp), "HustString_concat(%s, %s)", concat_expr, op_trim);
            }
            strncpy(concat_expr, temp, sizeof(concat_expr) - 1);
            concat_expr[sizeof(concat_expr) - 1] = '\0';
            expr_ptr = concat_expr + strlen(concat_expr);
            remaining = sizeof(concat_expr) - 1 - strlen(concat_expr);
        } else {
            p++;
        }
    }
    
    /* Output declaration/assignment */
    translator_write_indent(translator);
    
    /* Check if this is a new declaration */
    if (strstr(line, "String ") && strstr(line, "String ") < eq) {
        /* New declaration: HustString* result = concat_expr; */
        fprintf(translator->output, "HustString* %s = %s;\n", result_var, concat_expr);
    } else {
        /* Assignment: result = concat_expr; */
        fprintf(translator->output, "%s = %s;\n", result_var, concat_expr);
    }
}

/* Helper: Check if line contains string comparison */
static bool translator_is_string_comparison(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: s1 == s2 or s1 != s2 */
    if ((strstr(line, "==") || strstr(line, "!=")) && !strstr(line, "String ")) {
        return true;
    }
    
    return false;
}

/* Helper: Translate string comparison */
static void translator_translate_string_comparison(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Parse: if (s1 == s2) or bool eq = (s1 == s2) */
    /* Convert to: HustString_equals(s1, s2) */
    
    /* Find == or != */
    const char* op = strstr(line, "==");
    bool is_equal = true;
    if (!op) {
        op = strstr(line, "!=");
        is_equal = false;
    }
    if (!op) return;
    
    /* Extract left operand */
    char left[64] = {0};
    const char* left_start = line;
    while (*left_start == ' ' || *left_start == '\t') left_start++;
    
    /* Skip if( if present */
    if (strncmp(left_start, "if (", 4) == 0) {
        left_start += 4;
    } else if (strncmp(left_start, "if(", 3) == 0) {
        left_start += 3;
    }
    
    int j = 0;
    while (left_start < op && *left_start != ' ' && *left_start != '\t' && *left_start != '(' && j < 63) {
        left[j++] = *left_start++;
    }
    left[j] = '\0';
    
    /* Extract right operand */
    char right[64] = {0};
    const char* right_start = op + 2;
    while (*right_start == ' ' || *right_start == '\t') right_start++;
    
    j = 0;
    while (*right_start && *right_start != ')' && *right_start != ' ' && *right_start != '\t' && *right_start != ';' && *right_start != '{' && j < 63) {
        right[j++] = *right_start++;
    }
    right[j] = '\0';
    
    /* Trim whitespace */
    char* left_trim = left;
    while (*left_trim == ' ' || *left_trim == '\t') left_trim++;
    size_t left_len = strlen(left_trim);
    while (left_len > 0 && (left_trim[left_len-1] == ' ' || left_trim[left_len-1] == '\t')) {
        left_trim[--left_len] = '\0';
    }
    
    char* right_trim = right;
    while (*right_trim == ' ' || *right_trim == '\t') right_trim++;
    size_t right_len = strlen(right_trim);
    while (right_len > 0 && (right_trim[right_len-1] == ' ' || right_trim[right_len-1] == '\t')) {
        right_trim[--right_len] = '\0';
    }
    
    /* Build comparison expression */
    translator_write_indent(translator);
    
    /* Check if this is an if statement */
    if (strstr(line, "if") && strstr(line, "if") < op) {
        /* if statement: if (HustString_equals(s1, s2)) { */
        if (is_equal) {
            fprintf(translator->output, "if (HustString_equals(%s, %s)) {\n", left_trim, right_trim);
        } else {
            fprintf(translator->output, "if (!HustString_equals(%s, %s)) {\n", left_trim, right_trim);
        }
        translator->indent_level++;
    } else {
        /* Assignment: bool eq = HustString_equals(s1, s2); */
        /* Extract result variable if present */
        const char* eq_sign = strchr(line, '=');
        if (eq_sign && eq_sign < op) {
            char result_var[64] = {0};
            const char* var_start = line;
            while (*var_start == ' ' || *var_start == '\t') var_start++;
            
            /* Skip bool or other type */
            while (*var_start && *var_start != ' ' && *var_start != '\t') var_start++;
            while (*var_start == ' ' || *var_start == '\t') var_start++;
            
            j = 0;
            while (var_start < eq_sign && *var_start != ' ' && *var_start != '\t' && j < 63) {
                result_var[j++] = *var_start++;
            }
            result_var[j] = '\0';
            
            if (strlen(result_var) > 0) {
                if (is_equal) {
                    fprintf(translator->output, "bool %s = HustString_equals(%s, %s);\n", result_var, left_trim, right_trim);
                } else {
                    fprintf(translator->output, "bool %s = !HustString_equals(%s, %s);\n", result_var, left_trim, right_trim);
                }
            } else {
                /* Just output the expression */
                if (is_equal) {
                    fprintf(translator->output, "HustString_equals(%s, %s);\n", left_trim, right_trim);
                } else {
                    fprintf(translator->output, "!HustString_equals(%s, %s);\n", left_trim, right_trim);
                }
            }
        } else {
            /* Just output the expression */
            if (is_equal) {
                fprintf(translator->output, "HustString_equals(%s, %s);\n", left_trim, right_trim);
            } else {
                fprintf(translator->output, "!HustString_equals(%s, %s);\n", left_trim, right_trim);
            }
        }
    }
}

/* Helper: Check if line contains string method call */
static bool translator_is_string_method_call(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: str.method() or str.method(args) */
    /* Common string methods: length, to_upper, to_lower, trim, etc. */
    
    const char* dot = strchr(line, '.');
    if (!dot) return false;
    
    /* Check for known string methods */
    if (strstr(line, ".length()") ||
        strstr(line, ".len()") ||
        strstr(line, ".to_upper()") ||
        strstr(line, ".to_lower()") ||
        strstr(line, ".trim()") ||
        strstr(line, ".is_empty()")) {
        return true;
    }
    
    return false;
}

/* Helper: Translate string method call */
static void translator_translate_string_method(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Extract string variable and method name */
    char str_var[64] = {0};
    char method_name[64] = {0};
    
    const char* dot = strchr(line, '.');
    if (!dot) return;
    
    /* Extract string variable (before dot) */
    const char* var_start = line;
    while (*var_start == ' ' || *var_start == '\t') var_start++;
    
    /* Skip if( if present */
    if (strncmp(var_start, "if (", 4) == 0) {
        var_start += 4;
    } else if (strncmp(var_start, "if(", 3) == 0) {
        var_start += 3;
    }
    
    /* Skip type declarations */
    if (strncmp(var_start, "bool ", 5) == 0) {
        var_start += 5;
    } else if (strncmp(var_start, "i32 ", 4) == 0) {
        var_start += 4;
    } else if (strncmp(var_start, "String ", 7) == 0) {
        var_start += 7;
    }
    
    /* Skip assignment if present */
    const char* eq = strchr(var_start, '=');
    if (eq && eq < dot) {
        var_start = eq + 1;
        while (*var_start == ' ' || *var_start == '\t') var_start++;
    }
    
    int j = 0;
    while (var_start < dot && *var_start != ' ' && *var_start != '\t' && j < 63) {
        str_var[j++] = *var_start++;
    }
    str_var[j] = '\0';
    
    /* Trim whitespace */
    char* var_trim = str_var;
    while (*var_trim == ' ' || *var_trim == '\t') var_trim++;
    size_t var_len = strlen(var_trim);
    while (var_len > 0 && (var_trim[var_len-1] == ' ' || var_trim[var_len-1] == '\t')) {
        var_trim[--var_len] = '\0';
    }
    
    /* Extract method name (after dot, before () */
    const char* method_start = dot + 1;
    j = 0;
    while (*method_start && *method_start != '(' && *method_start != ')' && j < 63) {
        method_name[j++] = *method_start++;
    }
    method_name[j] = '\0';
    
    /* Map method name to C function */
    const char* c_function = NULL;
    if (strcmp(method_name, "length") == 0 || strcmp(method_name, "len") == 0) {
        c_function = "HustString_length";
    } else if (strcmp(method_name, "to_upper") == 0) {
        c_function = "HustString_to_upper";
    } else if (strcmp(method_name, "to_lower") == 0) {
        c_function = "HustString_to_lower";
    } else if (strcmp(method_name, "trim") == 0) {
        c_function = "HustString_trim";
    } else if (strcmp(method_name, "is_empty") == 0) {
        c_function = "HustString_is_empty";
    }
    
    if (!c_function) {
        /* Unknown method, output as comment */
        translator_write_indent(translator);
        fprintf(translator->output, "/* TODO: %s.%s() */\n", var_trim, method_name);
        return;
    }
    
    /* Check if this is an if statement */
    if (strstr(line, "if") && strstr(line, "if") < dot) {
        /* if statement: if (HustString_length(s) > 0) { */
        /* For now, just output the function call */
        translator_write_indent(translator);
        fprintf(translator->output, "if (%s(%s)) {\n", c_function, var_trim);
        translator->indent_level++;
    } else {
        /* Assignment or expression */
        /* Check for result variable */
        const char* eq_sign = strchr(line, '=');
        if (eq_sign && eq_sign < dot) {
            /* Extract result variable */
            char result_var[64] = {0};
            const char* var_start2 = line;
            while (*var_start2 == ' ' || *var_start2 == '\t') var_start2++;
            
            /* Skip type */
            while (*var_start2 && *var_start2 != ' ' && *var_start2 != '\t') var_start2++;
            while (*var_start2 == ' ' || *var_start2 == '\t') var_start2++;
            
            j = 0;
            while (var_start2 < eq_sign && *var_start2 != ' ' && *var_start2 != '\t' && j < 63) {
                result_var[j++] = *var_start2++;
            }
            result_var[j] = '\0';
            
            /* Determine result type */
            const char* result_type = "int32_t";  /* Default for length */
            if (strcmp(method_name, "to_upper") == 0 || strcmp(method_name, "to_lower") == 0 || strcmp(method_name, "trim") == 0) {
                result_type = "HustString*";
            } else if (strcmp(method_name, "is_empty") == 0) {
                result_type = "bool";
            }
            
            if (strlen(result_var) > 0) {
                translator_write_indent(translator);
                fprintf(translator->output, "%s %s = %s(%s);\n", result_type, result_var, c_function, var_trim);
            } else {
                translator_write_indent(translator);
                fprintf(translator->output, "%s(%s);\n", c_function, var_trim);
            }
        } else {
            /* Just the expression */
            translator_write_indent(translator);
            fprintf(translator->output, "%s(%s);\n", c_function, var_trim);
        }
    }
}

/* Helper: Convert method call to C function call with proper type detection */
static void translator_convert_method_call_advanced(const char* obj_name, const char* obj_type, const char* method_call, char* c_call, size_t c_call_size) {
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
    
    /* Trim whitespace from args */
    char* args_trim = args;
    while (*args_trim == ' ' || *args_trim == '\t') args_trim++;
    size_t args_len = strlen(args_trim);
    while (args_len > 0 && (args_trim[args_len-1] == ' ' || args_trim[args_len-1] == '\t')) {
        args_trim[--args_len] = '\0';
    }
    
    /* Build C function call: Type_method(obj, args) */
    if (obj_type && strlen(obj_type) > 0) {
        /* Use provided type */
        if (strlen(args_trim) > 0) {
            snprintf(c_call, c_call_size, "%s_%s(%s, %s)", obj_type, method_name, obj_name, args_trim);
        } else {
            snprintf(c_call, c_call_size, "%s_%s(%s)", obj_type, method_name, obj_name);
        }
    } else {
        /* Fallback: use obj_name as type (not ideal but works for simple cases) */
        if (strlen(args_trim) > 0) {
            snprintf(c_call, c_call_size, "%s_%s(%s, %s)", obj_name, method_name, obj_name, args_trim);
        } else {
            snprintf(c_call, c_call_size, "%s_%s(%s)", obj_name, method_name, obj_name);
        }
    }
}

/* Helper: Check if line contains method call with assignment */
static bool translator_is_method_call_assignment(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: var = obj.method(args) or type var = obj.method(args) */
    const char* eq = strchr(line, '=');
    if (!eq) return false;
    
    /* Check if there's a . after = */
    const char* dot = strchr(eq, '.');
    if (!dot) return false;
    
    /* Check if there's a ( after . */
    const char* paren = strchr(dot, '(');
    if (!paren) return false;
    
    return true;
}

/* Helper: Translate method call with assignment */
static void translator_translate_method_call_assignment(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Parse: [type] result = obj.method(args) */
    char result_var[64] = {0};
    char result_type[32] = {0};
    char obj_name[64] = {0};
    char method_call[256] = {0};
    
    const char* eq = strchr(line, '=');
    if (!eq) return;
    
    /* Extract result variable and type (before =) */
    const char* var_start = line;
    while (*var_start == ' ' || *var_start == '\t') var_start++;
    
    /* Check for type declaration */
    if (strncmp(var_start, "i32 ", 4) == 0) {
        strncpy(result_type, "int32_t", sizeof(result_type) - 1);
        var_start += 4;
    } else if (strncmp(var_start, "i64 ", 4) == 0) {
        strncpy(result_type, "int64_t", sizeof(result_type) - 1);
        var_start += 4;
    } else if (strncmp(var_start, "f32 ", 4) == 0) {
        strncpy(result_type, "float", sizeof(result_type) - 1);
        var_start += 4;
    } else if (strncmp(var_start, "f64 ", 4) == 0) {
        strncpy(result_type, "double", sizeof(result_type) - 1);
        var_start += 4;
    } else if (strncmp(var_start, "bool ", 5) == 0) {
        strncpy(result_type, "bool", sizeof(result_type) - 1);
        var_start += 5;
    } else if (strncmp(var_start, "char ", 5) == 0) {
        strncpy(result_type, "char", sizeof(result_type) - 1);
        var_start += 5;
    } else if (strncmp(var_start, "String ", 7) == 0) {
        strncpy(result_type, "HustString*", sizeof(result_type) - 1);
        var_start += 7;
    } else {
        /* No type declaration, just variable */
        result_type[0] = '\0';
    }
    
    /* Extract result variable */
    int j = 0;
    while (var_start < eq && *var_start != ' ' && *var_start != '\t' && j < 63) {
        result_var[j++] = *var_start++;
    }
    result_var[j] = '\0';
    
    /* Trim whitespace */
    char* var_trim = result_var;
    while (*var_trim == ' ' || *var_trim == '\t') var_trim++;
    size_t var_len = strlen(var_trim);
    while (var_len > 0 && (var_trim[var_len-1] == ' ' || var_trim[var_len-1] == '\t')) {
        var_trim[--var_len] = '\0';
    }
    
    /* Extract object and method call (after =) */
    const char* rhs = eq + 1;
    while (*rhs == ' ' || *rhs == '\t') rhs++;
    
    /* Extract object name (before .) */
    const char* dot = strchr(rhs, '.');
    if (!dot) return;
    
    j = 0;
    const char* obj_start = rhs;
    while (obj_start < dot && *obj_start != ' ' && *obj_start != '\t' && j < 63) {
        obj_name[j++] = *obj_start++;
    }
    obj_name[j] = '\0';
    
    /* Trim whitespace */
    char* obj_trim = obj_name;
    while (*obj_trim == ' ' || *obj_trim == '\t') obj_trim++;
    size_t obj_len = strlen(obj_trim);
    while (obj_len > 0 && (obj_trim[obj_len-1] == ' ' || obj_trim[obj_len-1] == '\t')) {
        obj_trim[--obj_len] = '\0';
    }
    
    /* Extract method call (after .) */
    const char* method_start = dot + 1;
    strncpy(method_call, method_start, sizeof(method_call) - 1);
    method_call[sizeof(method_call) - 1] = '\0';
    
    /* Convert method call to C function call */
    char c_call[512] = {0};
    translator_convert_method_call_advanced(obj_trim, NULL, method_call, c_call, sizeof(c_call));
    
    /* Output assignment */
    translator_write_indent(translator);
    if (strlen(result_type) > 0) {
        /* Declaration with type: int32_t result = Type_method(obj, args); */
        fprintf(translator->output, "%s %s = %s;\n", result_type, var_trim, c_call);
    } else {
        /* Just assignment: result = Type_method(obj, args); */
        fprintf(translator->output, "%s = %s;\n", var_trim, c_call);
    }
}

/* Helper: Check if line contains standalone method call (no assignment) */
static bool translator_is_standalone_method_call(const char* line) {
    if (!line) return false;
    
    /* Look for pattern: obj.method(args); (no assignment) */
    if (strchr(line, '=')) return false;  /* Has assignment */
    
    /* Check for obj.method( pattern */
    const char* dot = strchr(line, '.');
    if (!dot) return false;
    
    const char* paren = strchr(dot, '(');
    if (!paren) return false;
    
    /* Check if it ends with ); */
    size_t len = strlen(line);
    if (len >= 2 && line[len-2] == ')' && line[len-1] == ';') {
        return true;
    }
    
    return false;
}

/* Helper: Check if line is an import/use statement */
static bool translator_is_import_statement(const char* line) {
    if (!line) return false;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Check for "import " or "use " */
    if (strncmp(line, "import ", 7) == 0) return true;
    if (strncmp(line, "use ", 4) == 0) return true;
    
    return false;
}

/* Helper: Translate import/use statement to #include */
static void translator_translate_import(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Skip "import " or "use " */
    const char* p = line;
    if (strncmp(p, "import ", 7) == 0) {
        p += 7;
    } else if (strncmp(p, "use ", 4) == 0) {
        p += 4;
    }
    
    while (*p == ' ' || *p == '\t') p++;
    
    /* Extract module name (may contain :: for submodules) */
    char module_name[128] = {0};
    int j = 0;
    while (*p && *p != ';' && *p != ' ' && *p != '\t' && j < 127) {
        /* Convert :: to _ for C compatibility */
        if (*p == ':' && *(p+1) == ':') {
            module_name[j++] = '_';
            p += 2;
        } else {
            module_name[j++] = *p++;
        }
    }
    module_name[j] = '\0';
    
    /* Output #include directive */
    translator_write_indent(translator);
    fprintf(translator->output, "#include \"%s.h\"\n", module_name);
}

/* Helper: Translate standalone method call */
static void translator_translate_standalone_method_call(Translator* translator, const char* line) {
    if (!translator || !line) return;
    
    /* Extract object and method call */
    char obj_name[64] = {0};
    char method_call[256] = {0};
    
    /* Skip leading whitespace */
    const char* p = line;
    while (*p == ' ' || *p == '\t') p++;
    
    /* Extract object name (before .) */
    const char* dot = strchr(p, '.');
    if (!dot) return;
    
    int j = 0;
    const char* obj_start = p;
    while (obj_start < dot && *obj_start != ' ' && *obj_start != '\t' && j < 63) {
        obj_name[j++] = *obj_start++;
    }
    obj_name[j] = '\0';
    
    /* Trim whitespace */
    char* obj_trim = obj_name;
    while (*obj_trim == ' ' || *obj_trim == '\t') obj_trim++;
    size_t obj_len = strlen(obj_trim);
    while (obj_len > 0 && (obj_trim[obj_len-1] == ' ' || obj_trim[obj_len-1] == '\t')) {
        obj_trim[--obj_len] = '\0';
    }
    
    /* Extract method call (after .) */
    const char* method_start = dot + 1;
    strncpy(method_call, method_start, sizeof(method_call) - 1);
    method_call[sizeof(method_call) - 1] = '\0';
    
    /* Remove trailing ; if present */
    size_t method_len = strlen(method_call);
    if (method_len > 0 && method_call[method_len-1] == ';') {
        method_call[method_len-1] = '\0';
    }
    
    /* Convert method call to C function call */
    char c_call[512] = {0};
    translator_convert_method_call_advanced(obj_trim, NULL, method_call, c_call, sizeof(c_call));
    
    /* Output function call */
    translator_write_indent(translator);
    fprintf(translator->output, "%s;\n", c_call);
}

/* Translate statement */
void translator_translate_statement(Translator* translator, ASTNode* node) {
    if (!translator || !node || !node->value) return;
    
    const char* line = node->value;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Check for import/use statement */
    if (translator_is_import_statement(line)) {
        translator_translate_import(translator, line);
        return;
    }
    
    /* Check for method call with assignment */
    if (translator_is_method_call_assignment(line)) {
        translator_translate_method_call_assignment(translator, line);
        return;
    }
    
    /* Check for standalone method call */
    if (translator_is_standalone_method_call(line)) {
        translator_translate_standalone_method_call(translator, line);
        return;
    }
    
    /* Check for string concatenation */
    if (translator_is_string_concat(line)) {
        translator_translate_string_concat(translator, line);
        return;
    }
    
    /* Check for string comparison */
    if (translator_is_string_comparison(line)) {
        translator_translate_string_comparison(translator, line);
        return;
    }
    
    /* Check for string method call */
    if (translator_is_string_method_call(line)) {
        translator_translate_string_method(translator, line);
        return;
    }
    
    /* Check for array length method call */
    if (translator_is_array_length_call(line)) {
        translator_translate_array_length(translator, line);
        return;
    }
    
    /* Check for array push method call */
    if (translator_is_array_push_call(line)) {
        translator_translate_array_push(translator, line);
        return;
    }
    
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
