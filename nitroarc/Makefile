.PHONY: all clean ffi install lib tool uninstall

SHELL    := /bin/sh
BINDIR   := bin
BUILDDIR := $(BINDIR)/build
MANDIR   := $(BINDIR)/man

DESTDIR  ?= /usr/local

CFLAGS += -std=c99 -MMD -Iffi -Ilib/include
CFLAGS += -Wall -Wextra -Wpedantic -Wconversion -Wno-sign-conversion

ifeq ($(OS),Windows_NT)
FFI_SHARED  := $(BINDIR)/nitroarc_ffi.dll
FFI_CFLAGS  += -DNITROARC_FFI_BUILD_SHARED
FFI_SOFLAGS += -shared
FFI_SOFLAGS += -Wl,--out-implib,$(BINDIR)/libnitroarc_ffi.dll.a
else
UNAME_S := $(shell uname -s)
PICFLAGS += -fPIC -fvisibility=hidden
FFI_CFLAGS += -DNITROARC_FFI_BUILD_SHARED
ifeq ($(UNAME_S),Darwin)
FFI_SHARED  := $(BINDIR)/libnitroarc_ffi.dylib
FFI_SOFLAGS += -shared -Wl,-install_name,@rpath/libnitroarc_ffi.dylib
else
FFI_SHARED  := $(BINDIR)/libnitroarc_ffi.so
FFI_SOFLAGS += -shared -Wl,-soname,libnitroarc_ffi.so
endif
endif

# =============================== CORE RECIPES =============================== #

all: CFLAGS += -O3 -Werror -DNDEBUG
all: lib tool

lib:        $(BINDIR)/libnitroarc.a
ffi: CFLAGS += -O3 -Werror -DNDEBUG
ffi:        $(FFI_SHARED)
tool:       $(BINDIR)/nitroarc

install: all
	mkdir -p $(DESTDIR)/include $(DESTDIR)/lib $(DESTDIR)/bin
	mkdir -p $(DESTDIR)/share/man/man1 $(DESTDIR)/share/man/man3
	install -m 644 lib/include/nitroarc.h $(DESTDIR)/include
	install -m 644 $(BINDIR)/libnitroarc.a $(DESTDIR)/lib
	install -m 755 $(BINDIR)/nitroarc $(DESTDIR)/bin
	install -m 644 $(MANDIR)/nitroarc.1 $(DESTDIR)/share/man/man1
	install -m 644 $(MANDIR)/nitroarc.3 $(DESTDIR)/share/man/man3

uninstall:
	$(RM) $(DESTDIR)/include/nitroarc.h
	$(RM) $(DESTDIR)/lib/libnitroarc.a
	$(RM) $(DESTDIR)/bin/nitroarc
	$(RM) $(DESTDIR)/share/man/man1/nitroarc.1
	$(RM) $(DESTDIR)/share/man/man3/nitroarc.3

clean:
	$(RM) -r $(BINDIR)

# =========================== TARGET CONFIGURATION =========================== #

LIB_C := $(wildcard lib/*.c)
FFI_C := $(wildcard ffi/*.c)
SRC_C := $(wildcard src/*.c)

LIB_O := $(patsubst %.c,$(BUILDDIR)/%.o,$(LIB_C))
FFI_O := $(patsubst %.c,$(BUILDDIR)/%.o,$(FFI_C))
SRC_O := $(patsubst %.c,$(BUILDDIR)/%.o,$(SRC_C))

LIB_PIC_O := $(patsubst %.c,$(BUILDDIR)/pic/%.o,$(LIB_C))
FFI_PIC_O := $(patsubst %.c,$(BUILDDIR)/pic/%.o,$(FFI_C))

$(BINDIR)/libnitroarc.a: $(LIB_O)
$(BINDIR)/libnitroarc.a: | $(BINDIR)
	$(AR) rcs $@ $^

$(FFI_SHARED): $(LIB_PIC_O) $(FFI_PIC_O)
$(FFI_SHARED): | $(BINDIR)
	$(CC) $(LDFLAGS) $(FFI_SOFLAGS) -o $@ $^

$(BINDIR)/nitroarc: $(SRC_O)
$(BINDIR)/nitroarc: $(BINDIR)/libnitroarc.a
$(BINDIR)/nitroarc: | $(BINDIR)
	$(CC) $(LDFLAGS) -o $@ $^

$(BUILDDIR)/%.o: %.c | $(BUILDDIR)
	$(CC) $(CFLAGS) -c -o $@ $<

$(BUILDDIR)/pic/lib/%.o: lib/%.c | $(BUILDDIR)
	$(CC) $(CFLAGS) $(PICFLAGS) -c -o $@ $<

$(BUILDDIR)/pic/ffi/%.o: ffi/%.c | $(BUILDDIR)
	$(CC) $(CFLAGS) $(FFI_CFLAGS) $(PICFLAGS) -c -o $@ $<

$(BUILDDIR):
	@mkdir -p $@
	@mkdir -p $@/lib $@/src $@/ffi
	@mkdir -p $@/pic/lib $@/pic/ffi

$(BINDIR):
	@mkdir -p $@

-include $(LIB_O:%.o=%.d)
-include $(FFI_O:%.o=%.d)
-include $(SRC_O:%.o=%.d)
-include $(LIB_PIC_O:%.o=%.d)
-include $(FFI_PIC_O:%.o=%.d)
include Makefile.devel
