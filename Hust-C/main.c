/*
 * main.c - Main entry point for Hust-C transpiler
 * 
 * Pure C implementation - no Rust dependencies
 */

#define _XOPEN_SOURCE 700

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include "include/hust_ast.h"
#include "include/hust_preprocessor.h"
#include "include/hust_parser.h"
#include "include/hust_translator.h"

#define HUST_C_VERSION "0.1.0"
#define HUST_PATH_MAX 4096

/* Print usage information */
void print_usage(const char* program_name) {
    fprintf(stderr, "Usage: %s <input.hust> [-o output.c] [--project]\n", program_name);
    fprintf(stderr, "       %s transpile <entry> [--project] --out-dir <dir>\n", program_name);
    fprintf(stderr, "       %s build <entry> [--project] --out-dir <dir> [--release|--debug]\n", program_name);
    fprintf(stderr, "       %s info | --version | --help\n", program_name);
}

/* Print help information */
void print_help(void) {
    printf("Hust-C Transpiler\n");
    printf("=================\n\n");
    printf("Converts Hust source code to C code.\n\n");
    printf("Legacy usage:\n");
    printf("  hust_c <input.hust> [-o output.c]  Transpile Hust file to C\n");
    printf("  hust_c <entry.hust> --project       Transpile entry and user modules\n\n");
    printf("Core protocol:\n");
    printf("  hust_c transpile <entry> [--project] --out-dir <dir>\n");
    printf("  hust_c build <entry> [--project] --out-dir <dir> [--release|--debug]\n");
    printf("  hust_c info\n");
    printf("  hust_c --version\n\n");
    printf("Options:\n");
    printf("  -o <file>    Specify output file (default: stdout)\n");
    printf("  --out-dir    Directory for generated C and build output\n");
    printf("  --release    Optimized build (default for build)\n");
    printf("  --debug      Debug build\n");
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

/* ------------------------------------------------------------------------- */
/* Core protocol support                                                     */
/* ------------------------------------------------------------------------- */

static int copy_file(const char* src, const char* dst) {
    FILE* in = fopen(src, "rb");
    if (!in) {
        fprintf(stderr, "Error: cannot open '%s': %s\n", src, strerror(errno));
        return 1;
    }
    FILE* out = fopen(dst, "wb");
    if (!out) {
        fprintf(stderr, "Error: cannot create '%s': %s\n", dst, strerror(errno));
        fclose(in);
        return 1;
    }

    char buffer[4096];
    size_t count;
    int rc = 0;
    while ((count = fread(buffer, 1, sizeof(buffer), in)) > 0) {
        if (fwrite(buffer, 1, count, out) != count) {
            fprintf(stderr, "Error: cannot write '%s': %s\n", dst, strerror(errno));
            rc = 1;
            break;
        }
    }
    if (ferror(in)) {
        fprintf(stderr, "Error: cannot read '%s': %s\n", src, strerror(errno));
        rc = 1;
    }
    fclose(in);
    if (fclose(out) != 0) {
        fprintf(stderr, "Error: cannot close '%s': %s\n", dst, strerror(errno));
        rc = 1;
    }
    return rc;
}

static int ensure_dir(const char* path) {
    char buffer[HUST_PATH_MAX];
    size_t length = strlen(path);
    if (length == 0 || length >= sizeof(buffer)) {
        fprintf(stderr, "Error: invalid directory '%s'\n", path);
        return 1;
    }
    memcpy(buffer, path, length + 1);
    while (length > 1 && buffer[length - 1] == '/') {
        buffer[--length] = '\0';
    }

    for (char* p = buffer + 1; *p; p++) {
        if (*p != '/') continue;
        *p = '\0';
        if (mkdir(buffer, 0755) != 0 && errno != EEXIST) {
            fprintf(stderr, "Error: cannot create '%s': %s\n", buffer, strerror(errno));
            return 1;
        }
        *p = '/';
    }
    if (mkdir(buffer, 0755) != 0 && errno != EEXIST) {
        fprintf(stderr, "Error: cannot create '%s': %s\n", buffer, strerror(errno));
        return 1;
    }
    return 0;
}

static void path_stem(const char* path, char* out, size_t out_size) {
    const char* base = strrchr(path, '/');
    base = base ? base + 1 : path;
    snprintf(out, out_size, "%s", base);
    char* dot = strrchr(out, '.');
    if (dot) *dot = '\0';
    if (out[0] == '\0') {
        snprintf(out, out_size, "main");
    }
}

static void executable_dir(const char* argv0, char* out, size_t out_size) {
#ifdef __linux__
    char proc_path[HUST_PATH_MAX];
    ssize_t length = readlink("/proc/self/exe", proc_path, sizeof(proc_path) - 1);
    if (length > 0) {
        proc_path[length] = '\0';
        project_dirname(proc_path, out, out_size);
        return;
    }
#endif
    project_dirname(argv0, out, out_size);
}

static bool runtime_include_ok(const char* dir) {
    char path[HUST_PATH_MAX];
    snprintf(path, sizeof(path), "%s/hust_error.h", dir);
    return access(path, R_OK) == 0;
}

static bool find_runtime_include(const char* argv0, char* out, size_t out_size) {
    const char* env = getenv("HUST_C_INCLUDE");
    if (env && *env && runtime_include_ok(env)) {
        snprintf(out, out_size, "%s", env);
        return true;
    }

    char exe_dir[HUST_PATH_MAX];
    executable_dir(argv0, exe_dir, sizeof(exe_dir));
    snprintf(out, out_size, "%s/include", exe_dir);
    if (runtime_include_ok(out)) return true;

    snprintf(out, out_size, "%s/../include", exe_dir);
    if (runtime_include_ok(out)) return true;

    snprintf(out, out_size, "include");
    return runtime_include_ok(out);
}

static int copy_runtime_headers(const char* include_dir, const char* out_dir) {
    static const char* headers[] = {"hust_error.h", "hust_vec.h", "hust_string.h"};
    char dst_dir[HUST_PATH_MAX];
    snprintf(dst_dir, sizeof(dst_dir), "%s/include", out_dir);
    if (ensure_dir(dst_dir) != 0) return 1;

    for (size_t i = 0; i < sizeof(headers) / sizeof(headers[0]); i++) {
        char src[HUST_PATH_MAX];
        char dst[HUST_PATH_MAX];
        snprintf(src, sizeof(src), "%s/%s", include_dir, headers[i]);
        snprintf(dst, sizeof(dst), "%s/%s", dst_dir, headers[i]);
        if (copy_file(src, dst) != 0) return 1;
    }
    return 0;
}

static bool command_available(const char* name) {
    const char* path = getenv("PATH");
    if (!path) return false;

    char candidate[HUST_PATH_MAX];
    const char* start = path;
    for (;;) {
        const char* end = strchr(start, ':');
        size_t length = end ? (size_t)(end - start) : strlen(start);
        if (length == 0) {
            snprintf(candidate, sizeof(candidate), "%s", name);
        } else {
            snprintf(candidate, sizeof(candidate), "%.*s/%s", (int)length, start, name);
        }
        if (access(candidate, X_OK) == 0) return true;
        if (!end) break;
        start = end + 1;
    }
    return false;
}

static void print_core_info(void) {
    printf("{\"protocol\":1,\"name\":\"c\",\"language\":\"c\","
           "\"transpile\":true,\"build\":true,\"toolchain\":%s}\n",
           command_available("gcc") ? "true" : "false");
}

static int core_transpile(const char* entry, bool project_mode,
                          const char* out_dir, const char* argv0) {
    char include_dir[HUST_PATH_MAX];
    if (!find_runtime_include(argv0, include_dir, sizeof(include_dir))) {
        fprintf(stderr, "Error: Hust-C runtime headers not found (set HUST_C_INCLUDE)\n");
        return 1;
    }
    if (ensure_dir(out_dir) != 0) return 1;
    if (copy_runtime_headers(include_dir, out_dir) != 0) return 1;

    char stem[256];
    char c_path[HUST_PATH_MAX];
    path_stem(entry, stem, sizeof(stem));
    snprintf(c_path, sizeof(c_path), "%s/%s.c", out_dir, stem);

    FILE* output = fopen(c_path, "w");
    if (!output) {
        fprintf(stderr, "Error: cannot create '%s': %s\n", c_path, strerror(errno));
        return 1;
    }
    int rc = project_mode ? project_transpile(entry, output)
                          : transpile_file_to(entry, output);
    if (fclose(output) != 0) rc = 1;
    return rc == 0 ? 0 : 1;
}

static int run_c_compiler(const char* c_path, const char* include_dir,
                          const char* exe_path, bool debug) {
    char include_arg[HUST_PATH_MAX + 2];
    snprintf(include_arg, sizeof(include_arg), "-I%s", include_dir);

    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "Error: fork failed: %s\n", strerror(errno));
        return 1;
    }
    if (pid == 0) {
        char* args[12];
        int n = 0;
        args[n++] = (char*)"gcc";
        args[n++] = (char*)"-std=c99";
        args[n++] = include_arg;
        if (debug) {
            args[n++] = (char*)"-O0";
            args[n++] = (char*)"-g";
        } else {
            args[n++] = (char*)"-O2";
        }
        args[n++] = (char*)c_path;
        args[n++] = (char*)"-o";
        args[n++] = (char*)exe_path;
        args[n++] = (char*)"-lm";
        args[n] = NULL;
        execvp("gcc", args);
        fprintf(stderr, "Error: cannot execute gcc: %s\n", strerror(errno));
        _exit(127);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        fprintf(stderr, "Error: waitpid failed: %s\n", strerror(errno));
        return 1;
    }
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        fprintf(stderr, "Error: C compilation failed\n");
        return 1;
    }
    return 0;
}

