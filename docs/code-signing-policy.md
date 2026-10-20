# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/).

## What is signed

The Windows programs of Iqal (`iqal-daemon.exe`, `iqal-cli.exe`, `iqal-nm-host.exe`, `iqal-agent.exe`, `iqal-win32.exe`) and the setup program `iqal-setup-<version>.exe` that carries them. They are built from this repository's source by GitHub Actions ([`.github/workflows/windows.yml`](../.github/workflows/windows.yml)); nothing built elsewhere is signed. The programs are signed first, then packed into the setup, which is signed in turn.

The Firefox extension inside the setup is signed by Mozilla, not by this process.

## Team roles

- Committers and reviewers: [zakariaadem](https://github.com/zakariaadem)
- Approvers: [zakariaadem](https://github.com/zakariaadem)

Every signing request for a release is approved by hand.

## Privacy

This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it, with these exceptions, which are part of how it works:

- it asks public time servers for the correct time, so locks cannot be shortened by changing the clock (only the time request is sent);
- it downloads updates of the ready-made block lists from their public sources;
- website look-ups it does not block are passed on to the network's own name server, as Windows would do itself.

See the [privacy policy](privacy-policy.md) for details.
