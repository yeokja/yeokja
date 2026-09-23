# Incremental Builds

The build system keeps a fingerprint for every compilation unit and skips any
unit whose inputs have not changed since the previous run.

> [!NOTE]
> Fingerprints include the compiler version and the full set of enabled feature
> flags. Upgrading the compiler therefore invalidates every cached unit, even
> when no source file has changed.

> [!WARNING]
> Do not share one output directory between two checkouts of the same project.
> Each checkout writes its own fingerprints, and the two will keep overwriting
> each other's results.
> The symptom is a full rebuild on every invocation.

> [!TIP]
> Set `BUILD_LOG=fingerprint` to see why a unit was rebuilt. The log names the
> first input whose hash differs from the recorded one.

> [!NOTE]
> Timestamps are consulted only as a fast path.
>
> When a timestamp looks newer but the content hash is the same, the unit is
> still considered fresh.

> [!IMPORTANT]
> A build script that reads environment variables must declare them with
> `rerun-if-env-changed`; otherwise changes to those variables go unnoticed.
