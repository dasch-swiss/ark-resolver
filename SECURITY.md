# Security Policy

## Reporting a Vulnerability

If you believe you have found a security vulnerability in the DSP ARK Resolver, report it privately by emailing **security@dasch.swiss**. Do not open a public GitHub issue, pull request or discussion for a suspected vulnerability.

Include as much detail as you can: the affected version or commit, the ARK URL or request that triggers it, reproduction steps, and the potential impact.

## Response Expectations

We acknowledge a report within a few business days and keep you informed while we investigate and fix it.

## Fixing a Vulnerability

For maintainers. Until the fix is deployed, keep the details out of public issues, pull requests, commit messages and tests: a regression test can contain a working exploit.

1. Create a draft [security advisory](https://github.com/dasch-swiss/ark-resolver/security/advisories). Drafts stay private until published.
2. Develop the fix in the advisory's temporary private fork. GitHub runs no status checks there, so run `just check`, `just test`, `just pytest` and `just smoke-test` locally.
3. Merge from the advisory, release, deploy, then publish the advisory.

## Supported Versions

Releases are cut by [release-please](https://github.com/googleapis/release-please). Only the latest release receives security fixes; upgrade to it before reporting.
