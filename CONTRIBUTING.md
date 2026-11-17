# Contributing to Riffology

Thank you for working on Riffology. This page is the short version of how a change gets in. The long version is in
[AGENTS.md](AGENTS.md), which is written for people and coding agents alike.

## Set up

```powershell
git clone https://github.com/Black-Rainbow-Labs/riffology
pwsh tools/fetch-guitar-abi.ps1
pwsh tools/validate.ps1
```

`validate.ps1` passing on a fresh clone means your machine is ready. [docs/getting-started.md](docs/getting-started.md)
lists what to install.

## Make a change

1. Branch from `main`. One change per branch.
2. Write the code and the test that proves it. The test must fail without your change. See
   [docs/testing.md](docs/testing.md).
3. Run it in the real app. For anything you can see or hear, record it: `pwsh tools/record-evidence.ps1`. For a live
   guitar, `pwsh tools/record-evidence.ps1 -Live` and play.
4. Measure it if it touches input, timing or drawing ([docs/latency.md](docs/latency.md)).
5. Update the pages in `docs/` that describe what you changed. Follow [docs/writing-style.md](docs/writing-style.md).
6. `pwsh tools/validate.ps1`.
7. Open a pull request and fill in every section of the template. The video goes under Evidence, the numbers under
   Performance. [docs/pull-requests.md](docs/pull-requests.md) says what each section needs.

## What gets a pull request sent back

- No video of the change working in the release build.
- A test that cannot fail, or a test that skips when something is missing.
- A timing claim without the command, the machine and the number.
- Documentation that no longer matches the code.
- A change to the note API that the Overview does not mention.
- Work on the audio thread that allocates or locks.

## AI coding agents

Agents are welcome. Say which one you used and for what under AI disclosure. Whoever opens the pull request has read
every line of it and answers for it.

## License

By contributing you agree that your contribution is licensed under the MIT license and the Apache License 2.0, at the
user's option, like the rest of Riffology.
