/*
 * hust_parser.c - Parser implementation for Hust-C
 * 
 * Builds AST from preprocessed Hust source code
 * V2: Simpler, more robust node type detection
 */

#include "../include/hust_parser.h"
#include <ctype.h>
#include <string.h>

/* ============================================
 * Basic Parser Functions
 * ============================================ */

/* Create a new parser */
Parser* parser_new(Preprocessor* pp) {
    Parser* parser = (Parser*)malloc(sizeof(Parser));
    if (!parser) return NULL;
    
    parser->preprocessor = pp;
    parser->ast = NULL;
    parser->current_line = 0;
    
    return parser;
}

/* Free parser */
void parser_free(Parser* parser) {
    if (!parser) return;
    
    /* Free AST if owned */
    if (parser->ast) {
        ast_free(parser->ast);
    }
    
    free(parser);
}

/* Get the AST root node */
ASTNode* parser_get_ast(Parser* parser) {
    if (!parser) return NULL;
    return parser->ast;
}

/* Print AST for debugging */
void parser_print_ast(Parser* parser) {
    if (!parser || !parser->ast) {
        printf("No AST to print\n");
        return;
    }
    
    printf("=== AST Structure ===\n");
    ast_print(parser->ast, 0);
}

/* Parse a function definition from a line */
ASTNode* parser_parse_function(Parser* parser, const char* line) {
    if (!parser || !line) return NULL;
    
    /* Create function node */
    ASTNode* func = ast_create_node(NODE_FUNCTION_DEF, line);
    if (!func) return NULL;
    
    /* TODO: Parse function signature properly */
    /* For now, just store the whole line */
    
    return func;
}

/* Parse a variable declaration from a line */
ASTNode* parser_parse_variable(Parser* parser, const char* line) {
    if (!parser || !line) return NULL;
    
    /* Create variable node */
    ASTNode* var = ast_create_node(NODE_VARIABLE_DECL, line);
    if (!var) return NULL;
    
    /* TODO: Parse variable declaration properly */
    /* For now, just store the whole line */
    
    return var;
}

/* Parse a generic statement from a line */
ASTNode* parser_parse_statement(Parser* parser, const char* line) {
    if (!parser || !line) return NULL;
    
    /* Create statement node */
    ASTNode* stmt = ast_create_node(NODE_STATEMENT, line);
    if (!stmt) return NULL;
    
    return stmt;
}

/* ============================================
 * V2 Node Type Detection - Refactored
 * ============================================ */

/* Helper: Check if line starts with a keyword (followed by space or paren) */
static bool starts_with_keyword(const char* line, const char* keyword) {
    size_t len = strlen(keyword);
    if (strncmp(line, keyword, len) != 0) return false;
    
    /* Check next char is space, tab, '(', '[', or '\0' */
    char next = line[len];
    return (next == ' ' || next == '\t' || next == '(' || next == '[' || next == '\0');
}

/* Helper: Check if line is a variable declaration type */
static bool is_var_type(const char* line) {
    return (starts_with_keyword(line, "i32") ||
            starts_with_keyword(line, "i64") ||
            starts_with_keyword(line, "f32") ||
            starts_with_keyword(line, "f64") ||
            starts_with_keyword(line, "bool") ||
            starts_with_keyword(line, "char") ||
            starts_with_keyword(line, "String") ||
            starts_with_keyword(line, "void"));
}

/* Helper: Skip leading whitespace and 'public' prefix */
static const char* parser_skip_prefix(const char* line) {
    if (!line) return NULL;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Skip 'public ' prefix */
    if (strncmp(line, "public ", 7) == 0) {
        line += 7;
        /* Skip any additional whitespace after public */
        while (*line == ' ' || *line == '\t') line++;
    }
    
    return line;
}

/* Helper: Check if line is a block end (closing brace) */
static bool parser_is_block_end(const char* line) {
    if (!line) return false;
    
    /* Must start with } */
    if (line[0] != '}') return false;
    
    /* Check if it's followed by else (} else {) - not a block end */
    const char* rest = line + 1;
    while (*rest == ' ' || *rest == '\t') rest++;
    if (strncmp(rest, "else", 4) == 0) {
        return false;  /* This is } else {, not a block end */
    }
    
    /* Check if it's a pure block end: }, }, ;, or end of line */
    return (line[1] == '\0' || line[1] == ' ' || line[1] == '\t' || line[1] == ';');
}

