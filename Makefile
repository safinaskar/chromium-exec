export

CXX ?= c++
ifeq ($(RELEASE),1)
CPPFLAGS ?= -DNDEBUG
CXXFLAGS ?= -O3 -g -flto -Wall -Wextra -pedantic
LDFLAGS ?= -O3 -g -flto
else
CPPFLAGS ?=
CXXFLAGS ?= -g -Wall -Wextra -pedantic -fsanitize=undefined,bounds,nullability,float-divide-by-zero,implicit-conversion,address -fno-sanitize-recover=all -fno-omit-frame-pointer -fsanitize-address-use-after-scope -fno-optimize-sibling-calls
LDFLAGS ?= -g -fsanitize=undefined,bounds,nullability,float-divide-by-zero,implicit-conversion,address -fno-sanitize-recover=all -fno-omit-frame-pointer -fsanitize-address-use-after-scope -fno-optimize-sibling-calls
endif

all: chromium-exec

.DELETE_ON_ERROR:

FORCE:

libsh-treis/%: FORCE
	T='$@'; $(MAKE) -C "$${T%%/*}" "$${T#*/}"

chromium-exec.o: chromium-exec.cpp FORCE
	libsh-treis/compile '$(MAKE)' $< $(CXX) $(CPPFLAGS) $(CXXFLAGS) -std=c++2a

chromium-exec: chromium-exec.o libsh-treis/lib.a
	$(CXX) $(LDFLAGS) -o $@ $^ $$(cat $$(find -L libsh-treis -name libs) < /dev/null)
