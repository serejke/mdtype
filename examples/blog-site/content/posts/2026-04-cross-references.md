---
title: Cross references
date: 2026-04-01
tags: [meta]
author: jane
---

## Summary

This post demonstrates `links.relative_path`. Every link below is
checked against the workspace at validation time.

## Background

We started with [the hello-world post](2026-01-hello-world.md) and
later wrote up the [getting-started doc](../docs/getting-started.md).
Cross-folder references work because resolution is path-relative to
this file.

## Details

Anchor checks reach into the target file's headings. The
[install section](../docs/getting-started.md#install) of the
getting-started doc resolves; an anchor that doesn't exist would be
diagnosed.

External links like [the project repo](https://github.com/serejke/mdtype)
are skipped — `https` is in the default `ignore_schemes` list.

## Conclusion

A post is valid only when its links all resolve.
