export

CXX ?= c++
CPPFLAGS ?= -DNDEBUG
CXXFLAGS ?= -O3 -g -flto -Wall -Wextra -pedantic
LDFLAGS ?= -flto

all: chromium-exec

FORCE:

libsh-treis/libsh-treis.hpp: FORCE
	$(MAKE) -C libsh-treis libsh-treis.hpp

chromium-exec.o: chromium-exec.cpp libsh-treis/libsh-treis.hpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) -std=c++2a -c $<

libsh-treis/stamp: FORCE
	$(MAKE) -C libsh-treis

chromium-exec: chromium-exec.o libsh-treis/stamp
	$(CXX) $(LDFLAGS) -o $@ $< $$(find -L libsh-treis -name '*.o') $$(cat $$(find -L libsh-treis -name libs))
