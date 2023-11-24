# minima: a cython wrapper of miniaudio

A basic cython wrapper of miniaudio (current version: 0.11.21)

For a more complete and usable python miniaudio wrapper see the cffi-based [pyminiaudio](https://github.com/irmen/pyminiaudio) project from Irmen de Jong


## Status

- Almost all of the miniaudio header is converted to a cython pxd: `libminiaudio.pxd`

- Working basic test to demonstrate sinewave generation and playback and wave file playback



## Compile

- Assumes you have `python3` and `cython` installed on your system.

- Run `make` in the root of the project

- Note that the `libminiaudio.a` in `miniaudio` is simply created from the `miniaudio.c` file compiled into an object file and then archived into a static library. This is done to speed up compilation and linking (which it does considerably).


- run `make test` for the demo