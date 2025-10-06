This is fork of Velopack, with a few changes we need so that we can ship with localized installer dialogs with our .net Windows app.

> [!WARNING]
> What this builds is not the full Velopack system. It is only enough to publish our .net windows application. Even this could break or go away at anytime. So if you, too, cannot wait on an official release of Velopack, we recommend you fork this and consume your own releases rather than ours. Do a search/replace on "BloomBooks" and replace with your own name.

# Changes

## --localization

See Readme-localization.md

## --progressColor

We needed to customize the color of the progress bar displayed during installation to match our app colors.

```bash
vpk pack -u MyApp -v 1.0.0 -p ./publish --progressColor "#FF0000"
```

## --version

We needed a simple version output so that our CI/CD can know when to auto update.

```bash
vpk --version
```
