#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void main(void) {
    HustString* s = HustString_new("Hello, World!");
    HustString* empty = HustString_new("");
    HustString* cloned = HustString_clone(s);

    size_t len = HustString_len(s);
    char c = HustString_char_at(s, 0);

    HustString* temp = HustString_new("!!!");
    HustString* concat = HustString_concat(s, temp);
    HustString_free(temp);

    HustString* temp2 = HustString_new("Hello, World!");
    bool eq = HustString_equals(s, temp2);
    HustString_free(temp2);

    printf("%s\n", HustString_cstr(s));

    HustString_free(s);
    HustString_free(empty);
    HustString_free(cloned);
    HustString_free(concat);
}
