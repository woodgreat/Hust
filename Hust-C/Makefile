# Hust-C Makefile
# Pure C implementation - no Rust dependencies

CC = gcc
CFLAGS = -Wall -Wextra -std=c99 -O2 -I./include
LDFLAGS =

# Source files
SRCS = src/hust_ast.c \
       src/hust_preprocessor.c \
       src/hust_parser.c \
       src/hust_mappings.c \
       src/hust_translator.c \
       src/hust_error.c

OBJS = $(SRCS:.c=.o)

# Main target
TARGET = hust_c

# Default target
all: $(TARGET)

# Link main program
$(TARGET): main.o $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

# Compile source files
%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

# Unit tests
test_translator: tests/unit/test_translator.o $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

test_ast: tests/unit/test_ast.o src/hust_ast.o
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

test_preprocessor: tests/unit/test_preprocessor.o src/hust_ast.o src/hust_preprocessor.o
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

test_parser: tests/unit/test_parser.o src/hust_ast.o src/hust_preprocessor.o src/hust_parser.o
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

# Run unit tests
unit-tests: test_ast test_preprocessor test_parser test_translator
	@echo "=== Running Unit Tests ==="
	./test_ast
	./test_preprocessor
	./test_parser
	./test_translator

# Integration tests
integration-tests: $(TARGET)
	@echo "=== Running Integration Tests ==="
	cd tests && bash run_tests.sh

# Run all tests
test: unit-tests integration-tests

# Clean build artifacts
clean:
	rm -f $(TARGET) *.o src/*.o tests/unit/*.o
	rm -f test_ast test_preprocessor test_parser test_translator

# Install (copy to /usr/local/bin)
install: $(TARGET)
	cp $(TARGET) /usr/local/bin/

# Uninstall
uninstall:
	rm -f /usr/local/bin/$(TARGET)

.PHONY: all clean test unit-tests integration-tests install uninstall
