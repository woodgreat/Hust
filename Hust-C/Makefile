# Hust-C Makefile
# Builds the Hust-C transpiler

CC = gcc
CFLAGS = -Wall -Wextra -std=c99 -O2 -I./src
TARGET = hust_c
SRCS = main.c src/preprocessor.c src/parser.c src/translator.c src/c_mappings.c
OBJS = $(SRCS:.c=.o)

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $(TARGET) $(OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(TARGET) $(OBJS)

test: $(TARGET)
	./tests/run_tests.sh

.PHONY: all clean test
