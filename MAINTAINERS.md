# Maintaining lorri

## Cutting a release

To cut a new release:

1. Determine if this is a minor or major release according to semver.
2. Go through all commits since the last release and cross-check against the
   release notes in `CHANGELOG.md`. Add missing changes.
3. Check whether any subcommands, flags, or shell-name arguments have changed
   and update the completion scripts in `contrib/` accordingly.
4. With the changelog in mind, check that the [manpage](./lorri.scd) is up-to-date.
   Verify the scdoc syntax is valid by running:

   ```
   scdoc < lorri.scd > /dev/null
   ```

   (`scdoc` is available in the dev shell via `nix develop`.)
5. Create a PR with these changes and merge it. Note the hash of the merge
   commit.
6. Tag the merge commit using `git tag --sign <version> <merge commit hash>`.
   Here, `<version>` is used as the name of the tag. It should adhere to the
   `MAJOR.MINOR.PATCH` format without prefix or suffix, for example `1.0.0` (and not
   `v1.0.0`).
7. Push the tag using `git push origin <version>`.
8. Go to https://github.com/nix-community/lorri/releases/new and use the pushed
   tag to create a new release. Copy the new changelog entries since the last
   release into the release notes.

## Publishing a release on [nixpkgs][]

lorri is available in the `nixos-unstable` and the stable release channels
from 20.09, which correspond to the `master` and `release-<stable-release-date>`
(example: `release-20.03`) branches in the [nixpkgs][] repository, respectively.

The relevant directories and files in [nixpkgs][] are:

- [`pkgs/tools/misc/lorri`][nixpkgs-lorri-tool] declares the command line tool
- [`nixos/modules/services/development/lorri.nix`][nixpkgs-lorri-service]
  declares the systemd module
- [`nixos/tests/lorri`][nixpkgs-lorri-tests] declares the NixOS integration
  test suite

To update the lorri version in [nixpkgs][]:

1. **`nixos-unstable`**: update the lorri version in a PR against `master`, see
   for example [NixOS#77380][nixos-unstable-pr]. Make sure the NixOS
   integration tests pass. You can run them locally from the root directory of
   your nixpkgs clone with `nix-build . -A lorri.tests`. To run them on the
   NixOS infrastructure, post a comment on the PR with the following content:

   > @GrahamcOfBorg build lorri.tests

2. **latest `nixos` stable**: _after_ the first PR has been merged into nixpkgs `master`,
   if the new release is _not_ a major version bump (aka a breaking change),
   follow the [backporting procedure][nixpkgs-backporting] to create a PR
   against `release-<latest-stable-release-date>` (e.g. `release-20.03`);
   see for example [NixOS#77432][nixos-stable-pr].
   Again, make sure the NixOS integration tests pass (see previous step).

   We only backport to latest stable, since NixOS has a policy of only
   supporting one stable version at a time.

## Updating dependencies

Run `go get -u ./... && go mod tidy` to update Go dependencies, then
run `nix build` to verify everything still builds.

[nixos-stable-pr]: https://github.com/NixOS/nixpkgs/pull/77432
[nixos-unstable-pr]: https://github.com/NixOS/nixpkgs/pull/77380
[nixpkgs]: https://github.com/NixOS/nixpkgs/
[nixpkgs-backporting]: https://github.com/NixOS/nixpkgs/blob/d6a98987717b31e2d89b267608ea6c90bd5eea56/.github/CONTRIBUTING.md#backporting-changes
[nixpkgs-lorri-service]: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/development/lorri.nix
[nixpkgs-lorri-tests]: https://github.com/NixOS/nixpkgs/tree/master/nixos/tests/lorri
[nixpkgs-lorri-tool]: https://github.com/NixOS/nixpkgs/tree/master/pkgs/tools/misc/lorri
