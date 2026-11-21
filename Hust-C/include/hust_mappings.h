/*
 * hust_mappings.h - Type mappings for Hust-C
 * 
 * Maps Hust types to C types
 */

#ifndef HUST_MAPPINGS_H
#define HUST_MAPPINGS_H

#include "hust_ast.h"

/* Type mapping structure */
typedef struct {
    const char* hust_type;
    const char* c_type;
} TypeMapping;

/* Function declarations */
const char* get_c_type(const char* hust_type);
const char* get_hust_type(const char* c_type);
bool is_valid_hust_type(const char* type);
bool is_primitive_type(const char* type);
bool is_string_type(const char* type);
bool is_array_type(const char* type);

/* Type conversion helpers */
const char* convert_type_for_param(const char* hust_type);
const char* convert_type_for_return(const char* hust_type);
const char* convert_type_for_variable(const char* hust_type);

#endif /* HUST_MAPPINGS_H */
