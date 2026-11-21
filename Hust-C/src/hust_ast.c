/*
 * hust_ast.c - AST Node implementation for Hust-C
 * 
 * Pure C implementation - no Rust dependencies
 */

#include "../include/hust_ast.h"

/* Create a new AST node */
ASTNode* ast_create_node(NodeType type, const char* value) {
    ASTNode* node = (ASTNode*)malloc(sizeof(ASTNode));
    if (!node) return NULL;
    
    node->type = type;
    node->value = value ? strdup(value) : NULL;
    node->children = NULL;
    node->child_count = 0;
    node->child_capacity = 0;
    node->line = 0;
    
    return node;
}

/* Add a child node to a parent */
void ast_add_child(ASTNode* parent, ASTNode* child) {
    if (!parent || !child) return;
    
    /* Expand children array if needed */
    if (parent->child_count >= parent->child_capacity) {
        int new_capacity = parent->child_capacity == 0 ? 4 : parent->child_capacity * 2;
        ASTNode** new_children = (ASTNode**)realloc(parent->children, new_capacity * sizeof(ASTNode*));
        if (!new_children) return;
        
        parent->children = new_children;
        parent->child_capacity = new_capacity;
    }
    
    parent->children[parent->child_count++] = child;
}

/* Free an AST node and all its children */
void ast_free(ASTNode* node) {
    if (!node) return;
    
    /* Free children recursively */
    for (int i = 0; i < node->child_count; i++) {
        ast_free(node->children[i]);
    }
    
    /* Free children array */
    free(node->children);
    
    /* Free value string */
    free(node->value);
    
    /* Free node itself */
    free(node);
}

/* Print AST for debugging */
void ast_print(ASTNode* node, int depth) {
    if (!node) return;
    
    /* Print indentation */
    for (int i = 0; i < depth; i++) {
        printf("  ");
    }
    
    /* Print node info */
    printf("Node(type=%d", node->type);
    if (node->value) {
        printf(", value='%s'", node->value);
    }
    printf(")\n");
    
    /* Print children */
    for (int i = 0; i < node->child_count; i++) {
        ast_print(node->children[i], depth + 1);
    }
}
