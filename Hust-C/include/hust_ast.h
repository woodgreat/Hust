/*
 * hust_ast.h - AST Node definitions for Hust-C
 * 
 * Pure C implementation - no Rust dependencies
 */

#ifndef HUST_AST_H
#define HUST_AST_H

/* Enable POSIX functions like strdup */
#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

/* AST Node types */
typedef enum {
    NODE_PROGRAM,       /* Root node */
    NODE_FUNCTION_DEF,  /* Function definition */
    NODE_VARIABLE_DECL, /* Variable declaration */
    NODE_CONTROL_FLOW,  /* if/while/for */
    NODE_STATEMENT,     /* Generic statement */
    NODE_EXPRESSION,    /* Expression */
    NODE_BLOCK,         /* Code block { } */
    NODE_IDENTIFIER,    /* Identifier */
    NODE_LITERAL,       /* Literal value */
    NODE_BINARY_OP,     /* Binary operation */
    NODE_UNARY_OP,      /* Unary operation */
    NODE_CALL,          /* Function call */
    NODE_RETURN,        /* Return statement */
    NODE_CLASS_DEF,     /* Class definition */
    NODE_METHOD_DEF,    /* Method definition */
    NODE_ENUM_DEF,      /* Enum definition */
    NODE_UNKNOWN        /* Unknown/placeholder */
} NodeType;

/* AST Node structure */
typedef struct ASTNode {
    NodeType type;
    char* value;                    /* Node value (identifier name, literal, etc.) */
    struct ASTNode** children;      /* Child nodes */
    int child_count;
    int child_capacity;
    int line;                       /* Source line number */
} ASTNode;

/* Function declarations */
ASTNode* ast_create_node(NodeType type, const char* value);
void ast_add_child(ASTNode* parent, ASTNode* child);
void ast_free(ASTNode* node);
void ast_print(ASTNode* node, int depth);

/* Helper macros */
#define AST_FOREACH_CHILD(node, child, i) \
    for (i = 0; i < (node)->child_count && ((child) = (node)->children[i]); i++)

#endif /* HUST_AST_H */