/* Helper: Check if line is a control flow statement */
static bool parser_is_control_flow(const char* line) {
    if (!line) return false;
    return (starts_with_keyword(line, "if") ||
            starts_with_keyword(line, "while") ||
            starts_with_keyword(line, "for") ||
            starts_with_keyword(line, "else") ||
            starts_with_keyword(line, "match"));
}

/* Helper: Check if line is a return statement */
static bool parser_is_return(const char* line) {
    if (!line) return false;
    return starts_with_keyword(line, "return");
}

/* Helper: Check if line contains balanced parentheses */
static bool parser_has_balanced_parens(const char* line) {
    if (!line) return false;
    
    int paren_count = 0;
    bool has_paren = false;
    const char* p = line;
    
    while (*p && *p != '{') {
        if (*p == '(') {
            has_paren = true;
            paren_count++;
        } else if (*p == ')') {
            paren_count--;
        }
        p++;
    }
    
    return has_paren && paren_count == 0;
}

/* Helper: Check if line is a function definition */
static bool parser_is_function_def(const char* line) {
    if (!line) return false;
    if (!is_var_type(line)) return false;
    return parser_has_balanced_parens(line);
}

/* Helper: Check if line is a variable declaration */
static bool parser_is_variable_decl(const char* line) {
    if (!line) return false;
    if (!is_var_type(line)) return false;
    return !parser_has_balanced_parens(line);
}

/* Helper: Check if line is a class definition */
static bool parser_is_class_definition(const char* line) {
    if (!line) return false;
    return (strncmp(line, "class ", 6) == 0);
}

/* Helper: Check if line is an interface definition */
static bool parser_is_interface_definition(const char* line) {
    if (!line) return false;
    return (strncmp(line, "interface ", 10) == 0);
}

/* Helper: Check if line is an enum definition */
static bool parser_is_enum_definition(const char* line) {
    if (!line) return false;
    return (strncmp(line, "enum ", 5) == 0);
}

