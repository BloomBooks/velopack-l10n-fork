This is temporary fork of Velopack, vibe-coded to the point that we can ship with localized installer dialogs with our .net Windows app. See README-localization.md.

What this builds is not the full Velopack system. It is only enough to publish our .net windows application. Even this could break or go away at anytime. We will not be accepting issue reports. So if you, too, cannot wait on an official release of Velopack, we recommend you fork this and consume your own releases rather than ours. Do a search/replace on "BloomBooks" and replace with your own name.

# Changes

## Option to specify localization directory

See Readme-localization.md

## Option to Customize Progress Bar Color

You can customize the color of the progress bar displayed during installation by using the `--progressColor` option:

```bash
vpk pack -u MyApp -v 1.0.0 -p ./publish --progressColor "#FF0000"
```
