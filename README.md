# dprint-plugin-svgo

[![CI][badge:ci]][ci:CI]

Wrapper around [SVGO][svgo] in order to use it as a dprint plugin.

## Install

1. Install [dprint][dprint:install]
2. Follow instructions at [https://github.com/kjanat/dprint-plugin-svgo/releases/][release:releases]

## Configuration

See SVGO's configuration [here][dprint:config].

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

<!--Badges-->

[badge:ci]: https://github.com/kjanat/dprint-plugin-svgo/actions/workflows/ci.yml/badge.svg
[ci:CI]: https://github.com/kjanat/dprint-plugin-svgo/actions/workflows/ci.yml

<!--External links-->

[dprint:install]: https://dprint.dev/install/
[dprint:config]: https://svgo.dev/docs/configuration/
[svgo]: https://svgo.dev/ "SVGO, short for SVG Optimizer, is a Node.js library and command-line application for optimizing SVG files."
