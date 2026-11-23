/*
 * main.c - Main entry point for Hust-C transpiler
 * 
 * Pure C implementation - no Rust dependencies
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "include/hust_ast.h"
#include "include/hust_preprocessor.h"
#include "include/hust_parser.h"
#include "include/hust_translator.h"

/* Print usage information */
void print_usage(const char* program_name) {
    fprintf(stderr, "Usage: %s <input.hust> [-o output.c] [--project]\n", program_name);
    fprintf(stderr, "       %s --help\n", program_name);
}

/* Print help information */
void print_help(void) {
    printf("Hust-C Transpiler\n");
    printf("=================\n\n");
    printf("Converts Hust source code to C code.\n\n");
    printf("Usage:\n");
    printf("  hust_c <input.hust> [-o output.c]  Transpile Hust file to C\n");
    printf("  hust_c <entry.hust> --project       Transpile entry and user modules\n");
    printf("  hust_c --help                       Show this help message\n\n");
    printf("Options:\n");
    printf("  -o <file>    Specify output file (default: stdout)\n");
    printf("  --ast        Print AST structure for debugging\n");
    printf("  --pre        Print preprocessed lines for debugging\n");
}

#define HUST_MAX_PROJECT_MODULES 64
#define HUST_MAX_MODULE_PATHS 8

typedef struct {
    char name[64];
    char path[512];
    bool loaded;
    bool visiting;
} ProjectModule;

typedef struct {
    ProjectModule modules[HUST_MAX_PROJECT_MODULES];
    int count;
    int order[HUST_MAX_PROJECT_MODULES];
    int order_count;
    char base_dir[512];
    char paths[HUST_MAX_MODULE_PATHS][512];
    int path_count;
} Project;

