`nitroarc`
==========

An implementation of the Nitro Archive file format used by the Nintendo DS.

This project is split into two core components: a C library implementing the
archive format specification and a command-line tool for inspecting and
manipulating archives.

Getting Started
---------------

Releases of `nitroarc` are available for a number of different use-cases. All
release archives require only a C compiler as a dependency and provide the files
[`COPYING`](./COPYING) and [`COPYING.LESSER`](./COPYING.LESSER) for copyright
information.

### For Integration Into Other Applications ###

This release provides only the library implementation, amalgamated as a single
source file. Drop the header and this amalgamated source file into your project
as-is, then wire it into your build-system as needed.

### For Vendoring Into a Larger Project ###

This release provides the amalgamated library implementation, the source code
for the command-line tool, and plain-text documentation for both the library and
the command-line tool. Extract the archive into your project, then wire the
library and tool into your build-system as needed.

### For Toolchain Installation ###

This release provides the source code and an associated manual page for both the
library implementation and the command-line tool. The manual pages are formatted
from the plain-text documentation files listed in source-control. A simple
`Makefile` is also provided for building and installing the project:

```bash
cd nitroarc
make
sudo make install
```

By default, this `Makefile` will install to appropriate directories prefixed by
`/usr/local`. If you wish to install to directories with an alternate prefix,
use the `DESTDIR` variable:

```bash
DESTDIR=~/.local make install
```

### Additional Library Targets ###

Beyond the default `make` target, the project also provides FFI-focused build
targets for consumers in non-C languages:

- `make ffi`: build the FFI shared library
  - Linux: `bin/libnitroarc_ffi.so`
  - macOS: `bin/libnitroarc_ffi.dylib`
  - Windows: `bin/nitroarc_ffi.dll`

The FFI entrypoints are declared in
[`ffi/nitroarc_ffi.h`](./ffi/nitroarc_ffi.h), and provide a memory-first API
intended for cross-platform consumers such as Rust and .NET:

- opening archives from in-memory byte streams
- getting archive members by index or virtual path
- retrieving archive member names into a caller buffer or as an allocated string
- building/sealing archives with an owned builder handle
- freeing FFI-owned output buffers

Archive member names and virtual paths are archive-internal paths using `/` as
the directory separator, not host filesystem paths.

Archive member lookup by path accepts either an absolute archive path beginning
with `/` or a relative archive path without that leading `/`. The member-name
APIs return relative archive paths without the leading `/`.

Usage
-----

The library header aims to be well-documented and cover most of the use-cases
associated with manipulating archive files. Example code for the basics is
available in [the library documentation](./doc/nitroarc.3.adoc).

The command-line tool provides help-text (`nitroarc --help`) containing a list
of available options, a summary of each option's functionality, and examples
illustrating rudimentary invocation.

Contributing
------------

Please report bugs and submit patches to [rachel@lhea.me](mailto:rachel@lhea.me)
or [the upstream repository](https://codeberg.org/lhearachel/nitroarc).

Certain build targets are relegated to [`Makefile.devel`](./Makefile.devel).
This makes it easier to strip them from any `Makefile` that should be included
in a release archive, which in-turn minimizes the surface area of installation
dependencies to only a C compiler. If you are making a contribution to the
project, then you may need to install additional dependencies for development:

- Debug builds, by default, attempt to link against the following sanitizers.
  This can be suppressed by overriding the value of the `SANFLAGS` variable when
  running `make debug` to provide alternate (or no) flags to the compiler.
  - `AddressSanitizer`
  - `UndefinedBehaviorSanitizer`

- Manual pages are compiled using `asciidoctor`. This output must be verified if
  you are making a contribution to the documentation.

- The library amalgamation script requires an implementation of `tail` which
  supports a leading plus (`+`) for the `-n` option.
