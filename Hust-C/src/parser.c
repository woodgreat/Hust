/*
 * Hust-C Parser Implementation
 * Parses preprocessed Hust source code into an AST
 */

#define _POSIX_C_SOURCE 200809L

#include "parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Create a new parser */
Parser* parser_new(PreprocessedSource* source) {
    Parser* parser = (Parser*)malloc(sizeof(Parser));
    if (!parser) return NULL;
    
    parser->source = source;
    parser->current_line = 0;
    parser->root = NULL;
    
    return parser;
}

/* Free parser */
void parser_free(Parser* parser) {
    if (!parser) return;
    
    if (parser->root) {
        ast_node_free(parser->root);
    }
    
    free(parser);
}

/* Create a new AST node */
ASTNode* ast_node_new(NodeType type, const char* value, int line) {
    ASTNode* node = (ASTNode*)malloc(sizeof(ASTNode));
    if (!node) return NULL;
    
    node->type = type;
    node->value = value ? strdup(value) : NULL;
    node->children = NULL;
    node->child_count = 0;
    node->line_number = line;
    
    return node;
}

/* Free AST node recursively */
void ast_node_free(ASTNode* node) {
    if (!node) return;
    
    if (node->value) {
        free(node->value);
    }
    
    for (int i = 0; i < node->child_count; i++) {
        ast_node_free(node->children[i]);
    }
    
    free(node->children);
    free(node);
}

/* Add child to AST node */
void ast_node_add_child(ASTNode* parent, ASTNode* child) {
    if (!parent || !child) return;
    
    ASTNode** new_children = (ASTNode**)realloc(
        parent->children,
        (parent->child_count + 1) * sizeof(ASTNode*)
    );
    
    if (!new_children) return;
    
    parent->children = new_children;
    parent->children[parent->child_count] = child;
    parent->child_count++;
}

/* Determine node type from line content */
NodeType get_node_type(const char* line) {
    if (!line) return NODE_UNKNOWN;
    
    /* Check for function definition */
    if (strstr(line, "(") && strstr(line, ")") && strstr(line, "{")) {
        return NODE_FUNCTION_DEF;
    }
    
    /* Check for class definition */
    if (strncmp(line, "class ", 6) == 0) {
        return NODE_CLASS_DEF;
    }
    
    /* Check for variable declaration */
    const char* types[] = {"i32", "i64", "f32", "f64", "bool", "char", "String", NULL};
    for (int i = 0; types[i] != NULL; i++) {
        size_t len = strlen(types[i]);
        if (strncmp(line, types[i], len) == 0 && 
            (line[len] == ' ' || line[len] == '\t')) {
            return NODE_VARIABLE_DECL;
        }
    }
    
    /* Check for control flow */
    if (strncmp(line, "if ", 3) == 0 || strncmp(line, "while ", 6) == 0 || 
        strncmp(line, "for ", 4) == 0) {
        return NODE_CONTROL_FLOW;
    }
    
    /* Default to statement */
    return NODE_STATEMENT;
}

/* Parse the preprocessed source into an AST */
ASTNode* parser_parse(Parser* parser) {
    if (!parser || !parser->source) return NULL;
    
    /* Create root node */
    parser->root = ast_node_new(NODE_PROGRAM, "program", 0);
    if (!parser->root) return NULL;
    
    /* Parse each line */
    for (int i = 0; i < parser->source->line_count; i++) {
        const char* line = parser->source->lines[i];
        NodeType type = get_node_type(line);
        
        ASTNode* node = ast_node_new(type, line, i + 1);
        if (node) {
            ast_node_add_child(parser->root, node);
        }
    }
    
    return parser->root;
}

/* Print AST for debugging */
void ast_print(ASTNode* node, int depth) {
    if (!node) return;
    
    /* Print indentation */
    for (int i = 0; i < depth; i++) {
        printf("  ");
    }
    
    /* Print node info */
    const char* type_names[] = {
        "PROGRAM", "FUNCTION_DEF", "CLASS_DEF", "VARIABLE_DECL",
        "STATEMENT", "EXPRESSION", "CONTROL_FLOW", "UNKNOWN"
    };
    
    printf("[%s]", type_names[node->type]);
    if (node->value) {
        printf(" %s", node->value);
    }
    printf(" (line %d)\n", node->line_number);
    
    /* Print children */
    for (int i = 0; i < node->child_count; i++) {
        ast_print(node->children[i], depth + 1);
    }
}