static void project_trim(char* s) {
    if (!s) return;
    char* p = s;
    while (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n') p++;
    if (p != s) memmove(s, p, strlen(p) + 1);
    size_t len = strlen(s);
    while (len > 0 && (s[len - 1] == ' ' || s[len - 1] == '\t' ||
                       s[len - 1] == '\r' || s[len - 1] == '\n')) {
        s[--len] = '\0';
    }
}

static void project_join(const char* dir, const char* name, char* out, size_t out_size) {
    if (!dir || !name || !out || out_size == 0) return;
    if (name[0] == '/') {
        snprintf(out, out_size, "%s", name);
    } else {
        snprintf(out, out_size, "%s/%s", dir, name);
    }
}

static void project_dirname(const char* path, char* out, size_t out_size) {
    if (!path || !out || out_size == 0) return;
    const char* slash = strrchr(path, '/');
    if (!slash) {
        snprintf(out, out_size, ".");
        return;
    }
    size_t len = (size_t)(slash - path);
    if (len >= out_size) len = out_size - 1;
    memcpy(out, path, len);
    out[len] = '\0';
}

static void project_read_settings(Project* project) {
    if (!project) return;
    char config_path[600] = {0};
    project_join(project->base_dir, "settings.config", config_path, sizeof(config_path));
    FILE* f = fopen(config_path, "r");
    if (!f) return;
    
    char line[512];
    bool in_modules = false;
    while (fgets(line, sizeof(line), f)) {
        project_trim(line);
        if (line[0] == '\0' || line[0] == '#') continue;
        if (line[0] == '[') {
            in_modules = (strncmp(line, "[modules]", 9) == 0);
            continue;
        }
        if (!in_modules || strncmp(line, "paths", 5) != 0) continue;
        char* eq = strchr(line, '=');
        if (!eq) continue;
        char* value = eq + 1;
        char* token = strtok(value, ",");
        while (token && project->path_count < HUST_MAX_MODULE_PATHS) {
            project_trim(token);
            if (*token) {
                char resolved[512] = {0};
                project_join(project->base_dir, token, resolved, sizeof(resolved));
                snprintf(project->paths[project->path_count++], 512, "%s", resolved);
            }
            token = strtok(NULL, ",");
        }
    }
    fclose(f);
}

static int project_find_module(Project* project, const char* name) {
    for (int i = 0; i < project->count; i++) {
        if (strcmp(project->modules[i].name, name) == 0) return i;
    }
    return -1;
}

static bool project_resolve_module(Project* project, const char* name,
                                   char* out, size_t out_size) {
    char candidate[600] = {0};
    project_join(project->base_dir, name, candidate, sizeof(candidate));
    strncat(candidate, ".hust", sizeof(candidate) - strlen(candidate) - 1);
    if (access(candidate, R_OK) == 0) {
        snprintf(out, out_size, "%s", candidate);
        return true;
    }
    for (int i = 0; i < project->path_count; i++) {
        project_join(project->paths[i], name, candidate, sizeof(candidate));
        strncat(candidate, ".hust", sizeof(candidate) - strlen(candidate) - 1);
        if (access(candidate, R_OK) == 0) {
            snprintf(out, out_size, "%s", candidate);
            return true;
        }
    }
    return false;
}

static bool project_load_module(Project* project, const char* name, const char* path) {
    if (!project || !name || !*name) return false;
    int index = project_find_module(project, name);
    if (index >= 0) {
        if (project->modules[index].loaded) return true;
        if (project->modules[index].visiting) {
            fprintf(stderr, "Error: circular module dependency involving '%s'\n", name);
            return false;
        }
    } else {
        if (project->count >= HUST_MAX_PROJECT_MODULES) {
            fprintf(stderr, "Error: too many modules\n");
            return false;
        }
        index = project->count++;
        memset(&project->modules[index], 0, sizeof(project->modules[index]));
        snprintf(project->modules[index].name, sizeof(project->modules[index].name), "%s", name);
    }
    
    if (path && *path) {
        snprintf(project->modules[index].path, sizeof(project->modules[index].path), "%s", path);
    } else if (!project_resolve_module(project, name, project->modules[index].path,
                                       sizeof(project->modules[index].path))) {
        fprintf(stderr, "Error: module not found: %s\n", name);
        return false;
    }
    
    project->modules[index].visiting = true;
    
    Preprocessor* pp = preprocessor_new();
    if (!pp || preprocessor_process_file(pp, project->modules[index].path) != 0) {
        fprintf(stderr, "Error: cannot read module '%s'\n", project->modules[index].path);
        if (pp) preprocessor_free(pp);
        return false;
    }
    
    for (int i = 0; i < pp->line_count; i++) {
        char line[512] = {0};
        snprintf(line, sizeof(line), "%s", pp->lines[i]);
        project_trim(line);
        if (strncmp(line, "use ", 4) != 0) continue;
        char* p = line + 4;
        while (*p == ' ' || *p == '\t') p++;
        if (strncmp(p, "C.", 2) == 0 || strncmp(p, "RUST.", 5) == 0 ||
            strncmp(p, "HUST.", 5) == 0) {
            continue;
        }
        char dep[64] = {0};
        size_t dn = 0;
        while (*p && *p != '.' && *p != ';' && *p != ' ' && *p != '\t' &&
               *p != '*' && dn < sizeof(dep) - 1) {
            dep[dn++] = *p++;
        }
        dep[dn] = '\0';
        if (!dep[0]) continue;
        /* Uppercase names are class imports, not module files. */
        if (dep[0] >= 'A' && dep[0] <= 'Z') continue;
        char dep_path[512] = {0};
        if (!project_resolve_module(project, dep, dep_path, sizeof(dep_path))) {
            fprintf(stderr, "Error: module not found: %s (imported by %s)\n",
                    dep, project->modules[index].name);
            preprocessor_free(pp);
            return false;
        }
        if (!project_load_module(project, dep, dep_path)) {
            preprocessor_free(pp);
            return false;
        }
    }
    preprocessor_free(pp);
    
    project->modules[index].visiting = false;
    project->modules[index].loaded = true;
    project->order[project->order_count++] = index;
    return true;
}

static int transpile_file_to(const char* path, FILE* output) {
    Preprocessor* pp = preprocessor_new();
    if (!pp) return 1;
    if (preprocessor_process_file(pp, path) != 0) {
        fprintf(stderr, "Error: failed to preprocess '%s'\n", path);
        preprocessor_free(pp);
        return 1;
    }
    Parser* parser = parser_new(pp);
    if (!parser) {
        preprocessor_free(pp);
        return 1;
    }
    if (parser_parse_v2(parser) != 0) {
        fprintf(stderr, "Error: failed to parse '%s'\n", path);
        parser_free(parser);
        preprocessor_free(pp);
        return 1;
    }
    Translator* translator = translator_new(parser_get_ast(parser), output);
    if (!translator) {
        parser_free(parser);
        preprocessor_free(pp);
        return 1;
    }
    int rc = translator_translate(translator);
    translator_free(translator);
    parser_free(parser);
    preprocessor_free(pp);
    return rc == 0 ? 0 : 1;
}

static int project_transpile(const char* entry, FILE* output) {
    if (!entry || !output) return 1;
    Project project;
    memset(&project, 0, sizeof(project));
    project_dirname(entry, project.base_dir, sizeof(project.base_dir));
    project_read_settings(&project);
    
    const char* base = strrchr(entry, '/');
    base = base ? base + 1 : entry;
    char name[64] = {0};
    snprintf(name, sizeof(name), "%s", base);
    char* dot = strrchr(name, '.');
    if (dot) *dot = '\0';
    
    if (!project_load_module(&project, name, entry)) return 1;
    for (int i = 0; i < project.order_count; i++) {
        int index = project.order[i];
        if (transpile_file_to(project.modules[index].path, output) != 0) {
            fprintf(stderr, "Error: failed to transpile module '%s'\n",
                    project.modules[index].name);
            return 1;
        }
    }
    return 0;
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
    bool project_mode = false;
    
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-o") == 0 && i + 1 < argc) {
            output_file = argv[++i];
        } else if (strcmp(argv[i], "--ast") == 0) {
            print_ast = true;
        } else if (strcmp(argv[i], "--pre") == 0) {
            print_pre = true;
        } else if (strcmp(argv[i], "--project") == 0) {
            project_mode = true;
        } else if (argv[i][0] != '-') {
            input_file = argv[i];
        }
    }
    
    if (!input_file) {
        fprintf(stderr, "Error: No input file specified\n");
        print_usage(argv[0]);
        return 1;
    }
    
    if (project_mode) {
        FILE* project_output = stdout;
        if (output_file) {
            project_output = fopen(output_file, "w");
            if (!project_output) {
                fprintf(stderr, "Error: Cannot open output file '%s'\n", output_file);
                return 1;
            }
        }
        int rc = project_transpile(input_file, project_output);
        if (output_file) fclose(project_output);
        return rc;
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
