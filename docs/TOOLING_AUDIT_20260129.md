# Tooling Audit 2026-01-29

== Core CLI ==
OK   git -> /bin/git
git version 2.52.0
OK   rg -> /bin/rg
ripgrep 15.1.0
OK   yq -> /bin/yq
yq (https://github.com/mikefarah/yq/) version v4.50.1

== PDF + OCR ==
OK   ocrmypdf -> /bin/ocrmypdf
16.13.0
OK   tesseract -> /bin/tesseract
tesseract 5.5.2
OK   qpdf -> /bin/qpdf
qpdf version 12.3.2
OK   pdftotext -> /bin/pdftotext
pdftotext version 26.01.0
OK   pdfimages -> /bin/pdfimages
pdfimages version 26.01.0
OK   pdftoppm -> /bin/pdftoppm
pdftoppm version 26.01.0

== Image + Metadata ==
OK   magick -> /bin/magick
Version: ImageMagick 7.1.2-13 Q16-HDRI x86_64 2fae24192:20260118 https://imagemagick.org
OK   identify -> /bin/identify
Version: ImageMagick 7.1.2-13 Q16-HDRI x86_64 2fae24192:20260118 https://imagemagick.org
OK   exiftool -> /usr/bin/vendor_perl/exiftool
13.44

== Acquisition ==
OK   aria2c -> /bin/aria2c
aria2 version 1.37.0
OK   wget2 -> /bin/wget2
GNU Wget2 2.2.0 - multithreaded metalink/file/website downloader
OK   axel -> /bin/axel
Axel 2.17.14 (linux-gnu)

== Rust Tooling ==
OK   cargo -> /bin/cargo
cargo 1.94.0-nightly (b54051b15 2025-12-30)
OK   rustc -> /bin/rustc
rustc 1.94.0-nightly (0aced202c 2026-01-06)
OK   cargo-deny
cargo-deny 0.19.0
OK   cargo-audit
cargo-audit 0.22.0
OK   cargo-llvm-cov
cargo-llvm-cov 0.6.22

== Python Stack ==
OK   python -> /bin/python
Python 3.14.2
OK   python:pdfplumber 0.11.7
OK   python:pdfminer 20250506
OK   python:pytesseract 0.3.13
OK   python:PIL 12.1.0
OK   python:cv2 4.13.0
INFO onnxruntime.providers ['CUDAExecutionProvider', 'DnnlExecutionProvider', 'CPUExecutionProvider']
INFO torch.cuda.is_available True
INFO torch.cuda.device0 NVIDIA GeForce RTX 4070 Ti
OK   python:ocrmypdf 16.13.0
OK   python:pikepdf 10.2.0
OK   python:fitz 1.26.7

== pipx ==
OK   pipx
1.8.0
venvs are in /home/eirikr/.local/share/pipx/venvs
apps are exposed on your $PATH at /home/eirikr/.local/bin
manual pages are exposed at /home/eirikr/.local/share/man
   package aiohomekit 3.2.20, installed using Python 3.14.2
    - aiohomekitctl
   package bandit 1.9.2, installed using Python 3.14.2
    - bandit
    - bandit-baseline
    - bandit-config-generator
    - man1/bandit.1
   package beautysh 6.4.2, installed using Python 3.14.2
    - beautysh
   package catt 0.13.1, installed using Python 3.14.2
    - catt
   package detect-secrets 1.5.0, installed using Python 3.14.2
    - detect-secrets
    - detect-secrets-hook
   package linkchecker 10.6.0, installed using Python 3.14.2
    - linkchecker
    - man1/linkchecker.1
    - man5/linkcheckerrc.5
   package mutmut 3.4.0, installed using Python 3.14.2
    - mutmut
   package mypy 1.19.1, installed using Python 3.14.2
    - dmypy
    - mypy
    - mypyc
    - stubgen
    - stubtest
   package ollmcp 0.25.2, installed using Python 3.14.2
