/*
 * Hust-C Transpiler
 * Main entry point
 * 
 * This is a C implementation of the Hust transpiler for the C core.
 * It works independently without requiring the Rust core.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "src/preprocessor.h"
#include "src/parser.h"
#include "src/translator.h"

#define VERSION "0.1.0"

void print_usage(const char* prog_name) {
    fprintf(stderr, "Hust-C Transpiler v%s\n", VERSION);
    fprintf(stderr, "Usage: %s <input.hust> [-o <output.c>]\n", prog_name);
    fprintf(stderr, "\n");
    fprintf(stderr, "Options:\n");
    fprintf(stderr, "  -o <file>    Output file (default: stdout)\n");
    fprintf(stderr, "  -h, --help   Show this help\n");
    fprintf(stderr, "  -v, --version Show version\n");
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        print_usage(argv[0]);
        return 1;
    }
    
    const char* input_file = NULL;
    const char* output_file = NULL;
    
    /* Parse arguments */
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-o") == 0 && i + 1 < argc) {
            output_file = argv[i + 1];
            i++;
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printf("Hust-C Transpiler v%s\n", VERSION);
            printf("Transpiles Hust source code to C code\n");
            return 0;
        } else if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--version") == 0) {
            printf("hust_c v%s\n", VERSION);
            return 0;
        } else {
            input_file = argv[i];
        }
    }
    
    if (!input_file) {
        fprintf(stderr, "Error: No input file specified\n");
        return 1;
    }
    
    /* Step 1: Preprocess - clean the source */
    PreprocessedSource* source = preprocessor_new();
    if (!source) {
        fprintf(stderr, "Error: Failed to create preprocessor\n");
        return 1;
    }
    
    if (preprocessor_process_file(source, input_file) < 0) {
        fprintf(stderr, "Error: Failed to preprocess file '%s'\n", input_file);
        preprocessor_free(source);
        return 1;
    }
    
    /* Debug: print preprocessed source */
    /* preprocessor_print(source); */
    
    /* Step 2: Parse - build AST */
    Parser* parser = parser_new(source);
    if (!parser) {
        fprintf(stderr, "Error: Failed to create parser\n");
        preprocessor_free(source);
        return 1;
    }
    
    ASTNode* ast = parser_parse(parser);
    if (!ast) {
        fprintf(stderr, "Error: Failed to parse source\n");
        parser_free(parser);
        preprocessor_free(source);
        return 1;
    }
    
    /* Debug: print AST */
    /* ast_print(ast, 0); */
    
    /* Step 3: Translate - generate C code */
    FILE* output = stdout;
    if (output_file) {
        output = fopen(output_file, "w");
        if (!output) {
            fprintf(stderr, "Error: Cannot open output file '%s'\n", output_file);
            parser_free(parser);
            preprocessor_free(source);
            return 1;
        }
    }
    
    Translator* translator = translator_new(ast, output);
    if (!translator) {
        fprintf(stderr, "Error: Failed to create translator\n");
        if (output_file) fclose(output);
        parser_free(parser);
        preprocessor_free(source);
        return 1;
    }
    
    if (translator_translate(translator) < 0) {
        fprintf(stderr, "Error: Failed to translate\n");
        translator_free(translator);
        if (output_file) fclose(output);
        parser_free(parser);
        preprocessor_free(source);
        return 1;
    }
    
    /* Cleanup */
    translator_free(translator);
    if (output_file) {
        fclose(output);
        printf("Transpiled: %s -> %s\n", input_file, output_file);
    }
    
    parser_free(parser);
    preprocessor_free(source);
    
    return 0;
}
