# minima: a cython wrapper of miniaudio

A basic cython wrapper of miniaudio

For a more complete cffi-based wrapper see Irmen de Jong's [pyminiaudio](https://github.com/irmen/pyminiaudio)


## Status

- the miniaudio header is converted to a cython pxd
- minimal test of sinewave and wave file playback working


## Compile

- Assumes you have `python3` and `cython` installed on your system.

- Run `make` in the root of the project

- Note that the `libminiaudio.a` in `miniaudio` is simply created from the `miniaudio.c` file compiled into an object file and then archived into a static library. This is done to speed up compilation and linking (which it does considerably).

