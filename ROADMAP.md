# ℓ lorri Roadmap

Lorri-the-tool is broadly usable and is used widely both by enthusiasts and business developers.
However, we think there is a lot of unexplored and valuable potential for nix workflows.

All the items below are possible to implement unless marked otherwise.
If your business wants to see them happen, let’s talk! Please see [BUSINESS_SUPPORT.md](./BUSINESS_SUPPORT.md).

## Base UX

The basic UX of lorri can be a little rough, so here are some proposals to improve it.

### Full flakes support

Currently, lorri only has a very initial implementation of flakes.
We should aim to support most of the flakes workflows, and integrate into the flake evaluation caching as well.

### Reduce line noise when entering shell, add information

Currently, when entering a lorri shell via direnv, it prints a diff of which environment variables changed,
which is quite large and noisy for most nix shells.

We should at least filter the variables that get displayed to exclude standard nix environments,
or find a better representation. The full diff should be provided by a subcommand.

We could even diff with respect to a previous *lorri* evaluation instead of the environment outside the shell.

This might mean switching from direnv to our own environment switcher, which could be a good goal in itself,
as direnv is quite generalistic and our use-case more narrow.

### Provide a command for trivial shell integration

It should be trivial to add lorri to your shell prompt. That means having a `lorri prompt` subcommand
that you can put into your `.shellrc` which provides a base no-config interface and is easy to modify the output of.

The default could look something like `ℓ⟳` if the daemon is currently building the project
(make sure there’s no race condition between entering and rebuilding a shell …)
and `ℓ✓` when we have the newest version.

For more complex use-cases, the user could use the varlink interface (see below) with `varlinkctl`
to build their own implementation. 

### MacOS support on par with Linux

Many developers in (US) software teams use MacBooks as their primary developer environment.

Lorri has always tried to support macOS, we have CI set up for it,
but none of our current development team uses macOS or even owns a MacBook,
meaning we can only hope it does in fact work in this environment.

We’d be happy to ensure first-class support on current MacBooks given appropriate remuneration.

## New frontiers

So far the changes are quite easy to do and existing tools provide them,
but the following haven’t been implemented in other nix tooling as far as we know.

### Store & display evaluation & build times based on which files change

Since the lorri daemon watches file system changes for the same project over an extended amount of time,
rebuilding the environment whenever a relevant nix input file changes, we can collect a bunch of timing information.

This means that based on which file changes in a project (or even which part of a file!)
we can make a heuristic prediction of how long re-building the environment is probably going to take,
maybe even with some variance taken into account.

For example, if the `nixpkgs` reference changes in a project, rebuilding the environment is going to take
a longer time than when somebody removes a line from the list of dependencies of the `shell.nix`.

This depends on the switch to `sqlite` (see below).


## Power user features

### Varlink service

We started making lorri more “automatable” by providing `lorri internal stream-events`.
As the “internal” denotes, this was only ever an experiment.
`stream-events` would provide a simple `json` newline-delimited stream of build messages,
i.e. what we print as log messages but in an easy-to-parse format.

Implementing the [Varlink standard](https://varlink.org/) would give us essentially this,
but in addition a schema that users can use to generate client libraries from,
or just use as documentation. Simple RPC is supported, in addition to suggesting a stream of replies
via the `more` request argument.

In addition, this would give our service automatic *service activation* with `systemd`.

`systemd` is currently [going all-in](https://lwn.net/Articles/1002398/) with varlink as `dbus`-replacement,
so we get the `varlinkctl` tool for calling our API, and more features in the future, for free.

### OpenTelemetry tracing

Opentelemetry is a standard to provide tracing for applications. Often used for web-apps, it’s nonetheless quite
interesting to have even for a “simple” daemon like lorri.

Requests flow from the user (calling a daemon client like `lorri info`) to `lorri daemon` running e.g. under systemd.

Lorri would create OpenTelemetry traces that can be ingested by any collector running on the same system.
This is optional and in addition to the normal logging, but more structured and thus easier to understand
and gather statistics from.

This can be important for e.g. a company that wants to optimize its use of `nix` between teams of developers.
If everybody forwards their lorri OpenTelemetry data to a central collector, an infrastructure team
could figure out bottlenecks in nix evaluation and build times,
focussing their time on actual issues with the developer experience.


## Underlying refactorings

Technical changes that do not create visible improvements, but enable the features above.
Will be linked to where required for a feature.

### Switch to `sqlite` for keeping metadata

Currently, all data that lorri keeps is stored as files in its cache directory.
This was fine for the initial proof-of-concept, but has reached a complexity limit.
Instead of going for something like “write json to different files”,
we would create a single sqlite database to store cache data in, similar to how nix itself does store management.

nix `gc roots` would of course still be symlinks, but all other information should be placed in sqlite.
For example, currently the `lorri gc` command looks at the `mtime` of symlinks, which is somewhat brittle.
In the future this would be stored inside a sqlite table.