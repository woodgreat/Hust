/*
 * hust_mappings.c - Type mappings implementation for Hust-C
 * 
 * Maps Hust types to C types
 */

#include "../include/hust_mappings.h"

/* Type mapping table */
static const TypeMapping type_mappings[] = {
    /* Primitive types */
    {"i8", "int8_t"},
    {"i16", "int16_t"},
    {"i32", "int32_t"},
    {"i64", "int64_t"},
    {"u8", "uint8_t"},
    {"u16", "uint16_t"},
    {"u32", "uint32_t"},
    {"u64", "uint64_t"},
    {"usize", "size_t"},
    {"isize", "ptrdiff_t"},
    {"f32", "float"},
    {"f64", "double"},
    {"bool", "bool"},
    {"char", "char"},
    {"void", "void"},
    
    /* String type */
    {"String", "HustString*"},
    
    /* Array types */
    {"i32[]", "HustVec_i32*"},
    {"i64[]", "HustVec_i64*"},
    {"f32[]", "HustVec_f32*"},
    {"f64[]", "HustVec_f64*"},
    {"bool[]", "HustVec_bool*"},
    {"char[]", "HustVec_char*"},
    {"String[]", "HustVec_String*"},
    
    /* End marker */
    {NULL, NULL}
};

/* Get C type for a Hust type */
const char* get_c_type(const char* hust_type) {
    if (!hust_type) return "void";
    
    /* Look up in mapping table */
    for (int i = 0; type_mappings[i].hust_type != NULL; i++) {
        if (strcmp(type_mappings[i].hust_type, hust_type) == 0) {
            return type_mappings[i].c_type;
        }
    }
    
    /* Default: return as-is (unknown type) */
    return hust_type;
}

/* Get Hust type for a C type */
const char* get_hust_type(const char* c_type) {
    if (!c_type) return "void";
    
    /* Look up in mapping table (reverse) */
    for (int i = 0; type_mappings[i].c_type != NULL; i++) {
        if (strcmp(type_mappings[i].c_type, c_type) == 0) {
            return type_mappings[i].hust_type;
        }
    }
    
    /* Default: return as-is (unknown type) */
    return c_type;
}

/* Check if a type is a valid Hust type */
bool is_valid_hust_type(const char* type) {
    if (!type) return false;
    
    /* Check primitive types */
    if (is_primitive_type(type)) return true;
    if (is_string_type(type)) return true;
    if (is_array_type(type)) return true;
    
    return false;
}

/* Check if a type is a primitive type */
bool is_primitive_type(const char* type) {
    if (!type) return false;
    
    return (strcmp(type, "i8") == 0 ||
            strcmp(type, "i16") == 0 ||
            strcmp(type, "i32") == 0 ||
            strcmp(type, "i64") == 0 ||
            strcmp(type, "u8") == 0 ||
            strcmp(type, "u16") == 0 ||
            strcmp(type, "u32") == 0 ||
            strcmp(type, "u64") == 0 ||
            strcmp(type, "usize") == 0 ||
            strcmp(type, "isize") == 0 ||
            strcmp(type, "f32") == 0 ||
            strcmp(type, "f64") == 0 ||
            strcmp(type, "bool") == 0 ||
            strcmp(type, "char") == 0 ||
            strcmp(type, "void") == 0);
}

/* Check if a type is a string type */
bool is_string_type(const char* type) {
    if (!type) return false;
    return (strcmp(type, "String") == 0);
}

/* Check if a type is an array type */
bool is_array_type(const char* type) {
    if (!type) return false;
    
    /* Check if ends with [] */
    size_t len = strlen(type);
    if (len < 2) return false;
    
    return (type[len-2] == '[' && type[len-1] == ']');
}

/* Convert type for function parameter */
const char* convert_type_for_param(const char* hust_type) {
    /* For now, same as regular conversion */
    return get_c_type(hust_type);
}

/* Convert type for function return */
const char* convert_type_for_return(const char* hust_type) {
    /* For now, same as regular conversion */
    return get_c_type(hust_type);
}

/* Convert type for variable declaration */
const char* convert_type_for_variable(const char* hust_type) {
    /* For now, same as regular conversion */
    return get_c_type(hust_type);
}
