/*
 * hust_parser.h - Parser for Hust-C
 * 
 * Builds AST from preprocessed Hust source code
 */

#ifndef HUST_PARSER_H
#define HUST_PARSER_H

#include "hust_ast.h"
#include "hust_preprocessor.h"

/* Parser structure */
typedef struct {
    Preprocessor* preprocessor;
    ASTNode* ast;
    int current_line;
} Parser;

/* Function declarations */
Parser* parser_new(Preprocessor* pp);
void parser_free(Parser* parser);
int parser_parse(Parser* parser);
ASTNode* parser_get_ast(Parser* parser);
void parser_print_ast(Parser* parser);

/* V2 API - replaces old parser_determine_node_type */
NodeType parser_detect_node_type_v2(const char* line);
ASTNode* parser_parse_program_v2(Parser* parser);
int parser_parse_v2(Parser* parser);

/* Internal parsing functions */
ASTNode* parser_parse_function(Parser* parser, const char* line);
ASTNode* parser_parse_variable(Parser* parser, const char* line);
ASTNode* parser_parse_statement(Parser* parser, const char* line);

#endif /* HUST_PARSER_H */
