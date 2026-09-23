# Resolving Package Versions

The resolver reads every manifest in the tree before it picks a single version.
The algorithm is described in detail in [the resolution notes][res-notes], and the
on-disk format of its output is covered by [the lockfile chapter][lockfile-fmt].

If two packages ask for incompatible ranges of the same library, the tool stops and
prints both requirement chains. See [conflicts][] for a worked example, and
[backtracking] for why the search sometimes revisits a decision it made earlier.

**Never edit the lockfile by hand while a build is running. The build reads it
once at startup, so your change is silently lost; see [the locking rules][locking]
for the safe procedure.** Editing it between builds is fine.

Most users never need to touch the registry settings. Those who mirror packages
internally should read [mirrors] first, then [the authentication guide][auth], and
finally the short note on [offline mode][offline] before changing anything.

A version such as `1.4.0-rc.2` is a pre-release, and the resolver only selects it
when a requirement names a pre-release explicitly. This rule follows
[Semantic Versioning][semver] rather than inventing a new one.

- [Yanked versions][yanked] stay downloadable but are skipped by new resolutions.
- A **[path dependency][path-deps] is always rebuilt from source**, even when a
  matching archive is already cached.

[res-notes]: https://example.org/pkgtool/resolution
[lockfile-fmt]: https://example.org/pkgtool/lockfile
[conflicts]: https://example.org/pkgtool/conflicts
[backtracking]: https://example.org/pkgtool/backtracking
[locking]: https://example.org/pkgtool/locking
[mirrors]: https://example.org/pkgtool/mirrors
[auth]: https://example.org/pkgtool/auth
[offline]: https://example.org/pkgtool/offline
[semver]: https://semver.org/
[yanked]: https://example.org/pkgtool/yank
[path-deps]: https://example.org/pkgtool/path-dependencies
