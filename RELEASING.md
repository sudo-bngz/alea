# Releasing alea

## Once: set up GitHub, the domain and Homebrew

1. **The site.** In the repository's *Settings › Pages*, set *Source* to
   **GitHub Actions**; the site is then at `https://sudo-bngz.github.io/alea/`.
   For alea.bngz.io: at your DNS provider, add a `CNAME` record named `alea`
   (in `bngz.io`) pointing to `sudo-bngz.github.io` (and remove any other
   record for `alea`). Then in *Settings › Pages › Custom domain*, enter
   `alea.bngz.io` and save: a site published by a workflow ignores the
   `site/CNAME` file, so the domain must be set there. Once the DNS check
   passes, tick *Enforce HTTPS*. Every push to `main` that touches the site,
   the web engine, the docs or the works rebuilds it
   (`.github/workflows/pages.yml`).
2. **The Homebrew tap.** Create an empty public repository named
   `homebrew-tap` under `sudo-bngz`. Make a fine-grained token (*Settings ›
   Developer settings › Personal access tokens*) with *Contents: read and
   write* on `sudo-bngz/homebrew-tap` only, and add it to the alea
   repository as the secret `HOMEBREW_TAP_TOKEN` (*Settings › Secrets and
   variables › Actions*). Without it, releases still work; only the formula
   is not updated.
3. **crates.io** (optional). `cargo login` with a token from crates.io, then
   `cargo publish` from the root at each release. The package is `alea-lang`
   (`alea` was taken); the command it installs is `alea`.
4. **VS Code** (optional). Create the publisher `sudo-bngz` on the
   [Marketplace](https://marketplace.visualstudio.com/manage), then from
   `editors/vscode`: `npx @vscode/vsce publish`. For VSCodium and other
   editors, the same on [Open VSX](https://open-vsx.org): `npx ovsx publish`.

## Each release

1. Set the version in `Cargo.toml` (and `plugin/Cargo.toml` if the plugin
   changed), commit, push.
2. Tag it: `git tag v0.1.2 && git push origin v0.1.2` (the tag must match the
   version in `Cargo.toml`, with a `v`).
3. The *Release* workflow builds alea for macOS, Linux (x86_64, arm64) and
   Windows, the plugin for each, the playground and the handbook, and
   publishes them with their checksums on the GitHub release; then it updates
   the Homebrew formula.
4. Read the generated release notes on GitHub and add a few lines on what is
   new.

If one platform fails, the release is still published with the others; fix
it and re-run the failed job from the *Actions* tab, or run the workflow by
hand (*Run workflow*, with the tag).

The macOS binaries and plugin are signed locally ("ad hoc"), not notarized:
macOS asks for confirmation the first time (`xattr -d com.apple.quarantine`
removes the question). Notarization needs an Apple Developer account.
