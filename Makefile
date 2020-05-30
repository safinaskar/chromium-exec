export

CXX ?= c++
CPPFLAGS ?= -DNDEBUG
CXXFLAGS ?= -O3 -g -flto -Wall -Wextra -pedantic
LDFLAGS ?= -flto

all: chromium-exec

.DELETE_ON_ERROR:

FORCE:

libsh-treis/%: FORCE
	T='$@'; $(MAKE) -C "$${T%%/*}" "$${T#*/}"

chromium-exec.o: chromium-exec.cpp FORCE
	libsh-treis/compile $< $(CXX) $(CPPFLAGS) $(CXXFLAGS) -std=c++2a

chromium-exec: chromium-exec.o libsh-treis/lib.a
	$(CXX) $(LDFLAGS) -o $@ $^ $$(cat $$(find -L libsh-treis -name libs))
