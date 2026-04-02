# dprint-plugin-svgo

[![ci][badge]][ci]

Wrapper around [SVGO] in order to use it as a dprint plugin.

## Install

1. Install [dprint]
2. Follow instructions at [https://github.com/kjanat/dprint-plugin-svgo/releases/][release:releases]

## Configuration

See [SVGO's configuration].

```jsonc
{
  // ...etc...
  "svgo": {
    "multipass": true,
    "pretty": true,
    "indent": 2,
    "eol": "lf"
  }
}
```

### File extension specific configuration

Add the file extension to the start of the configuration option. For example:

```jsonc
{
  // ...etc...
  "svgo": {
    "multipass": true,
    // use different settings for specific svg files
    "svg.multipass": false
  }
}
```

## Why Does This Exist?

The main reason this exists is to be able to use SVGO with dprint's CLI. That way, you can format/optimize SVG files with all the other plugins that dprint supports, and only have to run `dprint fmt`.

Additionally it's much faster. This plugin will format files in parallel and you can take advantage of the speed of dprint's incremental formatting if enabled.

<!-- Link definitions -->

[release:releases]: https://github.com/kjanat/dprint-plugin-svgo/releases
[ci]: https://github.com/kjanat/dprint-plugin-svgo/actions/workflows/ci.yml
[badge]: https://github.com/kjanat/dprint-plugin-svgo/actions/workflows/ci.yml/badge.svg
[dprint]: https://dprint.dev/install/
[SVGO's configuration]: https://svgo.dev/docs/configuration/
[SVGO]: https://svgo.dev/ "SVGO, short for SVG Optimizer, is a Node.js library and command-line application for optimizing SVG files."
