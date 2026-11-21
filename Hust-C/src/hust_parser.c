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
 * V2 Node Type Detection
 * ============================================ */

/* Helper: Check if line starts with a keyword (followed by space or paren) */
static bool starts_with_keyword(const char* line, const char* keyword) {
    size_t len = strlen(keyword);
    if (strncmp(line, keyword, len) != 0) return false;
    
    /* Check next char is space, tab, or '(' */
    char next = line[len];
    return (next == ' ' || next == '\t' || next == '(' || next == '\0');
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

/* V2 node type detection */
NodeType parser_detect_node_type_v2(const char* line) {
    if (!line || line[0] == '\0') return NODE_UNKNOWN;
    
    /* Skip leading whitespace */
    while (*line == ' ' || *line == '\t') line++;
    
    /* Skip 'public ' prefix */
    if (strncmp(line, "public ", 7) == 0) {
        line += 7;
        /* Skip any additional whitespace after public */
        while (*line == ' ' || *line == '\t') line++;
    }
    
    /* Check for closing brace */
    if (line[0] == '}' && (line[1] == '\0' || line[1] == ' ' || line[1] == '\t')) {
        return NODE_BLOCK;
    }
    
    /* Check for control flow */
    if (starts_with_keyword(line, "if") ||
        starts_with_keyword(line, "while") ||
        starts_with_keyword(line, "for")) {
        return NODE_CONTROL_FLOW;
    }
    
    /* Check for return */
    if (starts_with_keyword(line, "return")) {
        return NODE_RETURN;
    }
    
    /* Check for function definition: has type + name + ( */
    if (is_var_type(line)) {
        /* Look for '(' to distinguish function from variable */
        const char* p = line;
        int paren_count = 0;
        bool has_paren = false;
        
        while (*p && *p != '{') {
            if (*p == '(') {
                has_paren = true;
                paren_count++;
            } else if (*p == ')') {
                paren_count--;
            }
            p++;
        }
        
        if (has_paren && paren_count == 0) {
            return NODE_FUNCTION_DEF;
        }
        
        /* No parens - it's a variable declaration */
        if (!has_paren) {
            return NODE_VARIABLE_DECL;
        }
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
    for (int i = 0; i < parser->preprocessor->line_count; i++) {
        const char* line = parser->preprocessor->lines[i];
        NodeType type = parser_detect_node_type_v2(line);
        
        ASTNode* node = ast_create_node(type, line);
        if (node) {
            ast_add_child(program, node);
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
