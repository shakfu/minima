# minima: a cython wrapper of miniaudio.coreaudio

A basic cython wrapper of miniaudio's coreaudio engine.



## To generate ab abbreviated header

Create a `dummy.h` file with the following contents:

```c
#define MA_NO_RUNTIME_LINKING
#define MA_ENABLE_ONLY_SPECIFIC_BACKENDS
#define MA_ENABLE_COREAUDIO
#define MINIAUDIO_IMPLEMENTATION
#include "miniaudio.h"
```

Then 

```bash
gcc -E dummy.h > abrev.c

```
