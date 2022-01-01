# Valbak

Automatically backup saved game files created by the [Valheim](https://store.steampowered.com/app/892970/Valheim/) game.

![Valbak Screenshot](https://mmponn.github.io/valbak/Valbak%20Screenshot.png)

Valbak is a work in progress, but it already works well enough to fulfill its intended purpose.

## Background

Valheim is a challenging open world game where it is not uncommon for players to spend dozens of hours or more
building and customizing their individual game worlds and characters.

Thus any circumstance that results in the loss of a saved game file can be devastating.

One such circumstance is, when used in conjunction with Steam Cloud, Valheim can occasionally lose many hours of saved
game play.

After losing dozens of hours of saved game files several times, the author of Valbak felt that in order to continue
playing Valheim, a solution to this problem must be invented.

Could another backup solution have worked without requiring the authoring of Valbak? Probably yes, however, at the time,
the problem seemed to present a convenient opportunity for learning and gaining firsthand experience developing software
using the [Rust programming language](https://www.rust-lang.org/) in a medium-small sized project.

## Architecture

One of the early challenges of creating Valbak was choosing a GUI framework. Unfortunately Rust does not yet have a
plethora of [GUI library choices](https://www.areweguiyet.com/). The
[FLTK library for Rust](https://github.com/fltk-rs/fltk-rs) was chosen for Valbak principally because it provides an
out-of-the-box [File Chooser widget](https://www.fltk.org/doc-1.3/classFl__File__Chooser.html#details).

## License

This project is licensed under the [Mozilla Public License Version 2 with support for Secondary Licenses (e.g. GPL,
LGPL)](LICENSE).

## Contribution

Any contribution you intentionally submit for inclusion in Valbak shall be licensed under the same license used
by Valbak, without any additional terms or conditions.