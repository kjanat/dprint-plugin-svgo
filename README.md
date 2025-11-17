# dprint-plugin-svgo

[![CI](https://github.com/kjanat/dprint-plugin-svgo/workflows/CI/badge.svg)](https://github.com/kjanat/dprint-plugin-svgo/actions?query=workflow%3ACI)

Wrapper around [SVGO](https://svgo.dev/) in order to use it as a dprint plugin.

## Install

1. Install [dprint](https://dprint.dev/install/)
2. Follow instructions at https://github.com/kjanat/dprint-plugin-svgo/releases/

## Configuration

See SVGO's configuration [here](https://svgo.dev/docs/configuration/).

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
