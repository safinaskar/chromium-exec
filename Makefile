export

CXX ?= c++
CPPFLAGS ?= -DNDEBUG
CXXFLAGS ?= -O3 -g -flto -Wall -Wextra -pedantic
LDFLAGS ?= -flto

all: chromium-exec

.DELETE_ON_ERROR:

FORCE:

libsh-treis/libsh-treis.hpp: FORCE
	$(MAKE) -C libsh-treis libsh-treis.hpp

chromium-exec.o: chromium-exec.cpp libsh-treis/libsh-treis.hpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) -std=c++2a -c $<

libsh-treis/lib.a: FORCE
	$(MAKE) -C libsh-treis lib.a

chromium-exec: chromium-exec.o libsh-treis/lib.a
	$(CXX) $(LDFLAGS) -o $@ $^ $$(cat $$(find -L libsh-treis -name libs))
