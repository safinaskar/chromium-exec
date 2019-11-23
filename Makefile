export

CXX ?= c++
CPPFLAGS ?=
CXXFLAGS ?= -O3 -g -flto -Wall -Wextra -pedantic
LDFLAGS ?= -flto

all: chromium-exec

FORCE:

libsh-treis/libsh-treis.hpp: FORCE
	$(MAKE) -C libsh-treis libsh-treis.hpp

chromium-exec.o: chromium-exec.cpp libsh-treis/libsh-treis.hpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) -std=c++17 -c $<

libsh-treis/stamp: FORCE
	$(MAKE) -C libsh-treis

chromium-exec: chromium-exec.o libsh-treis/stamp
	$(CXX) $(LDFLAGS) -o $@ $< $$(find -L libsh-treis -name '*.o') $$(cat $$(find -L libsh-treis -name libs))
