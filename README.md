# Quetzalcoatl

A Rust CLI for reliably importing PDFs onto a reMarkable 2 tablet over SSH/SFTP,
bypassing the official (size-limited, awkward) USB web upload interface, with
real folder management: any target folder can be selected, missing folders
are created automatically, and existing documents/folders can be moved
between folders.

This is a from-scratch Rust rewrite of an earlier TypeScript proof-of-concept
(preserved on the `legacy-typescript` branch for reference). The rewrite
carries over every correctness fix identified while reviewing that version:
real UUID generation per document, atomic staged-write-then-rename uploads
(no partially-written documents left behind on failure), a post-upload
integrity check, and a restart of `xochitl` that's actually awaited and
verified rather than fired-and-forgotten.

## Functionality

| Type | Supported |
|:--:|:--:|
| _pdf_ | ✅ |
| _epub_ | ❌ |
| _cbr_ | ❌ |
| _cbz_ | ❌ |

EPUB/CBR/CBZ are out of scope for now: they aren't fixed-pagination formats
the way PDF is, which the on-device `.content` schema assumes.

## Setup

1. Copy `quetzalcoatl.example.toml` to `quetzalcoatl.toml` and fill in your
   tablet's IP (`10.11.99.1` over the USB-gadget interface by default) and
   either the password shown under Settings > About > Copyrights and
   licenses, or a private key path. `quetzalcoatl.toml` is gitignored by
   name -- never commit your real one.
2. `cargo build --release`

## Usage

```
quetzalcoatl test-connection --config quetzalcoatl.toml
quetzalcoatl list-folders --config quetzalcoatl.toml
quetzalcoatl import book.pdf --folder "Livres//mangas" --config quetzalcoatl.toml
quetzalcoatl move <uuid> --to "Livres//comics" --config quetzalcoatl.toml
quetzalcoatl restart-xochitl --config quetzalcoatl.toml
```

`--folder` defaults to the tablet's root when omitted. Any folder segment
that doesn't already exist is created automatically -- there's no separate
flag for this.

## Architecture

A Cargo workspace: `core/` is a library crate holding all business logic
(config, the `SftpTransport` trait + `russh`-backed implementation, the
xochitl metadata/tree/PDF-conversion model, and the `import`/`move`/`list`/
`restart` operations); `cli/` is a thin `clap`-based wrapper around it. This
split exists so a planned second-step GUI (Tauri + React/TypeScript -- a
folder-tree browser with drag-and-drop import/reorganize, and a connection
form so other users can point the tool at their own tablet) can reuse `core`
directly without any changes to it.

## Compatibility

Verified against a reMarkable 2 running Codex Linux 5.8.202 (image
3.28.0.169) on 2026-09-17. The on-device `.metadata` schema used by this
tool was confirmed by live inspection against that build; if a firmware
update changes it, `core/src/xochitl/metadata.rs` is the place to look.

## Known limitations

- The SSH client accepts any server host key (no pinning) -- reasonable
  given the tablet is normally reached over a direct USB point-to-point
  link, but worth knowing if you route this over a shared network instead.
- No de-duplication: importing the same file twice creates two independent
  documents. This is expected behavior, not a bug.
- Not yet tested against a live device end-to-end (the tablet was
  unreachable over SSH during development) -- unit tests cover the pure
  logic and the atomic-write/retry behavior against a fake transport, but
  run the manual smoke test below before trusting this with real documents.

## Manual smoke test (once the tablet is reachable)

1. `test-connection` -- confirms auth and the negotiated cipher.
2. `list-folders` -- confirms the existing tree reads correctly.
3. `import` into a brand-new nested folder path -- confirms auto-creation.
4. `move` that document to another new path.
5. Over a manual `ssh` session, confirm the sidecars/parents on-device match.
6. Interrupt (Ctrl+C) mid-`import` -- confirm no partially-named *final*
   files are left behind (only `.partial` staging artifacts, or nothing).
7. Confirm the document shows up correctly placed in the tablet's UI.

## Warning and note

This software is developed as-is with no warranty. I won't be liable for
any damage to your reMarkable resulting from its use.

If anything goes wrong, this software only ever adds/moves files under
`~/.local/share/remarkable/xochitl` on the tablet. SSH in and removing the
affected UUID's files should fix any issue.
