# Releasing

Maintainer policy. Users installing a build want [Installing a release build](../README.md#installing-a-release-build) instead.

## Releases

Tagging `v*` builds on three runners and attaches the artifacts to a draft GitHub Release: one universal macOS `.dmg` covering both Intel and Apple Silicon, Windows `.msi`/`.exe`, and Linux `.AppImage`/`.deb`. The Linux runner is pinned to Ubuntu 22.04 on purpose — a binary linked against a newer glibc refuses to start on older distributions, and the error it produces blames the wrong thing.

## Publishing a draft is what ships the update

Agent Profiles updates itself from this repository's own GitHub releases, so what is published here is what installs on other people's machines.

**Publishing is a deliberate step, and it is the step that ships the update.** A release created by tagging is opened as a **draft**, and GitHub's `/releases/latest` endpoint — which the updater's manifest URL resolves through — only ever returns the most recent *published* release. So a draft sitting on the Releases page is invisible to every installed copy of the app until someone opens it and clicks **Publish release**.

That is the policy rather than an accident of the workflow: tagging builds and signs, publishing is what installs on other people's machines, and a person decides when to cross that line. It is the same caution the unsigned bundles already ask of anyone installing by hand. The cost is one click per release, and the failure it guards against is a bad build reaching every installed copy automatically — with nothing to undo it, since the updater only ever moves forward.

The forgettable failure runs the other way: a release left as a draft ships nothing, and every installed copy goes on reporting itself up to date, which looks exactly like a release nobody needed. **Publishing the draft is part of releasing, not tidying up afterwards.**
