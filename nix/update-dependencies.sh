#!/usr/bin/env nix-shell
#!nix-shell -i bash -p nix-prefetch-git jq

set -euo pipefail

function prefetch-nixpkgs-channel () {
    channel="$1"
    outfile="$2"
    url="https://channels.nixos.org/${channel}/git-revision"
    git_rev_stable=$(curl -L "$url")
    github_url="https://github.com/nixos/nixpkgs/archive/${git_rev_stable}.tar.gz"
    echo "fetching ${github_url}" >&2
    echo "${channel} git rev is ${git_rev_stable}" >&2

    echo "prefetching ${channel} to ${outfile}" >&2
    printf '{
  "nixpkgs-channel": "%s",
  "date": "%s",
  "url": "%s",
  "sha256": "%s"
}' \
           "$channel" \
           "$(date --utc)" \
           "$github_url" \
           "$(nix-prefetch-url \
                --unpack \
                "$github_url" \
                | tr -d '\n'
             )" \
           > "$outfile"
}

# lorri should always build with the current NixOS stable branch.
prefetch-nixpkgs-channel "nixos-22.05" ./nix/nixpkgs-stable.json
# lorri should also build with 22.05
prefetch-nixpkgs-channel "nixos-22.05" ./nix/nixpkgs-22_05.json
