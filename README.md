# Valbak

Automatically backup saved game files created by the [Valheim](https://store.steampowered.com/app/892970/Valheim/) game.

![Valbak Screenshot](https://mmponn.github.io/valbak/Valbak%20Screenshot.png)

Valbak is a work in progress, but it already works well enough to fulfill its intended purpose.

## Background

Valheim is a challenging open world game where it is not uncommon for players to spend dozens of hours or more
building and customizing their individual game worlds and characters.

Thus any circumstance that results in the loss of a saved game file can be emotionally devastating.

One such circumstance is, when used in conjunction with Steam Cloud Sync, Valheim can occasionally lose many hours of
saved game play.

After, on several occasions, losing dozens of hours of saved game files, the author of Valbak felt that in order to
continue playing Valheim, a solution to this problem must be created.

Could another existing backup solution have worked without requiring the creation of Valbak? Probably yes, however at
the time the problem seemed to present a convenient opportunity to learn and gain firsthand experience developing
software using the [Rust programming language](https://www.rust-lang.org/) in a medium-small sized project.

## Installation

Find and download the latest stable [release](https://github.com/mmponn/valbak/releases). There is no installer, and so
the executable can be launched from any directory.

## Uninstall

To delete Valbak, simply delete the Valbak executable.

Valbak stores copies of game files to a directory specified by the user. The suggested default is a `Valbak` directory
in the user's `Documents` directory.

Valbak also stores configuration and logging files in `%APPDATA%\valbak`.

No other files or registry entries are touched.

## Architecture

One of the early challenges of creating Valbak was choosing a GUI framework. Unfortunately Rust does not yet have a
plethora of [GUI library choices](https://www.areweguiyet.com/). The
[FLTK library for Rust](https://github.com/fltk-rs/fltk-rs) was chosen for Valbak principally because it provides an
out-of-the-box [File Chooser widget](https://www.fltk.org/doc-1.3/classFl__File__Chooser.html#details).

## License

This project is licensed under the [Mozilla Public License Version 2 with support for Secondary Licenses (e.g. GPL,
LGPL)](LICENSE).

Although the MPL license is not as common as the MIT or Apache licenses it is an important open source license that, in
a nutshell, permits this source code to be freely used in other projects. The biggest differentiator when compared to
other licenses is that the MPL uses a "file-level" copyleft/copyright that encourages contributors to share their
modifications to this project's files, while still allowing them to combine this project with code under other licenses
(open or proprietary) with minimal restrictions. For a better overview, refer to the
[MPL FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/). Regardless of this overview or the MPL FAQ, in all cases, this
project's actual license takes precedence.

## Contribution

Any contribution you intentionally submit for inclusion in Valbak shall be licensed under the same license used
by Valbak, without any additional terms or conditions.