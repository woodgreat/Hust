/*
 * Hust-C Type Mappings Implementation
 * Maps Hust types to C types
 */

#include "c_mappings.h"
#include <string.h>

/* Get C type for Hust type */
const char* get_c_type(const char* hust_type) {
    if (!hust_type) return "void";
    
    for (int i = 0; type_mappings[i].hust_type != NULL; i++) {
        if (strcmp(type_mappings[i].hust_type, hust_type) == 0) {
            return type_mappings[i].c_type;
        }
    }
    
    /* Return as-is if not found */
    return hust_type;
}

/* Get required header for Hust type */
const char* get_required_header(const char* hust_type) {
    if (!hust_type) return NULL;
    
    for (int i = 0; type_mappings[i].hust_type != NULL; i++) {
        if (strcmp(type_mappings[i].hust_type, hust_type) == 0) {
            return type_mappings[i].header;
        }
    }
    
    return NULL;
}
