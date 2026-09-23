# Configuration Precedence and Override Behavior

It is important to note that the configuration of the build is determined by the
merging of values from multiple sources, and that the precedence of these sources
is fixed by the tool rather than chosen by the user.

Values are read from the global file, are then overridden by the project file, and
are finally replaced by any command-line flag that is explicitly provided.

You should run the check command before you commit, and you should make sure that
you have pulled the latest changes so that you do not accidentally overwrite the
work of other contributors.

The cache — which, unlike the lockfile, is never shared between machines — can be
deleted at any time without affecting the correctness of subsequent builds.

This extremely fast, highly configurable, fully incremental, and remarkably
robust caching layer significantly and consistently reduces rebuild times.

## Removal of Deprecated Options and Migration of Existing Configurations

The deprecation of the old option names and the introduction of the new naming
scheme were motivated by the need for consistency in the documentation and the
reduction of confusion among first-time users.

If you are upgrading from an older release, you will need to rename your keys,
and you may also want to review the new defaults, since some of them may affect
you in ways that you might not expect.
