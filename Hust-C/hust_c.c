/*
 * Hust-C Transpiler
 * Transpiles Hust source code to C code
 * 
 * This is a C implementation of the Hust transpiler for the C core.
 * It works independently without requiring the Rust core.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <stdint.h>

#define VERSION "0.1.0"
#define MAX_LINE_LENGTH 1024
#define MAX_OUTPUT_LENGTH 65536

/* Type mapping structure */
typedef struct {
    const char* hust_type;
    const char* c_type;
    const char* header;
} TypeMapping;

/* Type mappings from Hust to C */
static const TypeMapping type_mappings[] = {
    {"i8", "int8_t", "stdint.h"},
    {"i16", "int16_t", "stdint.h"},
    {"i32", "int32_t", "stdint.h"},
    {"i64", "int64_t", "stdint.h"},
    {"u8", "uint8_t", "stdint.h"},
    {"u16", "uint16_t", "stdint.h"},
    {"u32", "uint32_t", "stdint.h"},
    {"u64", "uint64_t", "stdint.h"},
    {"f32", "float", NULL},
    {"f64", "double", NULL},
    {"bool", "bool", "stdbool.h"},
    {"char", "char", NULL},
    {"String", "HustString*", "hust_string.h"},
    {"void", "void", NULL},
    {NULL, NULL, NULL}
};

/* Function prototypes */
const char* get_c_type(const char* hust_type);
void generate_c_code(const char* input_file, const char* output_file);
void process_line(const char* line, FILE* output);
bool is_variable_declaration(const char* line);
void convert_variable_declaration(const char* line, FILE* output);
void convert_function_definition(const char* line, FILE* output);
void convert_class_definition(const char* line, FILE* output);
void convert_control_flow(const char* line, FILE* output);
void convert_printf_call(const char* line, FILE* output);

int main(int argc, char* argv[]) {
    if (argc < 2) {
        fprintf(stderr, "Hust-C Transpiler v%s\n", VERSION);
        fprintf(stderr, "Usage: %s <input.hust> [-o <output.c>]\n", argv[0]);
        fprintf(stderr, "\n");
        fprintf(stderr, "Options:\n");
        fprintf(stderr, "  -o <file>    Output file (default: stdout)\n");
        fprintf(stderr, "  -h, --help   Show this help\n");
        fprintf(stderr, "  -v, --version Show version\n");
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
            printf("hust-c v%s\n", VERSION);
            return 0;
        } else {
            input_file = argv[i];
        }
    }
    
    if (!input_file) {
        fprintf(stderr, "Error: No input file specified\n");
        return 1;
    }
    
    /* Generate C code */
    generate_c_code(input_file, output_file);
    
    return 0;
}

/* Get C type for Hust type */
const char* get_c_type(const char* hust_type) {
    for (int i = 0; type_mappings[i].hust_type != NULL; i++) {
        if (strcmp(type_mappings[i].hust_type, hust_type) == 0) {
            return type_mappings[i].c_type;
        }
    }
    return hust_type; /* Return as-is if not found */
}

/* Generate C code from Hust file */
void generate_c_code(const char* input_file, const char* output_file) {
    FILE* input = fopen(input_file, "r");
    if (!input) {
        fprintf(stderr, "Error: Cannot open input file '%s'\n", input_file);
        exit(1);
    }
    
    FILE* output = stdout;
    if (output_file) {
        output = fopen(output_file, "w");
        if (!output) {
            fprintf(stderr, "Error: Cannot open output file '%s'\n", output_file);
            fclose(input);
            exit(1);
        }
    }
    
    /* Write C header */
    fprintf(output, "#include <stdbool.h>\n");
    fprintf(output, "#include <stdint.h>\n");
    fprintf(output, "#include <stdio.h>\n");
    fprintf(output, "#include <stdlib.h>\n");
    fprintf(output, "#include <string.h>\n");
    fprintf(output, "\n");
    
    /* Process each line */
    char line[MAX_LINE_LENGTH];
    bool last_was_declaration = false;
    while (fgets(line, sizeof(line), input)) {
        /* Skip empty lines and comments */
        char* trimmed = line;
        while (*trimmed == ' ' || *trimmed == '\t') trimmed++;
        
        if (*trimmed == '\0' || *trimmed == '\n' || *trimmed == '/' || *trimmed == '!') {
            continue;
        }
        
        /* Skip empty lines after declarations */
        if (last_was_declaration && *trimmed == '\n') {
            continue;
        }
        
        /* Process the line */
        process_line(trimmed, output);
        
        /* Track if this was a declaration */
        last_was_declaration = is_variable_declaration(trimmed);
    }
    
    /* Cleanup */
    fclose(input);
    if (output_file) {
        fclose(output);
        printf("Transpiled: %s -> %s\n", input_file, output_file);
    }
}

