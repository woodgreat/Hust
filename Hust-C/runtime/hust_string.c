/*
 * HustString - C runtime implementation
 */

#include "hust_string.h"
#include <stdlib.h>
#include <string.h>

struct HustString {
    char* data;
    size_t len;
    size_t capacity;
};

HustString* HustString_new(const char* cstr) {
    if (!cstr) return NULL;
    
    HustString* s = (HustString*)malloc(sizeof(HustString));
    if (!s) return NULL;
    
    s->len = strlen(cstr);
    s->capacity = s->len + 1;
    s->data = (char*)malloc(s->capacity);
    if (!s->data) {
        free(s);
        return NULL;
    }
    
    strcpy(s->data, cstr);
    return s;
}

HustString* HustString_clone(const HustString* s) {
    if (!s) return NULL;
    return HustString_new(s->data);
}

void HustString_free(HustString* s) {
    if (!s) return;
    free(s->data);
    free(s);
}

size_t HustString_len(const HustString* s) {
    if (!s) return 0;
    return s->len;
}

char HustString_char_at(const HustString* s, size_t index) {
    if (!s || index >= s->len) return '\0';
    return s->data[index];
}

const char* HustString_cstr(const HustString* s) {
    if (!s) return "";
    return s->data;
}

HustString* HustString_concat(const HustString* a, const HustString* b) {
    if (!a || !b) return NULL;
    
    size_t new_len = a->len + b->len;
    char* buffer = (char*)malloc(new_len + 1);
    if (!buffer) return NULL;
    
    strcpy(buffer, a->data);
    strcat(buffer, b->data);
    
    HustString* result = HustString_new(buffer);
    free(buffer);
    return result;
}

bool HustString_equals(const HustString* a, const HustString* b) {
    if (!a || !b) return false;
    if (a->len != b->len) return false;
    return strcmp(a->data, b->data) == 0;
}
