/*
 * main.c - Main entry point for Hust-C transpiler
 * 
 * Pure C implementation - no Rust dependencies
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "include/hust_ast.h"
#include "include/hust_preprocessor.h"
#include "include/hust_parser.h"
#include "include/hust_translator.h"

/* Print usage information */
void print_usage(const char* program_name) {
    fprintf(stderr, "Usage: %s <input.hust> [-o output.c]\n", program_name);
    fprintf(stderr, "       %s --help\n", program_name);
}

/* Print help information */
void print_help(void) {
    printf("Hust-C Transpiler\n");
    printf("=================\n\n");
    printf("Converts Hust source code to C code.\n\n");
    printf("Usage:\n");
    printf("  hust_c <input.hust> [-o output.c]  Transpile Hust file to C\n");
    printf("  hust_c --help                       Show this help message\n\n");
    printf("Options:\n");
    printf("  -o <file>    Specify output file (default: stdout)\n");
    printf("  --ast        Print AST structure for debugging\n");
    printf("  --pre        Print preprocessed lines for debugging\n");
}

/* Main function */
int main(int argc, char* argv[]) {
    /* Check for help flag */
    if (argc < 2 || strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0) {
        print_help();
        return argc < 2 ? 1 : 0;
    }
    
    /* Parse arguments */
    const char* input_file = NULL;
    const char* output_file = NULL;
    bool print_ast = false;
    bool print_pre = false;
    
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-o") == 0 && i + 1 < argc) {
            output_file = argv[++i];
        } else if (strcmp(argv[i], "--ast") == 0) {
            print_ast = true;
        } else if (strcmp(argv[i], "--pre") == 0) {
            print_pre = true;
        } else if (argv[i][0] != '-') {
            input_file = argv[i];
        }
    }
    
    if (!input_file) {
        fprintf(stderr, "Error: No input file specified\n");
        print_usage(argv[0]);
        return 1;
    }
    
    /* Step 1: Preprocess */
    Preprocessor* pp = preprocessor_new();
    if (!pp) {
        fprintf(stderr, "Error: Failed to create preprocessor\n");
        return 1;
    }
    
    if (preprocessor_process_file(pp, input_file) != 0) {
        fprintf(stderr, "Error: Failed to preprocess file '%s'\n", input_file);
        preprocessor_free(pp);
        return 1;
    }
    
    if (print_pre) {
        preprocessor_print(pp);
    }
    
    /* Step 2: Parse */
    Parser* parser = parser_new(pp);
    if (!parser) {
        fprintf(stderr, "Error: Failed to create parser\n");
        preprocessor_free(pp);
        return 1;
    }
    
    if (parser_parse_v2(parser) != 0) {
        fprintf(stderr, "Error: Failed to parse file '%s'\n", input_file);
        parser_free(parser);
        preprocessor_free(pp);
        return 1;
    }
    
    if (print_ast) {
        parser_print_ast(parser);
    }
    
    /* Step 3: Translate */
    FILE* output = stdout;
    if (output_file) {
        output = fopen(output_file, "w");
        if (!output) {
            fprintf(stderr, "Error: Cannot open output file '%s'\n", output_file);
            parser_free(parser);
            preprocessor_free(pp);
            return 1;
        }
    }
    
    Translator* translator = translator_new(parser_get_ast(parser), output);
    if (!translator) {
        fprintf(stderr, "Error: Failed to create translator\n");
        if (output_file) fclose(output);
        parser_free(parser);
        preprocessor_free(pp);
        return 1;
    }
    
    if (translator_translate(translator) != 0) {
        fprintf(stderr, "Error: Failed to translate file '%s'\n", input_file);
        translator_free(translator);
        if (output_file) fclose(output);
        parser_free(parser);
        preprocessor_free(pp);
        return 1;
    }
    
    /* Cleanup */
    translator_free(translator);
    if (output_file) fclose(output);
    parser_free(parser);
    preprocessor_free(pp);
    
    return 0;
}