static int core_build(const char* entry, bool project_mode,
                      const char* out_dir, bool debug, const char* argv0) {
    if (core_transpile(entry, project_mode, out_dir, argv0) != 0) return 1;

    char stem[256];
    char c_path[HUST_PATH_MAX];
    char exe_path[HUST_PATH_MAX];
    char include_dir[HUST_PATH_MAX];
    path_stem(entry, stem, sizeof(stem));
    snprintf(c_path, sizeof(c_path), "%s/%s.c", out_dir, stem);
    snprintf(exe_path, sizeof(exe_path), "%s/%s", out_dir, stem);
    snprintf(include_dir, sizeof(include_dir), "%s/include", out_dir);

    if (run_c_compiler(c_path, include_dir, exe_path, debug) != 0) return 1;

    char absolute[HUST_PATH_MAX];
    if (!realpath(exe_path, absolute)) {
        fprintf(stderr, "Error: cannot resolve '%s': %s\n", exe_path, strerror(errno));
        return 1;
    }
    printf("executable=%s\n", absolute);
    return 0;
}

static int core_command(int argc, char* argv[], bool build_mode, const char* argv0) {
    const char* entry = NULL;
    const char* out_dir = NULL;
    bool project_mode = false;
    bool release_flag = false;
    bool debug_flag = false;

    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "--help") == 0 || strcmp(argv[i], "-h") == 0) {
            print_help();
            return 0;
        } else if (strcmp(argv[i], "--project") == 0) {
            project_mode = true;
        } else if (strcmp(argv[i], "--out-dir") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Error: --out-dir requires a path\n");
                return 2;
            }
            out_dir = argv[++i];
        } else if (build_mode && strcmp(argv[i], "--release") == 0) {
            release_flag = true;
        } else if (build_mode && strcmp(argv[i], "--debug") == 0) {
            debug_flag = true;
        } else if (argv[i][0] == '-') {
            fprintf(stderr, "Error: unknown option '%s'\n", argv[i]);
            return 2;
        } else if (!entry) {
            entry = argv[i];
        } else {
            fprintf(stderr, "Error: unexpected argument '%s'\n", argv[i]);
            return 2;
        }
    }

    if (release_flag && debug_flag) {
        fprintf(stderr, "Error: --release and --debug cannot be combined\n");
        return 2;
    }
    if (!entry) {
        fprintf(stderr, "Error: missing <entry>\n");
        return 2;
    }
    if (!out_dir) {
        fprintf(stderr, "Error: missing --out-dir <dir>\n");
        return 2;
    }

    if (build_mode) {
        return core_build(entry, project_mode, out_dir, debug_flag, argv0);
    }
    return core_transpile(entry, project_mode, out_dir, argv0);
}

/* Main function */
int main(int argc, char* argv[]) {
    /* Check for help flag */
    if (argc < 2 || strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0) {
        print_help();
        return argc < 2 ? 1 : 0;
    }

    /* Core protocol commands are additive; legacy invocation is unchanged. */
    if (strcmp(argv[1], "--version") == 0 || strcmp(argv[1], "-V") == 0) {
        printf("hust_c %s\n", HUST_C_VERSION);
        return 0;
    }
    if (strcmp(argv[1], "info") == 0) {
        print_core_info();
        return 0;
    }
    if (strcmp(argv[1], "transpile") == 0) {
        return core_command(argc, argv, false, argv[0]);
    }
    if (strcmp(argv[1], "build") == 0) {
        return core_command(argc, argv, true, argv[0]);
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
