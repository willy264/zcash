# Native Windows backend compatibility

The pinned upstream lightwalletd v0.6.0 opens its block-cache files with
`os.O_APPEND`. On native Windows, Darkside's `Reset` then fails with
`truncate lengths file failed: Access is denied.` The Go append handle does
not permit the truncate operation used by `common/cache.go`.

On Windows only, `scripts/setup_backend.py` replaces the cache's
`c.lengthsFile.Truncate(...)` and `c.blocksFile.Truncate(...)` calls with
`os.Truncate(c.lengthsName, ...)` and `os.Truncate(c.blocksName, ...)`.
This opens a separate writable handle to the same file. The source archive
is checksum-verified before this small build-time patch is applied, and the
build stamp records `windows-truncate-v1`.

No transaction, cryptography, protocol, or Darkside staging code changes.
Linux and macOS builds, including GitHub Actions, use the unmodified pinned
upstream source. `--nocache` is unsuitable here: upstream Darkside's reset
path assumes that its cache exists.
