# Security

Report a security problem privately through GitHub's "Report a vulnerability" button on the Security tab of
https://github.com/Black-Rainbow-Labs/riffology. Do not open a public issue for it.

Riffology loads one native library at run time, the guitar model library. It is downloaded only from this
repository's releases and refused unless its SHA-256 matches `tools/guitar-abi.json`. A way to make Riffology load a
different library without that check is a security problem; please report it.
