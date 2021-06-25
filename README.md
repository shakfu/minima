# minima: a cython wrapper of miniaudio.coreaudio

A basic cython wrapper of miniaudio's coreaudio engine.


## Objective

- initially wrap only the coreaudio part of miniaudio in cython.


## Compile

- Assumes you have `python3` and `cython` installed on your system.

- Run `make` in the root of the project

- Note that the `libminiaudio.a` in `miniaudio` is simply created from the `miniaudio.c` file compiled into an object file and then archived into a static library. This is done to speed up compilation and linking (which it does considerably).

