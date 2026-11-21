/*
 * Hust-C Parser
 * Parses preprocessed Hust source code into an AST
 */

#ifndef PARSER_H
#define PARSER_H

#include "preprocessor.h"
#include <stdbool.h>

/* AST Node types */
typedef enum {
    NODE_PROGRAM,
    NODE_FUNCTION_DEF,
    NODE_CLASS_DEF,
    NODE_VARIABLE_DECL,
    NODE_STATEMENT,
    NODE_EXPRESSION,
    NODE_CONTROL_FLOW,
    NODE_UNKNOWN
} NodeType;

/* AST Node structure */
typedef struct ASTNode {
    NodeType type;
    char* value;
    struct ASTNode** children;
    int child_count;
    int line_number;
} ASTNode;

/* Parser structure */
typedef struct {
    PreprocessedSource* source;
    int current_line;
    ASTNode* root;
} Parser;

/* Function prototypes */
Parser* parser_new(PreprocessedSource* source);
void parser_free(Parser* parser);
ASTNode* parser_parse(Parser* parser);
ASTNode* ast_node_new(NodeType type, const char* value, int line);
void ast_node_free(ASTNode* node);
void ast_print(ASTNode* node, int depth);

#endif /* PARSER_H */