/* Helper: Check if line is a method definition (inside class) */
static bool parser_is_method_definition(const char* line) {
    if (!line) return false;
    
    /* Method definitions have type + name + (params) pattern */
    const char* p = line;
    
    /* Skip public prefix */
    if (strncmp(p, "public ", 7) == 0) {
        p += 7;
    }
    
    /* Check for return type */
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

/* V2 node type detection - refactored with single responsibility */
NodeType parser_detect_node_type_v2(const char* line) {
    if (!line || line[0] == '\0') return NODE_UNKNOWN;
    
    /* Skip prefix (whitespace + public) */
    const char* stripped = parser_skip_prefix(line);
    if (!stripped) return NODE_UNKNOWN;
    
    /* Check for block end */
    if (parser_is_block_end(stripped)) {
        return NODE_BLOCK;
    }
    
    /* Check for else (must be before control flow) */
    if (starts_with_keyword(stripped, "else")) {
        return NODE_CONTROL_FLOW;
    }
    
    /* Check for class definition */
    if (parser_is_class_definition(stripped)) {
        return NODE_CLASS_DEF;
    }
    
    /* Check for enum definition */
    if (parser_is_enum_definition(stripped)) {
        return NODE_ENUM_DEF;
    }
    
    /* Check for interface definition */
    if (parser_is_interface_definition(stripped)) {
        return NODE_CLASS_DEF;  /* Use CLASS_DEF for interface too */
    }
    
    /* Check for control flow */
    if (parser_is_control_flow(stripped)) {
        return NODE_CONTROL_FLOW;
    }
    
    /* Check for match case (contains : and looks like a case pattern) */
    if (strchr(stripped, ':') && !strchr(stripped, '=')) {
        /* Check if it's a match case (not a regular statement with :) */
        const char* colon = strchr(stripped, ':');
        if (colon && colon != stripped) {
            /* Check if line starts with a case pattern (not a type keyword) */
            if (strncmp(stripped, "i32 ", 4) != 0 &&
                strncmp(stripped, "i64 ", 4) != 0 &&
                strncmp(stripped, "f32 ", 4) != 0 &&
                strncmp(stripped, "f64 ", 4) != 0 &&
                strncmp(stripped, "bool ", 5) != 0 &&
                strncmp(stripped, "char ", 5) != 0 &&
                strncmp(stripped, "String ", 7) != 0 &&
                strncmp(stripped, "void ", 5) != 0 &&
                strncmp(stripped, "return ", 7) != 0) {
                return NODE_MATCH_CASE;
            }
        }
    }
    
    /* Check for return */
    if (parser_is_return(stripped)) {
        return NODE_RETURN;
    }
    
    /* Check for function definition */
    if (parser_is_function_def(stripped)) {
        return NODE_FUNCTION_DEF;
    }
    
    /* Check for variable declaration */
    if (parser_is_variable_decl(stripped)) {
        return NODE_VARIABLE_DECL;
    }
    
    /* Check for method call (contains . but not declaration) */
    if (strchr(stripped, '.') && strchr(stripped, '(') && !strchr(stripped, '=')) {
        return NODE_STATEMENT;
    }
    
    /* Default: generic statement */
    return NODE_STATEMENT;
}

/* ============================================
 * V2 Parse Functions
 * ============================================ */

/* V2 parse program function */
ASTNode* parser_parse_program_v2(Parser* parser) {
    if (!parser || !parser->preprocessor) return NULL;
    
    /* Create program root node */
    ASTNode* program = ast_create_node(NODE_PROGRAM, NULL);
    if (!program) return NULL;
    
    /* Parse each line */
    int i = 0;
    while (i < parser->preprocessor->line_count) {
        const char* line = parser->preprocessor->lines[i];
        NodeType type = parser_detect_node_type_v2(line);
        
        /* Check if this is a class definition */
        if (type == NODE_CLASS_DEF) {
            /* Parse class body (may span multiple lines) */
            ASTNode* class_node = ast_create_node(NODE_CLASS_DEF, line);
            if (class_node) {
                /* Collect all lines until closing brace */
                int brace_count = 0;
                bool first_line = true;
                
                while (i < parser->preprocessor->line_count) {
                    const char* class_line = parser->preprocessor->lines[i];
                    
                    /* Count braces */
                    for (const char* p = class_line; *p; p++) {
                        if (*p == '{') brace_count++;
                        else if (*p == '}') brace_count--;
                    }
                    
                    /* Add line to class node value (concatenate) */
                    if (!first_line) {
                        /* Append to existing value */
                        size_t old_len = class_node->value ? strlen(class_node->value) : 0;
                        size_t new_len = strlen(class_line);
                        char* new_value = realloc(class_node->value, old_len + new_len + 2);
                        if (new_value) {
                            strcat(new_value, "\n");
                            strcat(new_value, class_line);
                            class_node->value = new_value;
                        }
                    }
                    
                    first_line = false;
                    i++;
                    
                    /* Check if class body is complete */
                    if (brace_count <= 0 && !first_line) {
                        break;
                    }
                }
                
                ast_add_child(program, class_node);
            }
        } else if (type == NODE_ENUM_DEF) {
            /* Parse enum body (may span multiple lines) */
            ASTNode* enum_node = ast_create_node(NODE_ENUM_DEF, line);
            if (enum_node) {
                /* Collect all lines until closing brace */
                int brace_count = 0;
                bool first_line = true;
                
                while (i < parser->preprocessor->line_count) {
                    const char* enum_line = parser->preprocessor->lines[i];
                    
                    /* Count braces */
                    for (const char* p = enum_line; *p; p++) {
                        if (*p == '{') brace_count++;
                        else if (*p == '}') brace_count--;
                    }
                    
                    /* Add line to enum node value (concatenate) */
                    if (!first_line) {
                        /* Append to existing value */
                        size_t old_len = enum_node->value ? strlen(enum_node->value) : 0;
                        size_t new_len = strlen(enum_line);
                        char* new_value = realloc(enum_node->value, old_len + new_len + 2);
                        if (new_value) {
                            strcat(new_value, "\n");
                            strcat(new_value, enum_line);
                            enum_node->value = new_value;
                        }
                    }
                    
                    first_line = false;
                    i++;
                    
                    /* Check if enum body is complete */
                    if (brace_count <= 0 && !first_line) {
                        break;
                    }
                }
                
                ast_add_child(program, enum_node);
            }
        } else {
            /* Regular line */
            ASTNode* node = ast_create_node(type, line);
            if (node) {
                ast_add_child(program, node);
            }
            i++;
        }
    }
    
    return program;
}

/* V2 main parse function */
int parser_parse_v2(Parser* parser) {
    if (!parser || !parser->preprocessor) return -1;
    
    parser->ast = parser_parse_program_v2(parser);
    if (!parser->ast) {
        fprintf(stderr, "Error: Failed to parse program\n");
        return -1;
    }
    
    return 0;
}

/* ============================================
 * Main Parse Function (uses V2)
 * ============================================ */

/* Main parse function - uses V2 implementation */
int parser_parse(Parser* parser) {
    return parser_parse_v2(parser);
}