/* Process a single line of Hust code */
void process_line(const char* line, FILE* output) {
    /* Check for variable declaration */
    if (is_variable_declaration(line)) {
        convert_variable_declaration(line, output);
        return;
    }
    
    /* Fix main function signature */
    if (strncmp(line, "void main()", 11) == 0) {
        fprintf(output, "void main(void) {\n");
        return;
    }
    
    /* Check for function definition */
    if (strstr(line, "(") && strstr(line, ")") && strstr(line, "{")) {
        convert_function_definition(line, output);
        return;
    }
    
    /* Check for class definition */
    if (strncmp(line, "class ", 6) == 0) {
        convert_class_definition(line, output);
        return;
    }
    
    /* Check for control flow */
    if (strncmp(line, "if ", 3) == 0 || strncmp(line, "while ", 6) == 0 || 
        strncmp(line, "for ", 4) == 0) {
        convert_control_flow(line, output);
        return;
    }
    
    /* Check for printf */
    if (strncmp(line, "printf(", 7) == 0) {
        convert_printf_call(line, output);
        return;
    }
    
    /* Default: copy line as-is */
    fprintf(output, "%s", line);
}

/* Check if line is a variable declaration */
bool is_variable_declaration(const char* line) {
    /* Check for known Hust types at start of line */
    const char* types[] = {"i32", "i64", "f32", "f64", "bool", "char", "String", NULL};
    
    for (int i = 0; types[i] != NULL; i++) {
        size_t len = strlen(types[i]);
        if (strncmp(line, types[i], len) == 0 && 
            (line[len] == ' ' || line[len] == '\t')) {
            return true;
        }
    }
    
    return false;
}

/* Convert Hust variable declaration to C */
void convert_variable_declaration(const char* line, FILE* output) {
    /* Parse: type name = value; or type name; */
    char type[32], rest[256];
    
    /* Extract type */
    int i = 0;
    while (line[i] && line[i] != ' ' && line[i] != '\t' && i < 31) {
        type[i] = line[i];
        i++;
    }
    type[i] = '\0';
    
    /* Skip whitespace */
    while (line[i] == ' ' || line[i] == '\t') i++;
    
    /* Rest of line */
    strcpy(rest, &line[i]);
    
    /* Get C type */
    const char* c_type = get_c_type(type);
    
    /* Output C declaration */
    fprintf(output, "    %s %s", c_type, rest);
    
    /* Ensure semicolon */
    if (!strchr(rest, ';')) {
        fprintf(output, ";");
    }
}

/* Convert Hust function definition to C */
void convert_function_definition(const char* line, FILE* output) {
    /* For now, just copy the line */
    /* TODO: Parse and convert function signature properly */
    fprintf(output, "%s", line);
}

/* Convert Hust class definition to C */
void convert_class_definition(const char* line, FILE* output) {
    /* For now, just copy the line */
    /* TODO: Convert class to struct */
    fprintf(output, "%s", line);
}

/* Convert Hust control flow to C */
void convert_control_flow(const char* line, FILE* output) {
    /* For now, just copy the line */
    /* TODO: Convert if/while/for properly */
    fprintf(output, "%s", line);
}

/* Convert printf call to handle HustString* arguments */
void convert_printf_call(const char* line, FILE* output) {
    /* For now, just copy the line */
    /* TODO: Detect HustString* arguments and wrap with HustString_cstr() */
    fprintf(output, "%s", line);
}
