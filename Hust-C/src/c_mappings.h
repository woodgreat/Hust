/*
 * Hust-C Type Mappings
 * Maps Hust types to C types
 */

#ifndef C_MAPPINGS_H
#define C_MAPPINGS_H

#include <stddef.h>

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
const char* get_required_header(const char* hust_type);

#endif /* C_MAPPINGS_H */
