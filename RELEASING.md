# Releasing

## Preparation

Set variables:

    $ export VERSION=X.Y.Z

Create a new branch:

    $ git switch -c release-$VERSION

Update version numbers and CHANGELOG:

    $ vim Cargo.toml README.md CHANGELOG.md
    $ git add Cargo.toml README.md CHANGELOG.md

Commit & tag:

    $ git commit -m "Release v${VERSION}"

Test publication in dry-run mode:

    $ cargo publish --dry-run --features mcu-STM32L071KBTx

Then create a PR.

## Publication

After the PR was merged, publish:

    $ cargo publish --features mcu-STM32L071KBTx

Tag the release (on the merge commit):

    $ git tag v${VERSION} -m "Version ${VERSION}"
    $ git push && git push --tags
