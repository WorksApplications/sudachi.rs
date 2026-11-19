Wheel Status
=========================

We provide wheel builds for following platforms:

* Linux-amd64
* Linux-aarch64
* Windows-x86_64, arm64
* macOS-universal2, x86_64, arm64

Wheels for Linux are supported (we will fix bugs and problems when reported).
Wheels for Windows are supported in best-effort manner (we will try to fix bugs when reported, but that can take unspecified amount of time).
Wheels for macOS are **not** supported and we welcome contributions for them.

Linux
------

We build wheels only for amd64 (x86_64) and arm64 (aarch64) architecture at the moment.
Wheels for Linux are built with `Profile-Guided Optimizations <https://doc.rust-lang.org/rustc/profile-guided-optimization.html>`_
and will probably be faster than installations which are manaully-built from the source package.
Wheels are built using manylinux container and should be compatible with most of distributions.

Windows
-------

Builds for Windows are built witout PGO, but should work as if you have built the wheels yourself.
We don't provide builds for x86 architecture.

macOS
------

The situation is similar to Windows, but these builds are tested even less.
We provide universal2 builds which should work both on Intel and ARM-based Macs,
but we only run tests with latest runner in CI and are provided on best effort basis.
