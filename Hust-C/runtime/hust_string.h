/*
 * HustString - C runtime for Hust String type
 * 
 * This provides a dynamic string implementation for Hust code
 * transpiled to C.
 */

#ifndef HUST_STRING_H
#define HUST_STRING_H

#include <stddef.h>
#include <stdbool.h>

/* Opaque string type */
typedef struct HustString HustString;

/* Lifecycle functions */
HustString* HustString_new(const char* cstr);
HustString* HustString_clone(const HustString* s);
void HustString_free(HustString* s);

/* Accessors */
size_t HustString_len(const HustString* s);
char HustString_char_at(const HustString* s, size_t index);
const char* HustString_cstr(const HustString* s);

/* Operations */
HustString* HustString_concat(const HustString* a, const HustString* b);
bool HustString_equals(const HustString* a, const HustString* b);

#endif /* HUST_STRING_H */
